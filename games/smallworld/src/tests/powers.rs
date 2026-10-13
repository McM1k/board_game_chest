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

/// Seat 0's Ghouls in decline hold region 7, ready to conquer with
/// `hand`; seat 0's active race is `race` with `power`.
fn ghouls_in_decline(race: Race, power: Power, hand: u8) -> State {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ghouls, Power::Stout);
    place(&mut s, 7, Race::Ghouls, 1);
    s.players[0].active.as_mut().unwrap().hand = 0;
    s.this_turn.may_decline = true;
    s.apply(0, Action::Decline).unwrap();
    give(&mut s, 0, race, power);
    s.phase = Phase::Conquest { ghouls: true };
    s.players[0].declined[0].hand = hand;
    s
}

#[test]
fn ghouls_in_decline_pay_corrupt_and_scarecrows_but_break_no_peace() {
    let mut s = ghouls_in_decline(Race::Ratmen, Power::PeaceLoving, 3);
    give(&mut s, 1, Race::Scarecrows, Power::Corrupt);
    place(&mut s, 6, Race::Scarecrows, 1);
    s.players[1].active.as_mut().unwrap().hand = 0;
    s.current = 0;
    s.phase = Phase::Conquest { ghouls: true };
    s.players[0].coins = 10;
    s.players[1].coins = 10;
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    assert!(s.regions[6].tokens.is_some_and(|t| t.race == Race::Ghouls));
    // The Scarecrows' coin and Corrupt's, whoever conquers.
    assert_eq!((s.players[0].coins, s.players[1].coins), (10 + 1 - 1, 10 + 1));
    // The Peace-Loving race attacked nobody.
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(s.turn_coins(0), 1 + 1 + PEACE_COINS);

    // Nor did a Diplomat whose Ghouls in decline attacked.
    let mut s = ghouls_in_decline(Race::Ratmen, Power::Diplomat, 3);
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 6, Race::Humans, 1);
    place(&mut s, 13, Race::Humans, 1);
    s.players[1].active.as_mut().unwrap().hand = 0;
    s.current = 0;
    s.phase = Phase::Conquest { ghouls: true };
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(s.moves(0).allies, vec![1]);
}

#[test]
fn a_copied_behemoth_power_brings_its_behemoths() {
    let mut s = packs_game();
    s.offers[0].power = Power::Behemoth;
    give(&mut s, 0, Race::Ratmen, Power::Copycat);
    place(&mut s, 5, Race::Ratmen, 1);
    s.apply(0, Action::Copy { offer: Some(0) }).unwrap();
    assert_eq!(s.players[0].active.unwrap().behemoths, BEHEMOTHS);
    // One Swamp: Mountain 4 costs 3 - 1.
    assert_eq!(s.moves(0).costs(RegionMove::Behemoth), vec![(3, 1), (4, 2), (6, 1), (7, 1)]);
    s.apply(0, Action::Behemoth { region: 4 }).unwrap();
    assert_eq!(s.regions[4].behemoths, 1);
    // Whoever takes the power takes the Behemoths with it.
    end_turn(&mut s, 0);
    s.apply(1, Action::Pick { offer: 0 }).unwrap();
    let a = s.players[0].active.unwrap();
    assert_eq!((a.copied, a.behemoths, s.regions[4].behemoths), (None, 0, 0));
}

// ----- the base game ---------------------------------------------------------------------------

/// Seat 0's active race goes into decline at the start of its turn.
fn decline(s: &mut State) {
    s.current = 0;
    s.phase = Phase::Conquest { ghouls: false };
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    s.apply(0, Action::Decline).unwrap();
}

