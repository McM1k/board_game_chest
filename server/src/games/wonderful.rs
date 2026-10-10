//! It's a Wonderful World on the server. Everybody moves at the same time,
//! so a move is checked against the rules and the seat that sent it, never
//! against a turn.

use super::{Game, Running, WRONG_GAME};
use protocol::{GameMove, GameOptions, GameView};
use rand::{Rng, RngCore};
use wonderful_core::State;

pub struct Wonderful;

impl Game for Wonderful {
    /// The seat order decides who drafts from whom and which Empire you get,
    /// so it is shuffled.
    fn shuffles_seats(&self) -> bool {
        true
    }

    fn start(&self, names: Vec<String>, _options: Option<&GameOptions>, rng: &mut dyn RngCore) -> Box<dyn Running> {
        Box::new(State::new(names.len(), rng.gen()))
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
    use crate::rooms::disconnect;
    use crate::testing::*;
    use protocol::{ClientMsg, GameKind, GameMove, GameOptions, GameView, ServerMsg};
    use wonderful_core::{Action, Phase, Piece, Res, State, Target, Token, View};

    fn play(action: Action) -> ClientMsg {
        ClientMsg::Move(GameMove::Wonderful(action))
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
    fn wonderful_lobbies_have_no_options() {
        let state = new_state();
        let (_code, mut clients) = lobby(&state, GameKind::Wonderful, 2);
        let hanabi_rules = GameOptions::Hanabi(hanabi_core::GameRules::default());
        let r = clients[0].send(&state, ClientMsg::SetOptions(hanabi_rules));
        assert_eq!(r.as_deref(), Some("This game has no options to set."));
        // The lobby message says so too.
        clients[0].send(&state, ClientMsg::Leave);
        let msgs = clients[1].drain();
        assert!(matches!(msgs.last(), Some(ServerMsg::Lobby { game: GameKind::Wonderful, options: None, .. })));
    }

    #[test]
    fn a_whole_wonderful_game_can_be_played_to_the_end_through_the_server() {
        for n in [2, 3, 5] {
            let state = new_state();
            let (code, mut clients) = lobby(&state, GameKind::Wonderful, n);
            assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
            let mut views: Vec<Option<Box<View>>> = vec![None; n];
            latest_views(&mut clients, &mut views);

            for _ in 0..5000 {
                if views.iter().all(|v| v.as_ref().is_some_and(|v| v.phase == Phase::Over)) {
                    break;
                }
                let mut moved = false;
                for i in 0..n {
                    let view = views[i].clone().expect("every seat has a view");
                    if let Some(action) = next_move(&view) {
                        let r = clients[i].send(&state, play(action.clone()));
                        assert_eq!(r, None, "seat {i} {action:?} in {:?}", view.phase);
                        latest_views(&mut clients, &mut views);
                        moved = true;
                    }
                }
                assert!(moved, "the game got stuck");
            }

            for v in &views {
                let v = v.as_ref().unwrap();
                assert_eq!((v.phase, v.round), (Phase::Over, wonderful_core::ROUNDS));
                assert_eq!(v.scores.len(), n);
                assert!(!v.winners.is_empty());
            }
            // Once it is over, further moves are refused politely.
            let r = clients[0].send(&state, play(Action::Ready));
            assert_eq!(r.as_deref(), Some("The game is over."));
            with_game(&state, &code, |g: &mut State| assert_eq!(g.phase, Phase::Over));
        }
    }
}
