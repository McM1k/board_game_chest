//! It's a Wonderful World on the server: its expansion picked in the lobby,
//! and moves checked against the rules and the seat that sent them, never
//! against a turn (everybody moves at the same time).

use super::{Game, Running, WRONG_GAME};
use protocol::{GameMove, GameOptions, GameView};
use rand::{Rng, RngCore};
use wonderful_core::{Options, State};

pub struct Wonderful;

impl Game for Wonderful {
    /// The seat order decides who drafts from whom and which Empire you get,
    /// so it is shuffled.
    fn shuffles_seats(&self) -> bool {
        true
    }

    /// Corruption & Ascension off, until the host ticks it.
    fn default_options(&self) -> Option<GameOptions> {
        Some(GameOptions::Wonderful(Options::default()))
    }

    fn start(&self, names: Vec<String>, options: Option<&GameOptions>, rng: &mut dyn RngCore) -> Box<dyn Running> {
        let options = match options {
            Some(GameOptions::Wonderful(options)) => *options,
            _ => Options::default(),
        };
        Box::new(State::with_options(names.len(), rng.gen(), options))
    }
}

impl Running for State {
    fn play(&mut self, seat: usize, mv: GameMove) -> Result<(), String> {
        let GameMove::Wonderful(action) = mv else {
            return Err(WRONG_GAME.into());
        };
        self.apply(seat, action).map_err(|e| e.message().into())
    }

    fn view(&self, seat: usize) -> GameView {
        GameView::Wonderful(Box::new(self.view_for(seat)))
    }
}

#[cfg(test)]
mod tests {
    use crate::rooms::{disconnect, Shared};
    use crate::testing::*;
    use protocol::{ClientMsg, GameKind, GameMove, GameOptions, GameView, ServerMsg};
    use wonderful_core::{Action, Options, Phase, Piece, Res, State, Target, Token, View};

    fn play(action: Action) -> ClientMsg {
        ClientMsg::Move(GameMove::Wonderful(action))
    }

    /// The host's message turning Corruption & Ascension on or off.
    fn expansion(on: bool) -> ClientMsg {
        ClientMsg::SetOptions(GameOptions::Wonderful(Options { ascension: on }))
    }

    /// Every seat plays `next_move` until the game is over; the final views.
    fn play_to_the_end(state: &Shared, clients: &mut [Client]) -> Vec<View> {
        let n = clients.len();
        let mut views: Vec<Option<Box<View>>> = vec![None; n];
        latest_views(clients, &mut views);
        for _ in 0..5000 {
            if views.iter().all(|v| v.as_ref().is_some_and(|v| v.phase == Phase::Over)) {
                break;
            }
            let mut moved = false;
            for i in 0..n {
                let view = views[i].clone().expect("every seat has a view");
                if let Some(action) = next_move(&view) {
                    let r = clients[i].send(state, play(action.clone()));
                    assert_eq!(r, None, "seat {i} {action:?} in {:?}", view.phase);
                    latest_views(clients, &mut views);
                    moved = true;
                }
            }
            assert!(moved, "the game got stuck");
        }
        views.into_iter().map(|v| *v.expect("a view")).collect()
    }

    /// The latest Wonderful World view each client has been sent.
    fn latest_views(clients: &mut [Client], views: &mut [Option<Box<View>>]) {
        for (c, v) in clients.iter_mut().zip(views.iter_mut()) {
            if let Some((GameView::Wonderful(view), _, _)) = c.latest_game() {
                *v = Some(view);
            }
        }
    }

    /// What a player who can only see their own view would do next.
    fn next_move(v: &View) -> Option<Action> {
        let me = &v.players[v.you];
        match v.phase {
            Phase::Draft => (v.picked.is_none() && !v.hand.is_empty()).then(|| Action::Draft { card: v.hand[0] }),
            Phase::Planning => match v.drafted.first() {
                Some(&card) if card.0 % 2 == 0 => Some(Action::Build { card }),
                Some(&card) => Some(Action::Recycle { card, to: Target::Empire }),
                None => (!me.ready).then_some(Action::Ready),
            },
            Phase::Production { step } if (step as usize) < Res::ALL.len() => {
                if me.choose {
                    return Some(Action::Choose { token: Token::General });
                }
                if me.ready {
                    return None;
                }
                let res = Res::ALL[step as usize];
                let target = me
                    .buildings
                    .iter()
                    .find(|b| b.remaining().res[res.index()] > 0)
                    .map_or(Target::Empire, |b| Target::Card(b.card));
                Some(Action::Place { piece: Piece::Cube(res), target })
            }
            Phase::Production { .. } => (!me.ready).then_some(Action::Ready),
            Phase::Over => None,
        }
    }

