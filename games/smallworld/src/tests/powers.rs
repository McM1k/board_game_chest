//! Each race and power checked against its rule sheet, for what the tests
//! in `tests.rs` and `expansions.rs` leave out: decline, the other races'
//! and powers' tricks, and the limits. The two-player board, as listed at
//! the top of `tests.rs`.

use super::expansions::{end_turn, packs_game, pass};
use super::*;

#[test]
fn each_mountain_pours_its_own_lava_next_to_it() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Humans, Power::Stout);
    for r in [5, 6, 15, 18] {
        place(&mut s, r, Race::Humans, 1);
    }
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Ratmen, Power::Lava);
    place(&mut s, 4, Race::Ratmen, 1);
    place(&mut s, 19, Race::Ratmen, 1);
    s.apply(0, Action::EndConquests).unwrap();
    // Two Mountains, two Lavas; 5 and 6 are both next to Mountain 4 only.
    s.apply(0, Action::Lava { region: 6 }).unwrap();
    assert_eq!(s.apply(0, Action::Lava { region: 5 }), Err(Error::NoneLeft));
    assert!(!s.moves(0).can(RegionMove::Lava, 5));
    // 15 touches both: Mountain 19 pours it.
    assert!(s.moves(0).can(RegionMove::Lava, 18) && s.moves(0).can(RegionMove::Lava, 15));
    s.apply(0, Action::Lava { region: 15 }).unwrap();
    assert_eq!(s.apply(0, Action::Lava { region: 18 }), Err(Error::NoneLeft));
}
