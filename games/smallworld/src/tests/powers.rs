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

#[test]
fn escargots_are_paid_at_the_start_what_the_winter_and_their_power_leave() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Escargots, Power::Aquatic);
    // Coastal 3; inland 6; inland 7 in the Winter.
    for r in [3, 6, 7] {
        place(&mut s, r, Race::Escargots, 1);
    }
    s.regions[7].winter = true;
    // At the end, only what Aquatic adds: the coastal region.
    assert_eq!(s.turn_coins(0), 1);
    s.current = 1;
    s.phase = Phase::Pick;
    let coins = s.players[0].coins;
    pass(&mut s, 1);
    // At the start: region 3's coin; Aquatic takes 6's, the Winter 7's.
    assert_eq!(s.current, 0);
    assert_eq!(s.players[0].coins, coins + 1);
}