    #[test]
    fn wonderful_deals_every_seat_its_own_hand_in_a_shuffled_order() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Wonderful, 4);
        assert_eq!(clients[0].send(&state, ClientMsg::Start), None);

        let mut hands: Vec<Vec<wonderful_core::CardId>> = Vec::new();
        let mut seen_names: Vec<String> = Vec::new();
        for (i, c) in clients.iter_mut().enumerate() {
            let Some((GameView::Wonderful(view), names, connected)) = c.latest_game() else {
                panic!("expected a Wonderful World state");
            };
            // The names are listed by seat, and this view is for this client's seat.
            assert_eq!(names[view.you], format!("P{i}"));
            assert_eq!(connected, [true; 4]);
            assert_eq!(view.hand.len(), 7);
            assert_eq!((view.round, view.phase), (1, Phase::Draft));
            seen_names = names;
            hands.push(view.hand);
        }
        seen_names.sort();
        assert_eq!(seen_names, ["P0", "P1", "P2", "P3"]);
        // Nobody holds a card somebody else was dealt.
        let mut all: Vec<_> = hands.concat();
        all.sort_by_key(|c| c.0);
        all.dedup();
        assert_eq!(all.len(), 28);
        with_game(&state, &code, |g: &mut State| assert_eq!(g.seats(), 4));
    }

    #[test]
    fn wonderful_hides_picks_until_everybody_has_picked() {
        let state = new_state();
        let (_code, mut clients) = lobby(&state, GameKind::Wonderful, 3);
        clients[0].send(&state, ClientMsg::Start);
        let mut views = vec![None; 3];
        latest_views(&mut clients, &mut views);
        let first = views[0].clone().unwrap();
        let card = first.hand[2];

        assert_eq!(clients[0].send(&state, play(Action::Draft { card })), None);
        latest_views(&mut clients, &mut views);
        // The picker sees their pick; the others only learn that somebody has picked.
        assert_eq!(views[0].as_ref().unwrap().picked, Some(card));
        for other in [1, 2] {
            let v = views[other].as_ref().unwrap();
            assert_eq!(v.picked, None);
            assert!(v.players[first.you].picked);
            assert!(v.players.iter().filter(|p| p.picked).count() == 1);
            assert!(!v.hand.contains(&card));
        }
        // Nothing has been passed on yet.
        assert_eq!(views[1].as_ref().unwrap().hand.len(), 7);
    }

    #[test]
    fn wonderful_moves_are_checked_against_the_seat_not_the_message() {
        let state = new_state();
        let (_code, mut clients) = lobby(&state, GameKind::Wonderful, 3);
        clients[0].send(&state, ClientMsg::Start);
        let mut views = vec![None; 3];
        latest_views(&mut clients, &mut views);
        let hand1 = views[1].clone().unwrap().hand;
        let hand0 = views[0].clone().unwrap().hand;

        // A card from somebody else's hand can't be taken, whatever the message says.
        let r = clients[0].send(&state, play(Action::Draft { card: hand1[0] }));
        assert_eq!(r.as_deref(), Some("That card isn't in your hand."));
        // Planning moves during the draft.
        let r = clients[0].send(&state, play(Action::Build { card: hand0[0] }));
        assert_eq!(r.as_deref(), Some("You can't do that right now."));
        let r = clients[0].send(&state, play(Action::Recycle { card: hand0[0], to: Target::Empire }));
        assert_eq!(r.as_deref(), Some("You can't do that right now."));
        // One pick per step.
        assert_eq!(clients[0].send(&state, play(Action::Draft { card: hand0[0] })), None);
        let r = clients[0].send(&state, play(Action::Draft { card: hand0[1] }));
        assert_eq!(r.as_deref(), Some("You already picked a card: wait for the others."));
        // Refused moves change nothing for anyone.
        for c in &mut clients {
            c.drain();
        }
        assert!(clients[0].send(&state, play(Action::Draft { card: hand0[1] })).is_some());
        for c in &mut clients {
            assert!(c.drain().is_empty());
        }
    }

    #[test]
    fn a_dropped_wonderful_player_comes_back_to_their_own_hand() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Wonderful, 3);
        clients[0].send(&state, ClientMsg::Start);
        let mut views = vec![None; 3];
        latest_views(&mut clients, &mut views);
        let before = views[2].clone().unwrap();

        let mut dropped = clients.remove(2);
        let token = dropped.token();
        disconnect(&state, &dropped.tx, dropped.ident.take());

        let mut back = Client::new();
        let r = back.send(&state, ClientMsg::Join { room: code, name: "P2".into(), token: Some(token) });
        assert_eq!(r, None);
        let Some((GameView::Wonderful(view), _, connected)) = back.latest_game() else {
            panic!("expected the game state on reconnecting");
        };
        assert_eq!((view.you, view.hand), (before.you, before.hand));
        assert_eq!(connected, [true; 3]);
    }

    #[test]
    fn the_host_turns_the_expansion_on_for_up_to_seven_players() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Wonderful, 5);
        // Off at first: five seats, and the lobby says so.
        assert_eq!(join(&state, &code, "P5").err().as_deref(), Some("That room is full."));
        let off = Some(GameOptions::Wonderful(Options::default()));
        assert_eq!(state.lock().unwrap().map[&code].options, off);
        assert_eq!(clients[1].send(&state, expansion(true)).as_deref(), Some("Only the host can change the options."));
        assert_eq!(clients[0].send(&state, expansion(true)), None);
        let on = Some(GameOptions::Wonderful(Options { ascension: true }));
        for c in &mut clients {
            assert!(matches!(c.drain().last(), Some(ServerMsg::Lobby { options, .. }) if *options == on));
        }
        // Now up to seven.
        clients.push(join(&state, &code, "P5").unwrap());
        clients.push(join(&state, &code, "P6").unwrap());
        assert_eq!(join(&state, &code, "P7").err().as_deref(), Some("That room is full."));
        // With seven seated it can't be turned off any more.
        let r = clients[0].send(&state, expansion(false));
        assert_eq!(r.as_deref(), Some("There are too many players here for that."));

        // The game deals from both decks to all seven: 5 + 2 cards each.
        assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
        for c in &mut clients {
            let Some((GameView::Wonderful(view), names, _)) = c.latest_game() else {
                panic!("expected a Wonderful World state");
            };
            assert!(view.options.ascension);
            assert_eq!(names.len(), 7);
            assert_eq!(view.hand.len(), 7);
            assert_eq!(view.hand.iter().filter(|c| c.def().ascension).count(), 2);
        }
        with_game(&state, &code, |g: &mut State| assert!(g.options.ascension));
    }

    #[test]
    fn a_game_without_the_expansion_never_sees_its_cards() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Wonderful, 3);
        assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
        with_game(&state, &code, |g: &mut State| {
            assert!(!g.options.ascension && g.ascension_deck.is_empty());
            assert!(g.players.iter().flat_map(|p| &p.hand).all(|c| !c.def().ascension));
        });
    }

    #[test]
    fn a_whole_wonderful_game_can_be_played_to_the_end_through_the_server() {
        // Three games without the expansion, one with it at a full table.
        for (n, ascension) in [(2, false), (3, false), (5, false), (7, true)] {
            let state = new_state();
            let (code, mut clients) = lobby(&state, GameKind::Wonderful, 2);
            assert_eq!(clients[0].send(&state, expansion(ascension)), None);
            for i in 2..n {
                clients.push(join(&state, &code, &format!("P{i}")).unwrap());
            }
            assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
            let views = play_to_the_end(&state, &mut clients);

            for v in &views {
                assert_eq!(v.options.ascension, ascension);
                assert_eq!((v.phase, v.round), (Phase::Over, wonderful_core::ROUNDS));
                assert_eq!(v.scores.len(), n);
                assert!(!v.winners.is_empty());
                // No expansion deck, or seven players used all of it.
                assert_eq!(v.ascension_deck, 0);
            }
            // Once it is over, further moves are refused politely.
            let r = clients[0].send(&state, play(Action::Ready));
            assert_eq!(r.as_deref(), Some("The game is over."));
            with_game(&state, &code, |g: &mut State| assert_eq!(g.phase, Phase::Over));
        }
    }
}
