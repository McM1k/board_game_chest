//! The ember contour on whatever just changed on the island: spotting what
//! changed since the previous state, and how long it glows.

use crate::app::App;
use catan_core::{Building, GameView};
use leptos::prelude::*;
use std::time::Duration;

/// How long a changed piece keeps its ember contour.
pub const EMBER_MS: f64 = 2400.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FreshKind {
    Road(usize),
    /// A new settlement, or a settlement upgraded to a city.
    Piece(usize),
    /// The robber's new tile.
    Robber(usize),
}

#[derive(Clone, Copy)]
pub struct Fresh {
    pub kind: FreshKind,
    /// `now_ms()` when the change arrived.
    pub at: f64,
}

/// The parts of the previous game state needed to spot what changed.
#[derive(Clone)]
pub struct Snap {
    buildings: Vec<Option<Building>>,
    roads: Vec<Option<usize>>,
    robber: usize,
}

impl Snap {
    pub fn of(v: &GameView) -> Snap {
        Snap {
            buildings: v.buildings.clone(),
            roads: v.roads.clone(),
            robber: v.robber,
        }
    }
}

/// Everything on the board that differs between `prev` and `v`.
pub fn diff(prev: &Snap, v: &GameView) -> Vec<FreshKind> {
    if prev.buildings.len() != v.buildings.len() || prev.roads.len() != v.roads.len() {
        return vec![]; // a different board: nothing to compare
    }
    let mut out = Vec::new();
    for (e, r) in v.roads.iter().enumerate() {
        if r.is_some() && prev.roads[e] != *r {
            out.push(FreshKind::Road(e));
        }
    }
    for (i, b) in v.buildings.iter().enumerate() {
        if b.is_some() && prev.buildings[i] != *b {
            out.push(FreshKind::Piece(i));
        }
    }
    if prev.robber != v.robber {
        out.push(FreshKind::Robber(v.robber));
    }
    out
}

#[cfg(target_arch = "wasm32")]
pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

/// Compares the new state with the previous one and starts an ember
/// contour on everything that changed. The first state after joining or
/// reconnecting is only remembered, not highlighted.
pub fn note_changes(app: App, view: &GameView) {
    let catan = app.games.catan;
    let changes = catan
        .snap
        .with_value(|prev| prev.as_ref().map(|p| diff(p, view)))
        .unwrap_or_default();
    catan.snap.set_value(Some(Snap::of(view)));
    if changes.is_empty() {
        return;
    }
    let now = now_ms();
    catan.fresh.update(|list| {
        list.retain(|f| now - f.at < EMBER_MS);
        for kind in changes {
            // One robber ring at a time; re-flag anything that changed again.
            list.retain(|f| {
                f.kind != kind && !(matches!(f.kind, FreshKind::Robber(_)) && matches!(kind, FreshKind::Robber(_)))
            });
            list.push(Fresh { kind, at: now });
        }
    });
    set_timeout(
        move || {
            let now = now_ms();
            catan.fresh.update(|list| list.retain(|f| now - f.at < EMBER_MS));
        },
        Duration::from_millis(EMBER_MS as u64 + 50),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use catan_core::{Action, Game, Hand, Phase, Rules, SetupExpect};
    use rand::{rngs::StdRng, SeedableRng};

    fn game_after_setup() -> (Game, StdRng) {
        let mut rng = StdRng::seed_from_u64(7);
        let names = (0..3).map(|i| format!("P{i}")).collect();
        let mut g = Game::new(Rules::default(), names, &mut rng);
        while let Phase::Setup { expect, .. } = g.phase.clone() {
            let s = g.current;
            let a = match expect {
                SetupExpect::Settlement => Action::BuildSettlement { vertex: g.legal_settlements(s, true)[0] },
                SetupExpect::Road => Action::BuildRoad { edge: g.legal_roads(s, g.setup_vertex)[0] },
            };
            g.apply(s, a, &mut rng).unwrap();
        }
        (g, rng)
    }

    #[test]
    fn diff_flags_exactly_what_changed() {
        let (mut g, mut rng) = game_after_setup();
        g.do_roll(2, 3, &mut rng).unwrap();
        let snap = |g: &Game| Snap::of(&g.view_for(Some(1)));

        // Nothing changed -> nothing flagged (also for another viewer).
        let before = snap(&g);
        assert!(diff(&before, &g.view_for(Some(2))).is_empty());

        // A new road.
        g.players[0].hand = Hand([1, 1, 0, 0, 0]);
        let e = g.legal_roads(0, None)[0];
        g.apply(0, Action::BuildRoad { edge: e }, &mut rng).unwrap();
        assert_eq!(diff(&before, &g.view_for(Some(1))), vec![FreshKind::Road(e)]);

        // A settlement upgraded to a city counts as a new piece.
        let before = snap(&g);
        let v = g.legal_cities(0)[0];
        g.players[0].hand = Hand([0, 0, 0, 2, 3]);
        g.apply(0, Action::BuildCity { vertex: v }, &mut rng).unwrap();
        assert_eq!(diff(&before, &g.view_for(Some(1))), vec![FreshKind::Piece(v)]);

        // The robber moving to a new tile.
        let before = snap(&g);
        g.phase = Phase::MoveRobber { back_to_roll: false };
        let tile = (0..19)
            .find(|&t| t != g.robber && g.board.tiles[t].vertices.iter().all(|&v| g.buildings[v].is_none()))
            .unwrap();
        g.apply(0, Action::MoveRobber { tile, victim: None }, &mut rng).unwrap();
        assert_eq!(diff(&before, &g.view_for(Some(1))), vec![FreshKind::Robber(tile)]);
    }
}
