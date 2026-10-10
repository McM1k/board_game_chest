//! Catan on the server. The rules (turns, dice, the robber) are all in
//! `catan_core`; this plugs them into the rooms.

use super::{Game, Running, WRONG_GAME};
use catan_core::Rules;
use protocol::{GameMove, GameOptions, GameView};
use rand::RngCore;

pub struct Catan;

impl Game for Catan {
    /// The seats are shuffled for a random turn order.
    fn shuffles_seats(&self) -> bool {
        true
    }

    fn start(&self, names: Vec<String>, _options: Option<&GameOptions>, mut rng: &mut dyn RngCore) -> Box<dyn Running> {
        Box::new(catan_core::Game::new(Rules::default(), names, &mut rng))
    }
}

impl Running for catan_core::Game {
    fn play(&mut self, seat: usize, mv: GameMove) -> Result<(), String> {
        let GameMove::Catan(action) = mv else {
            return Err(WRONG_GAME.into());
        };
        // The dice are rolled here, on the server.
        self.apply(seat, action, &mut rand::thread_rng())
    }

    fn view(&self, seat: usize) -> GameView {
        GameView::Catan(Box::new(self.view_for(Some(seat))))
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::*;
    use protocol::{ClientMsg, GameKind, GameMove, GameView};

    #[test]
    fn a_catan_game_deals_every_seat_its_own_view_and_checks_whose_turn_it_is() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Catan, 3);
        assert_eq!(clients[0].send(&state, ClientMsg::Start), None);

        // Everyone gets a view of their own seat; seats are a permutation.
        let mut views = Vec::new();
        for c in &mut clients {
            let Some((GameView::Catan(view), _, _)) = c.latest_game() else {
                panic!("a Catan state for every player");
            };
            views.push(view);
        }
        let mut seats: Vec<usize> = views.iter().map(|v| v.seat.unwrap()).collect();
        seats.sort_unstable();
        assert_eq!(seats, [0, 1, 2]);

        // The player whose turn it isn't is turned away; the other one can place.
        let current = views[0].current;
        let (mine, other): (Vec<usize>, Vec<usize>) = (0..3).partition(|&i| views[i].seat == Some(current));
        let (cur, oth) = (mine[0], other[0]);
        let vertex = views[cur].legal.settlements[0];
        let act = ClientMsg::Move(GameMove::Catan(catan_core::Action::BuildSettlement { vertex }));
        assert!(clients[oth].send(&state, act.clone()).is_some());
        assert_eq!(clients[cur].send(&state, act), None);
        for c in &mut clients {
            assert!(matches!(c.latest_game(), Some((GameView::Catan(_), _, _))), "everyone sees the move");
        }
        // The room holds a Catan game.
        with_game(&state, &code, |g: &mut catan_core::Game| assert_eq!(g.players.len(), 3));
    }
}
