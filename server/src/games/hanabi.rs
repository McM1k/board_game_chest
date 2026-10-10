//! Hanabi on the server: the variant rules picked in the lobby, seats kept
//! in join order, and the rules' refusals put into words.

use super::{Game, Running, WRONG_GAME};
use hanabi_core::{ActionError, GameRules, GameState, PlayerId};
use protocol::{GameMove, GameOptions, GameView};
use rand::{Rng, RngCore};

pub struct Hanabi;

impl Game for Hanabi {
    /// Seats keep their join order: the host plays first.
    fn shuffles_seats(&self) -> bool {
        false
    }

    fn default_options(&self) -> Option<GameOptions> {
        Some(GameOptions::Hanabi(GameRules::default()))
    }

    fn check_options(&self, options: GameOptions) -> Result<GameOptions, String> {
        let GameOptions::Hanabi(rules) = options;
        // The hanabii mode is a fixed preset that replaces every other
        // option. Resolve it here, so the room stores - and every client is
        // shown - the concrete rules that will actually be played, and a
        // hand-built message can't sneak extra options in next to it.
        Ok(GameOptions::Hanabi(rules.normalized()))
    }

    fn start(&self, names: Vec<String>, options: Option<&GameOptions>, rng: &mut dyn RngCore) -> Box<dyn Running> {
        let rules = match options {
            Some(GameOptions::Hanabi(rules)) => *rules,
            _ => GameRules::default(),
        };
        Box::new(GameState::new(names.len() as u8, rng.gen(), rules))
    }
}

impl Running for GameState {
    fn play(&mut self, seat: usize, mv: GameMove) -> Result<(), String> {
        let GameMove::Hanabi(action) = mv else {
            return Err(WRONG_GAME.into());
        };
        self.apply_action(PlayerId(seat as u8), action).map(|_events| ()).map_err(describe_error)
    }

    fn view(&self, seat: usize) -> GameView {
        GameView::Hanabi(Box::new(self.view_for(PlayerId(seat as u8))))
    }
}

/// What a refused move means, in words for the player.
fn describe_error(e: ActionError) -> String {
    match e {
        ActionError::NotYourTurn => "It's not your turn.",
        ActionError::GameOver => "The game is over.",
        ActionError::NoClueTokens => "No clue tokens left: play or discard a card instead.",
        ActionError::CannotClueSelf => "You can't give a clue to yourself.",
        ActionError::UnknownPlayer => "There's no such player.",
        ActionError::CannotClueMulticolor => "The multicolor suit can't be clued directly.",
        ActionError::CannotClueBlack => "The black suit has no color to clue.",
        ActionError::CannotClueSecondaryColor => "In Hanabii mode only red, yellow and blue can be clued.",
        ActionError::ClueMatchesNothing => "That clue doesn't touch any card in their hand.",
        ActionError::CardNotInHand => "That card isn't in your hand.",
        ActionError::CannotDiscardAtMaxClues => "All 8 clue tokens are available: play a card or give a clue instead.",
    }
    .into()
}

#[cfg(test)]
mod tests {
    use crate::rooms::Shared;
    use crate::testing::*;
    use hanabi_core::{Action, Card, Clue, Color, GameRules, GameState, LastMove, PlayerId, PlayerView};
    use protocol::{ClientMsg, GameKind, GameMove, GameOptions, GameView, ServerMsg};

    fn set_rules(state: &Shared, c: &mut Client, rules: GameRules) -> Option<String> {
        c.send(state, ClientMsg::SetOptions(GameOptions::Hanabi(rules)))
    }

    fn rules_of(state: &Shared, code: &str) -> GameRules {
        match &state.lock().unwrap().map[code].options {
            Some(GameOptions::Hanabi(rules)) => *rules,
            other => panic!("expected Hanabi options, found {other:?}"),
        }
    }

    /// The Hanabi rules shown in the latest lobby message in `msgs`.
    fn lobby_rules(msgs: &[ServerMsg]) -> Option<GameRules> {
        msgs.iter().rev().find_map(|m| match m {
            ServerMsg::Lobby { options: Some(GameOptions::Hanabi(rules)), .. } => Some(*rules),
            _ => None,
        })
    }

    fn play(action: Action) -> ClientMsg {
        ClientMsg::Move(GameMove::Hanabi(action))
    }

    /// The latest Hanabi view among the messages `c` received.
    fn latest_view(c: &mut Client) -> Option<Box<PlayerView>> {
        match c.latest_game() {
            Some((GameView::Hanabi(view), _, _)) => Some(view),
            _ => None,
        }
    }

