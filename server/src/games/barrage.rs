//! Barrage on the server. The rules check whose move it is: the player
//! whose turn it is, or the one a decision is waiting for (an income, a
//! Contract's reward). Every seat is then sent its view; the game hides
//! nothing but the order of its face-down piles.

use super::{Game, Running, WRONG_GAME};
use barrage_core::State;
use protocol::{GameMove, GameOptions, GameView};
use rand::{Rng, RngCore};

pub struct Barrage;

impl Game for Barrage {
    /// Seats are shuffled: seat 1 picks its Company first, and plays last.
    fn shuffles_seats(&self) -> bool {
        true
    }

    fn start(&self, names: Vec<String>, _options: Option<&GameOptions>, rng: &mut dyn RngCore) -> Box<dyn Running> {
        Box::new(State::new(names.len(), rng.gen()))
    }
}

impl Running for State {
    fn play(&mut self, seat: usize, mv: GameMove) -> Result<(), String> {
        let GameMove::Barrage(action) = mv else {
            return Err(WRONG_GAME.into());
        };
        self.apply(seat, action).map_err(|e| e.message().into())
    }

    fn view(&self, seat: usize) -> GameView {
        GameView::Barrage(Box::new(self.view_for(seat)))
    }
}

#[cfg(test)]
mod tests {
    use crate::rooms::disconnect;
    use crate::testing::*;
    use barrage_core::{Action, Phase, Production, State, View, HEADSTREAMS};
    use protocol::{ClientMsg, GameKind, GameMove, GameView};

    fn play(action: Action) -> ClientMsg {
        ClientMsg::Move(GameMove::Barrage(action))
    }

    /// The latest Barrage view each client has been sent.
    fn latest_views(clients: &mut [Client], views: &mut [Option<Box<View>>]) {
        for (c, v) in clients.iter_mut().zip(views.iter_mut()) {
            if let Some((GameView::Barrage(view), _, _)) = c.latest_game() {
                *v = Some(view);
            }
        }
    }

    /// What a player who only sees their own view would do next: pick the
    /// first pair, settle decisions the simplest way, build when they can,
    /// produce when they can, otherwise send Engineers to the Bank.
    fn next_move(v: &View) -> Action {
        let m = &v.moves;
        if m.choose {
            let setup = v.setup.as_ref().unwrap();
            let pair = setup.pairs.iter().position(|p| p.2.is_none()).unwrap();
            let contract = setup.contracts.iter().position(|c| c.1.is_none()).unwrap();
            return Action::Choose { pair, contract };
        }
        if let Some(n) = m.machinery {
            return Action::Machinery { excavators: n };
        }
        if let Some((n, _)) = m.water {
            let mut drops = [0; HEADSTREAMS];
            drops[0] = n;
            return Action::Water { space: None, drops };
        }
        if let Some(&target) = m.places.first() {
            return Action::Place { target };
        }
        if let Some(&contract) = m.fulfill.first() {
            return Action::Fulfill { contract: Some(contract) };
        }
        if m.discard {
            return Action::Discard { contract: v.players[v.you].contracts[0] };
        }
        if v.todo.is_some() {
            return Action::Skip;
        }
        if let Some(o) = m.builds.first() {
            return Action::Build(o.build);
        }
        if m.pass {
            return Action::Pass;
        }
        let credits = v.players[v.you].credits;
        for &spot in &m.spots {
            if spot.section != barrage_core::Section::Turbine || spot.right {
                continue;
            }
            let bonus = m.bonus + barrage_core::TURBINE_BONUS[spot.row as usize];
            for o in &m.productions {
                let drops = o.most(credits);
                if drops > 0 && barrage_core::production_energy(drops, o.value, m.fiesler, bonus) >= 1 {
                    let production = Production { powerhouse: o.powerhouse, conduit: o.conduit, dam: o.dam, drops };
                    return Action::Produce { space: Some(spot), production };
                }
            }
        }
        Action::Bank { engineers: m.bank.min(3) }
    }

    #[test]
    fn a_barrage_game_starts_with_the_first_seat_picking_a_company() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Barrage, 4);
        assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
        let mut views = vec![None; 4];
        latest_views(&mut clients, &mut views);
        let mut seats = Vec::new();
        for v in views.iter().flatten() {
            assert_eq!((v.phase, v.round, v.actor), (Phase::Setup, 0, Some(0)));
            assert_eq!(v.setup.as_ref().unwrap().pairs.len(), 4);
            assert_eq!(v.moves.choose, v.you == 0);
            assert_eq!(v.nationals.len(), 3);
            seats.push(v.you);
        }
        seats.sort();
        assert_eq!(seats, [0, 1, 2, 3]);
        with_game(&state, &code, |g: &mut State| assert_eq!(g.seats(), 4));
    }

    #[test]
    fn barrage_moves_are_checked_against_the_seat_to_act() {
        let state = new_state();
        let (code, clients) = lobby(&state, GameKind::Barrage, 2);
        let mut clients = clients;
        clients[0].send(&state, ClientMsg::Start);
        let mut clients = in_seat_order(&state, &code, clients);
        let r = clients[1].send(&state, play(Action::Choose { pair: 0, contract: 0 }));
        assert_eq!(r.as_deref(), Some("It isn't your turn."));
        let r = clients[0].send(&state, play(Action::Bank { engineers: 1 }));
        assert_eq!(r.as_deref(), Some("You can't do that right now."));
        assert_eq!(clients[0].send(&state, play(Action::Choose { pair: 1, contract: 0 })), None);
        assert_eq!(clients[1].send(&state, play(Action::Choose { pair: 0, contract: 1 })), None);
        // The first to pick plays last.
        let mut views = vec![None; 2];
        latest_views(&mut clients, &mut views);
        for v in views.iter().flatten() {
            assert_eq!((v.phase, v.round, v.current), (Phase::Actions, 1, Some(1)));
            assert!(v.players.iter().all(|p| p.company.is_some()));
        }
    }

    #[test]
    fn a_dropped_barrage_player_comes_back_to_their_seat() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Barrage, 3);
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
        let Some((GameView::Barrage(view), _, connected)) = back.latest_game() else {
            panic!("expected the game state on reconnecting");
        };
        assert_eq!(view.you, before.you);
        assert_eq!(connected, [true; 3]);
    }

    #[test]
    fn whole_barrage_games_through_the_rooms() {
        for players in 2..=4 {
            let state = new_state();
            let (code, clients) = lobby(&state, GameKind::Barrage, players);
            let mut clients = clients;
            clients[0].send(&state, ClientMsg::Start);
            let mut clients = in_seat_order(&state, &code, clients);
            let mut views = vec![None; players];
            latest_views(&mut clients, &mut views);
            let mut moves = 0;
            loop {
                let v = views[0].as_ref().unwrap();
                if v.phase == Phase::Over {
                    break;
                }
                let actor = v.actor.expect("somebody acts");
                let mv = next_move(views[actor].as_ref().unwrap());
                let r = clients[actor].send(&state, play(mv.clone()));
                assert_eq!(r, None, "{mv:?}");
                latest_views(&mut clients, &mut views);
                moves += 1;
                assert!(moves < 3000, "the game doesn't end");
            }
            for v in views.iter().flatten() {
                assert_eq!((v.phase, v.round), (Phase::Over, 5));
                assert!(!v.winners.is_empty());
            }
            let built: u32 = with_game(&state, &code, |g: &mut State| {
                g.players.iter().map(|p| p.built.iter().map(|&b| b as u32).sum::<u32>()).sum()
            });
            assert!(built > 0);
        }
    }
}
