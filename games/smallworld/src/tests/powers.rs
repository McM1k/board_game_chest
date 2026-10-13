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

/// A two-player game with every expansion but the Sky Islands, from `seed`.
fn packs_seed(seed: u64) -> State {
    no_tribes(State::with_options(2, seed, Options { sky: None, ..Options::all() }))
}

fn raised(s: &State, race: Race) -> u8 {
    s.log.iter().map(|e| if let Event::Raised { race: r, tokens, .. } = *e { if r == race { tokens } else { 0 } } else { 0 }).sum()
}

#[test]
fn pygmies_breed_whatever_takes_a_pygmy() {
    let (mut wendigos, mut skags, mut zeppelins) = (0, 0, 0);
    for seed in 0..30 {
        // The Wendigos clear their Forest: one Pygmy lost, two back in hand.
        let mut s = packs_seed(seed);
        give(&mut s, 1, Race::Pygmies, Power::Stout);
        place(&mut s, 14, Race::Pygmies, 3);
        s.players[1].active.as_mut().unwrap().hand = 0;
        give(&mut s, 0, Race::Wendigos, Power::Stout);
        s.apply(0, Action::Wendigo { region: 14 }).unwrap();
        assert_eq!(hand(&s, 1), 2 + raised(&s, Race::Pygmies));
        wendigos += raised(&s, Race::Pygmies);

        // Pygmies attacking the Skags fall to a Skag Attack.
        let mut s = packs_seed(seed);
        give(&mut s, 1, Race::Skags, Power::Stout);
        place(&mut s, 7, Race::Skags, 1);
        s.regions[7].loot = true;
        s.loot[7] = Some(Loot::SkagAttack);
        give(&mut s, 0, Race::Pygmies, Power::Stout);
        place(&mut s, 6, Race::Pygmies, 1);
        s.players[0].active.as_mut().unwrap().hand = 5;
        s.apply(0, Action::Conquer { region: 7 }).unwrap();
        assert_eq!(hand(&s, 0), 4 + raised(&s, Race::Pygmies));
        skags += raised(&s, Race::Pygmies);

        // A Pygmy Zeppelin crashes.
        let mut s = packs_seed(seed);
        give(&mut s, 0, Race::Pygmies, Power::Zeppelined);
        s.apply(0, Action::Zeppelin { region: 15 }).unwrap();
        if s.regions[15].burning.is_some() {
            assert_eq!(hand(&s, 0), 6 + 5 - 1 + raised(&s, Race::Pygmies));
            zeppelins += raised(&s, Race::Pygmies);
        }
    }
    assert!(wendigos > 0 && skags > 0 && zeppelins > 0, "{wendigos} {skags} {zeppelins}");
}

#[test]
fn a_sorcerers_enchantment_is_a_conquest() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Scarecrows, Power::Corrupt);
    place(&mut s, 6, Race::Scarecrows, 1);
    s.regions[6].fortress = true;
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Sorcerers, Power::Ransacking);
    place(&mut s, 7, Race::Sorcerers, 1);
    // Nothing in hand: the Sorcerer comes from the box.
    s.players[0].active.as_mut().unwrap().hand = 0;
    s.players[0].coins = 10;
    s.players[1].coins = 10;
    s.apply(0, Action::Enchant { region: 6 }).unwrap();
    assert_eq!(s.regions[6].tokens.map(|t| (t.race, t.count)), Some((Race::Sorcerers, 1)));
    assert!(!s.regions[6].fortress);
    // Corrupt, Ransacking and the Scarecrows, as for any conquest.
    let paid: Vec<Why> = s.log.iter().filter_map(|e| if let Event::Paid { why, .. } = e { Some(*why) } else { None }).collect();
    assert_eq!(paid, [Why::Corrupt, Why::Ransacking, Why::Scarecrows]);
    assert_eq!((s.players[0].coins, s.players[1].coins), (10 - 1 + 1 + 1, 10 + 1 - 1));
    assert_eq!(s.this_turn.attacked, [1]);

    // An Elf replaced is lost; a Pygmy breeds; the Loot is the Sorcerers'.
    let mut s = packs_game();
    give(&mut s, 1, Race::Elves, Power::Stout);
    place(&mut s, 6, Race::Elves, 1);
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Sorcerers, Power::Stout);
    place(&mut s, 7, Race::Sorcerers, 1);
    s.apply(0, Action::Enchant { region: 6 }).unwrap();
    assert_eq!((hand(&s, 1), s.on_map(Race::Elves)), (0, 0));

    let mut bred = 0;
    for seed in 0..20 {
        let mut s = packs_seed(seed);
        give(&mut s, 1, Race::Pygmies, Power::Stout);
        place(&mut s, 6, Race::Pygmies, 1);
        place(&mut s, 13, Race::Pygmies, 1);
        s.players[1].active.as_mut().unwrap().hand = 0;
        give(&mut s, 0, Race::Sorcerers, Power::Stout);
        place(&mut s, 7, Race::Sorcerers, 1);
        s.apply(0, Action::Enchant { region: 6 }).unwrap();
        assert_eq!(hand(&s, 1), raised(&s, Race::Pygmies));
        bred += hand(&s, 1);
    }
    assert!(bred > 0);

    let mut s = packs_game();
    give(&mut s, 1, Race::Skags, Power::Stout);
    place(&mut s, 6, Race::Skags, 1);
    s.regions[6].loot = true;
    s.loot[6] = Some(Loot::Coins(3));
    give(&mut s, 0, Race::Sorcerers, Power::Stout);
    place(&mut s, 7, Race::Sorcerers, 1);
    let coins = s.players[0].coins;
    s.apply(0, Action::Enchant { region: 6 }).unwrap();
    assert_eq!(s.players[0].coins, coins + 3);
    assert_eq!(s.loot[6], None);
}