    #[test]
    fn hanabi_seats_follow_join_order() {
        let state = new_state();
        let (_code, mut clients) = lobby(&state, GameKind::Hanabi, 3);
        clients[0].send(&state, ClientMsg::Start);
        for (i, c) in clients.iter_mut().enumerate() {
            let Some((GameView::Hanabi(view), names, connected)) = c.latest_game() else {
                panic!("expected a Hanabi state");
            };
            assert_eq!(view.you, PlayerId(i as u8));
            assert_eq!(names, ["P0", "P1", "P2"]);
            assert_eq!(connected, [true, true, true]);
            assert_eq!(view.current_turn, PlayerId(0), "the host plays first");
            // Your own cards are hidden, everyone else's are shown.
            for (pid, hand) in &view.hands {
                assert_eq!(hand.iter().all(|c| c.card.is_none()), *pid == view.you);
            }
        }
    }

    #[test]
    fn a_hanabi_lobby_starts_with_ordinary_rules() {
        let state = new_state();
        let (mut host, code) = create(&state, GameKind::Hanabi, "Ann");
        assert_eq!(rules_of(&state, &code), GameRules::default());
        assert_eq!(lobby_rules(&host.drain()), Some(GameRules::default()));
    }

    #[test]
    fn picking_hanabii_locks_the_other_options_and_tells_everyone() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Hanabi, 2);

        // A client that (buggy, or hand-built) sends every other option
        // alongside the mode.
        let greedy = GameRules {
            multicolor: true,
            black: true,
            extra_colors: 2,
            multicolor_short: true,
            black_short: true,
            extra_colors_short: true,
            six_cards: false,
            hanabii: true,
        };
        assert_eq!(set_rules(&state, &mut clients[0], greedy), None);

        let preset = GameRules { hanabii: true, ..Default::default() }.normalized();
        assert_eq!(rules_of(&state, &code), preset);
        assert!(preset.hanabii && preset.six_cards && preset.extra_colors == 1);
        assert!(!preset.multicolor && !preset.black);

        // Everyone - the sender included - is shown the locked-in preset,
        // not what was asked for.
        for c in &mut clients {
            assert_eq!(lobby_rules(&c.drain()), Some(preset));
        }
    }

    #[test]
    fn ordinary_rules_pass_through_untouched() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Hanabi, 2);
        let rules = GameRules { multicolor: true, extra_colors: 2, black_short: true, ..Default::default() };
        assert_eq!(set_rules(&state, &mut clients[0], rules), None);
        assert_eq!(rules_of(&state, &code), rules);
        assert_eq!(lobby_rules(&clients[1].drain()), Some(rules));
    }

    #[test]
    fn unpicking_hanabii_frees_the_options_again() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Hanabi, 2);
        set_rules(&state, &mut clients[0], GameRules { hanabii: true, ..Default::default() });
        // Back to a plain game, then a couple of ordinary options on top.
        let rules = GameRules { black: true, six_cards: true, ..Default::default() };
        set_rules(&state, &mut clients[0], rules);
        assert_eq!(rules_of(&state, &code), rules);
    }

    #[test]
    fn only_the_host_picks_the_rules() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Hanabi, 2);
        let rules = GameRules { multicolor: true, ..Default::default() };
        assert_eq!(set_rules(&state, &mut clients[1], rules).as_deref(), Some("Only the host can change the options."));
        assert_eq!(rules_of(&state, &code), GameRules::default());
        assert!(clients[0].drain().is_empty(), "a refused change tells nobody");
    }

    #[test]
    fn rules_cannot_change_once_the_game_has_started() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Hanabi, 2);
        assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
        assert!(state.lock().unwrap().map[&code].game.is_some());

        let r = set_rules(&state, &mut clients[0], GameRules { hanabii: true, ..Default::default() });
        assert_eq!(r.as_deref(), Some("The game has already started."));
        assert_eq!(rules_of(&state, &code), GameRules::default());
    }

    #[test]
    fn a_hanabii_lobby_starts_a_hanabii_game_and_sends_everyone_a_view_of_it() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Hanabi, 2);
        set_rules(&state, &mut clients[0], GameRules { hanabii: true, ..Default::default() });
        assert_eq!(clients[1].send(&state, ClientMsg::Start).as_deref(), Some("Only the host can start the game."));
        assert_eq!(clients[0].send(&state, ClientMsg::Start), None);

        with_game(&state, &code, |game: &mut GameState| {
            assert!(game.rules.hanabii);
            assert_eq!(game.rules.max_score(), 36);
            assert_eq!(game.fireworks.len(), 6);
            assert!(!game.fireworks.contains_key(&Color::White));
        });

        // The state push is what actually reaches the browser, which reads
        // it back with serde_json - so check it survives that trip, new
        // fields and all.
        for c in &mut clients {
            let view = latest_view(c).expect("every player gets a state update when the game starts");
            let msg = ServerMsg::Game { view: GameView::Hanabi(view.clone()), names: vec![], connected: vec![] };
            let json = serde_json::to_string(&msg).unwrap();
            let Ok(ServerMsg::Game { view: GameView::Hanabi(back), .. }) = serde_json::from_str::<ServerMsg>(&json) else {
                panic!("state update didn't round-trip: {json}");
            };
            assert_eq!(back.rules, view.rules);
            assert!(back.rules.hanabii);
            assert_eq!(back.fireworks, view.fireworks);
        }
    }

    /// Starts a two-player Hanabi game and gives the second player a hand of
    /// `card`s, then forgets everything sent so far.
    fn hanabi_game_with_second_hand(state: &Shared, rules: GameRules, card: Card) -> (String, Vec<Client>) {
        let (code, mut clients) = lobby(state, GameKind::Hanabi, 2);
        if rules != GameRules::default() {
            assert_eq!(set_rules(state, &mut clients[0], rules), None);
        }
        assert_eq!(clients[0].send(state, ClientMsg::Start), None);
        with_game(state, &code, |game: &mut GameState| {
            for hc in game.hands.get_mut(&PlayerId(1)).unwrap().iter_mut() {
                hc.card = card;
            }
        });
        for c in &mut clients {
            c.drain();
        }
        (code, clients)
    }

    #[test]
    fn a_hanabii_color_clue_that_touches_nothing_goes_through_the_server() {
        let state = new_state();
        // Deal the second player a hand with no red in it.
        let (_code, mut clients) = hanabi_game_with_second_hand(
            &state,
            GameRules { hanabii: true, ..Default::default() },
            Card { color: Color::Green, number: 1 },
        );
        let clue = play(Action::Clue { target: PlayerId(1), clue: Clue::Color(Color::Red) });
        assert_eq!(clients[0].send(&state, clue), None);

        for c in &mut clients {
            let view = latest_view(c).expect("everyone gets the new state");
            assert_eq!(view.last_actor, Some(PlayerId(0)));
            match &view.last_moves[&PlayerId(0)] {
                LastMove::Clue { touched, .. } => assert!(touched.is_empty()),
                other => panic!("expected a clue, got {other:?}"),
            }
        }
    }

    #[test]
    fn the_same_clue_in_an_ordinary_game_is_still_turned_away() {
        let state = new_state();
        let (_code, mut clients) = hanabi_game_with_second_hand(
            &state,
            GameRules::default(),
            Card { color: Color::White, number: 1 },
        );
        let clue = play(Action::Clue { target: PlayerId(1), clue: Clue::Color(Color::Red) });
        assert_eq!(
            clients[0].send(&state, clue).as_deref(),
            Some("That clue doesn't touch any card in their hand."),
            "an ordinary game must still refuse a clue that touches nothing"
        );
        assert!(clients[1].drain().is_empty(), "a refused move changes nothing for anyone");
    }

    #[test]
    fn hanabi_moves_are_checked_against_the_seat_not_the_message() {
        let state = new_state();
        let (_code, mut clients) = hanabi_game_with_second_hand(
            &state,
            GameRules::default(),
            Card { color: Color::Blue, number: 3 },
        );
        // The guest tries to act on the host's turn, and to clue themselves.
        let clue = play(Action::Clue { target: PlayerId(0), clue: Clue::Number(1) });
        assert_eq!(clients[1].send(&state, clue.clone()).as_deref(), Some("It's not your turn."));
        assert_eq!(clients[0].send(&state, clue).as_deref(), Some("You can't give a clue to yourself."));
    }

    #[test]
    fn a_whole_hanabi_game_can_be_played_to_the_end_through_the_server() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Hanabi, 2);
        clients[0].send(&state, ClientMsg::Start);
        let in_progress = |g: &GameState| g.status == hanabi_core::GameStatus::InProgress;
        for _ in 0..200 {
            let (playing, current, card) = with_game(&state, &code, |g: &mut GameState| {
                let cur = g.current_player();
                (in_progress(g), cur, g.hands[&cur][0].id)
            });
            if !playing {
                break;
            }
            // Always play the first card: fuses run out quickly, which is
            // exactly the game-over path we want to see the clients receive.
            let r = clients[current.0 as usize].send(&state, play(Action::Play { card_id: card }));
            assert_eq!(r, None);
        }
        assert!(!with_game(&state, &code, |g: &mut GameState| in_progress(g)), "the game ended");
        for c in &mut clients {
            let last = latest_view(c).expect("final state");
            assert_ne!(last.status, hanabi_core::GameStatus::InProgress);
        }
        // Once it is over, further moves are refused politely.
        let r = clients[0].send(&state, play(Action::Play { card_id: hanabi_core::CardId(0) }));
        assert_eq!(r.as_deref(), Some("The game is over."));
    }
}