#[test]
fn in_decline_the_powers_pieces_go_but_fortresses_and_lairs_stay() {
    let pieces = |s: &State, r: usize| {
        let x = &s.regions[r];
        (x.hole, x.hero, x.dragon, x.encampments, x.catapult, x.cannon, x.behemoths)
    };
    let mut s = empty_game();
    give(&mut s, 0, Race::Halflings, Power::Heroic);
    for r in [6, 4] {
        s.apply(0, Action::Conquer { region: r }).unwrap();
    }
    s.regions[6].hero = true;
    s.regions[4].hero = true;
    decline(&mut s);
    assert_eq!([6, 4].map(|r| pieces(&s, r)), [(false, false, false, 0, false, None, 0); 2]);

    let mut s = packs_game();
    give(&mut s, 0, Race::Trolls, Power::DragonMaster);
    s.apply(0, Action::Dragon { region: 7 }).unwrap();
    assert!(s.regions[7].dragon && s.regions[7].lair);
    decline(&mut s);
    assert!(!s.regions[7].dragon && s.regions[7].lair);

    for power in [Power::Catapult, Power::Gunner, Power::Behemoth, Power::Fortified] {
        let mut s = packs_game();
        give(&mut s, 0, Race::Ratmen, power);
        place(&mut s, 7, Race::Ratmen, 1);
        let r = &mut s.regions[7];
        (r.catapult, r.cannon, r.behemoths, r.fortress) = (true, Some(true), 1, true);
        decline(&mut s);
        assert_eq!(pieces(&s, 7), (false, false, false, 0, false, None, 0), "{power:?}");
        // The fortress stays and defends, but pays no more.
        assert!(s.regions[7].fortress);
        assert_eq!(s.turn_coins(0), 1);
        give(&mut s, 1, Race::Wizards, Power::Stout);
        place(&mut s, 6, Race::Wizards, 1);
        assert_eq!(s.conquest_cost(1, 7), Ok(2 + 1 + 1));
    }
}

#[test]
fn race_and_power_bonuses_stop_in_decline_but_the_dwarves() {
    let cases: [(Race, Power, &[usize]); 6] = [
        (Race::Humans, Power::Merchant, &[2, 6]),
        (Race::Wizards, Power::Alchemist, &[2, 8]),
        (Race::Ratmen, Power::Forest, &[3, 8]),
        (Race::Ratmen, Power::Hill, &[7, 10]),
        (Race::Ratmen, Power::Swamp, &[5, 16]),
        (Race::Orcs, Power::Pillaging, &[7, 6]),
    ];
    for (race, power, regions) in cases {
        let mut s = empty_game();
        give(&mut s, 0, race, power);
        for &r in regions {
            place(&mut s, r, race, 1);
        }
        s.this_turn.plundered = 2;
        assert!(s.turn_coins(0) > 2, "{race:?} {power:?}");
        decline(&mut s);
        assert_eq!(s.turn_coins(0), 2, "{race:?} {power:?}");
    }
    // Dwarves still dig.
    let mut s = empty_game();
    give(&mut s, 0, Race::Dwarves, Power::Hill);
    place(&mut s, 1, Race::Dwarves, 1);
    place(&mut s, 7, Race::Dwarves, 1);
    decline(&mut s);
    assert_eq!(s.turn_coins(0), 2 + 1);
}

#[test]
fn elves_in_decline_lose_a_token_like_anyone() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Elves, Power::Stout);
    place(&mut s, 6, Race::Elves, 1);
    place(&mut s, 13, Race::Elves, 1);
    decline(&mut s);
    give(&mut s, 1, Race::Ratmen, Power::Stout);
    place(&mut s, 7, Race::Ratmen, 1);
    s.apply(1, Action::Conquer { region: 6 }).unwrap();
    assert_eq!((s.players[0].declined[0].hand, s.on_map(Race::Elves)), (0, 1));
}

