//! Small World on the server. The rules check whose move it is: the player
//! whose turn it is, or one putting back tokens they kept after losing a
//! region. Every seat is then sent its own view, with the others' coins
//! hidden.

use super::{Game, Running, WRONG_GAME};
use protocol::{GameMove, GameOptions, GameView};
use rand::{Rng, RngCore};
use smallworld_core::State;

pub struct SmallWorld;

impl Game for SmallWorld {
    /// Seats are shuffled for a random first player.
    fn shuffles_seats(&self) -> bool {
        true
    }

    fn start(&self, names: Vec<String>, _options: Option<&GameOptions>, rng: &mut dyn RngCore) -> Box<dyn Running> {
        Box::new(State::new(names.len(), rng.gen()))
    }
}

impl Running for State {
    fn play(&mut self, seat: usize, mv: GameMove) -> Result<(), String> {
        let GameMove::SmallWorld(action) = mv else {
            return Err(WRONG_GAME.into());
        };
        self.apply(seat, action).map_err(|e| e.message().into())
    }

    fn view(&self, seat: usize) -> GameView {
        GameView::SmallWorld(Box::new(self.view_for(seat)))
    }
}

#[cfg(test)]
mod tests {
    use crate::rooms::disconnect;
    use crate::testing::*;
    use protocol::{ClientMsg, GameKind, GameMove, GameView};
    use smallworld_core::{Action, Phase, State, View, START_COINS};

    fn play(action: Action) -> ClientMsg {
        ClientMsg::Move(GameMove::SmallWorld(action))
    }

    /// The latest Small World view each client has been sent.
    fn latest_views(clients: &mut [Client], views: &mut [Option<Box<View>>]) {
        for (c, v) in clients.iter_mut().zip(views.iter_mut()) {
            if let Some((GameView::SmallWorld(view), _, _)) = c.latest_game() {
                *v = Some(view);
            }
        }
    }

    /// What a player who only sees their own view would do next: conquer
    /// the first region on offer, put tokens back, end the turn.
    fn next_move(v: &View) -> Action {
        let m = &v.moves;
        if let Some(&offer) = m.pick.first() {
            Action::Pick { offer }
        } else if let Some(&(region, _)) = m.conquer.first() {
            Action::Conquer { region }
        } else if m.end_conquests {
            Action::EndConquests
        } else if !m.can_end_turn && m.end_turn.as_deref() == Some("Place both your Heroes first.") {
            let region = *m.hero.iter().find(|&&r| !v.regions[r].hero).unwrap();
            Action::Hero { region }
        } else if !m.can_end_turn && !m.put.is_empty() && m.end_turn.as_deref() != Some("Keep 4 Amazons in hand: they rest until your next turn.") {
            Action::Put { region: m.put[0] }
        } else if !m.can_end_turn && !m.take.is_empty() {
            Action::Take { region: m.take[0] }
        } else if !m.can_end_turn && !m.put.is_empty() {
            Action::Put { region: m.put[0] }
        } else {
            Action::EndTurn
        }
    }

    #[test]
    fn a_small_world_game_starts_with_the_first_seat_picking_a_race() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::SmallWorld, 4);
        assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
        let mut views = vec![None; 4];
        latest_views(&mut clients, &mut views);
        let mut seats: Vec<usize> = Vec::new();
        for v in views.iter().flatten() {
            assert_eq!((v.board, v.regions.len(), v.turns), (4, 40, 9));
            assert_eq!((v.turn, v.current, v.phase), (1, 0, Phase::Pick));
            // Your own coins only.
            for (i, p) in v.players.iter().enumerate() {
                assert_eq!(p.coins, (i == v.you).then_some(START_COINS));
            }
            // Only the first seat has something to do.
            assert_eq!(v.moves.pick.is_empty(), v.you != 0);
            seats.push(v.you);
        }
        seats.sort();
        assert_eq!(seats, [0, 1, 2, 3]);
        with_game(&state, &code, |g: &mut State| assert_eq!(g.seats(), 4));
    }

    #[test]
    fn small_world_moves_are_checked_against_the_seat_whose_turn_it_is() {
        let state = new_state();
        let (_code, mut clients) = lobby(&state, GameKind::SmallWorld, 2);
        clients[0].send(&state, ClientMsg::Start);
        let mut views = vec![None; 2];
        latest_views(&mut clients, &mut views);
        let first = views.iter().position(|v| v.as_ref().unwrap().you == 0).unwrap();
        let second = 1 - first;
        let r = clients[second].send(&state, play(Action::Pick { offer: 0 }));
        assert_eq!(r.as_deref(), Some("It isn't your turn."));
        let r = clients[first].send(&state, play(Action::EndTurn));
        assert_eq!(r.as_deref(), Some("You can't do that right now."));
        assert_eq!(clients[first].send(&state, play(Action::Pick { offer: 0 })), None);
        // Everyone sees the pick.
        latest_views(&mut clients, &mut views);
        for v in views.iter().flatten() {
            assert!(v.players[0].active.is_some());
            assert_eq!(v.phase, Phase::Conquest { ghouls: false });
        }
    }

    #[test]
    fn a_dropped_small_world_player_comes_back_to_their_seat() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::SmallWorld, 3);
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
        let Some((GameView::SmallWorld(view), _, connected)) = back.latest_game() else {
            panic!("expected the game state on reconnecting");
        };
        assert_eq!(view.you, before.you);
        assert_eq!(connected, [true; 3]);
    }

    #[test]
    fn whole_small_world_games_can_be_played_to_the_end_through_the_server() {
        for n in 2..=5 {
            let state = new_state();
            let (code, mut clients) = lobby(&state, GameKind::SmallWorld, n);
            assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
            let mut views: Vec<Option<Box<View>>> = vec![None; n];
            latest_views(&mut clients, &mut views);
            let mut moves = 0;
            while views[0].as_ref().unwrap().phase != Phase::Over {
                let actor = views[0].as_ref().unwrap().actor.expect("somebody acts");
                let i = views.iter().position(|v| v.as_ref().unwrap().you == actor).unwrap();
                let action = next_move(views[i].as_ref().unwrap());
                let r = clients[i].send(&state, play(action));
                assert_eq!(r, None, "{n} players: seat {actor} {action:?}");
                latest_views(&mut clients, &mut views);
                moves += 1;
                assert!(moves < 20_000, "the game got stuck");
            }
            for v in views.iter().flatten() {
                // The coins are out on the table at the end.
                assert!(v.players.iter().all(|p| p.coins.is_some()));
                assert!(!v.winners.is_empty());
            }
            let r = clients[0].send(&state, play(Action::EndTurn));
            assert_eq!(r.as_deref(), Some("The game is over."));
            with_game(&state, &code, |g: &mut State| assert_eq!(g.phase, Phase::Over));
        }
    }
}