#[test]
fn skeletons_and_sorcerers_take_only_what_the_box_holds() {
    let mut s = empty_game();
    for r in [7, 6, 5, 8] {
        s.regions[r].lost_tribe = true;
    }
    give(&mut s, 0, Race::Skeletons, Power::Commando);
    for r in [7, 6, 5, 8] {
        s.apply(0, Action::Conquer { region: r }).unwrap();
    }
    // Two for four Lost Tribes, but the box holds 20: 10 out, 9 more on the map.
    place(&mut s, 13, Race::Skeletons, 9);
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(s.on_map(Race::Skeletons) + hand(&s, 0), Race::Skeletons.supply());
    assert!(s.log.contains(&Event::Raised { seat: 0, race: Race::Skeletons, tokens: 1 }));

    let mut s = empty_game();
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 6, Race::Humans, 1);
    give(&mut s, 0, Race::Sorcerers, Power::Merchant);
    place(&mut s, 7, Race::Sorcerers, 1);
    s.players[0].active.as_mut().unwrap().hand = Race::Sorcerers.supply() - 1;
    assert_eq!(s.apply(0, Action::Enchant { region: 6 }), Err(Error::NoneLeft));
}

#[test]
fn halflings_land_anywhere_but_the_water() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Halflings, Power::Stout);
    assert_eq!(s.apply(0, Action::Conquer { region: 11 }), Err(Error::Water));
    s.apply(0, Action::Conquer { region: 14 }).unwrap();
    // Then next to their regions, like anyone.
    assert_eq!(s.apply(0, Action::Conquer { region: 7 }), Err(Error::OutOfReach));
}

#[test]
fn trolls_lairs_and_fortresses_go_with_the_region() {
    let mut s = empty_game();
    give(&mut s, 1, Race::Trolls, Power::Fortified);
    place(&mut s, 6, Race::Trolls, 1);
    place(&mut s, 13, Race::Trolls, 1);
    s.regions[6].lair = true;
    s.regions[6].fortress = true;
    s.regions[13].lair = true;
    give(&mut s, 0, Race::Ratmen, Power::Stout);
    place(&mut s, 7, Race::Ratmen, 1);
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    assert!(!s.regions[6].lair && !s.regions[6].fortress);
    // Abandoned: gone too.
    s.current = 1;
    s.this_turn = Turn::default();
    s.apply(1, Action::Abandon { region: 13 }).unwrap();
    assert!(!s.regions[13].lair);
}

#[test]
fn fortresses_one_a_region_six_on_the_map() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Fortified);
    for r in [2, 3, 5, 6, 7, 8, 13] {
        place(&mut s, r, Race::Ratmen, 1);
    }
    s.phase = Phase::Redeploy;
    s.regions[7].fortress = true;
    assert_eq!(s.apply(0, Action::Fortify { region: 7 }), Err(Error::NotHere));
    for r in [2, 3, 5, 6, 8] {
        s.regions[r].fortress = true;
    }
    assert_eq!(s.apply(0, Action::Fortify { region: 13 }), Err(Error::NoneLeft));
    // A coin each while active.
    assert_eq!(s.turn_coins(0), 7 + 6);
}

#[test]
fn the_dragon_flies_to_a_new_region_each_turn() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::DragonMaster);
    s.apply(0, Action::Dragon { region: 7 }).unwrap();
    end_turn(&mut s, 0);
    pass(&mut s, 1);
    // Still there at the start of the next turn; it leaves when it attacks again.
    assert!(s.regions[7].dragon);
    s.apply(0, Action::Dragon { region: 6 }).unwrap();
    assert!(!s.regions[7].dragon && s.regions[6].dragon);
}

#[test]
fn encampments_survive_an_attack_and_pitch_again() {
    let mut s = empty_game();
    give(&mut s, 1, Race::Ratmen, Power::Bivouacking);
    place(&mut s, 6, Race::Ratmen, 1);
    place(&mut s, 13, Race::Ratmen, 1);
    s.regions[6].encampments = 5;
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Giants, Power::Stout);
    s.players[0].active.as_mut().unwrap().hand = 20;
    place(&mut s, 7, Race::Giants, 1);
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    // The five come back to the Ratmen for their next redeployment.
    assert_eq!(s.regions[6].encampments, 0);
    s.current = 1;
    s.phase = Phase::Redeploy;
    for _ in 0..ENCAMPMENTS {
        s.apply(1, Action::Camp { region: 13 }).unwrap();
    }
    assert_eq!(s.regions[13].encampments, ENCAMPMENTS);
}
