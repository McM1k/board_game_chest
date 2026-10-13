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

// ----- Cursed!, Grand Dames, Be Not Afraid... ----------------------------------------------------

#[test]
fn kobolds_in_decline_keep_one_a_region() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Kobolds, Power::Stout);
    place(&mut s, 7, Race::Kobolds, 3);
    place(&mut s, 6, Race::Kobolds, 2);
    decline(&mut s);
    assert_eq!((tokens(&s, 7), tokens(&s, 6)), (1, 1));
    give(&mut s, 1, Race::Ratmen, Power::Stout);
    place(&mut s, 5, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(1, 6), Ok(2 + 1));
}

#[test]
fn ransacking_takes_only_from_an_active_race_with_coins() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 6, Race::Humans, 1);
    s.current = 1;
    decline_seat(&mut s, 1);
    give(&mut s, 1, Race::Wizards, Power::Stout);
    place(&mut s, 8, Race::Wizards, 1);
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Ratmen, Power::Ransacking);
    place(&mut s, 7, Race::Ratmen, 1);
    let coins = s.players[0].coins;
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    assert_eq!(s.players[0].coins, coins, "the Humans are in decline");
    s.players[1].coins = 0;
    s.apply(0, Action::Conquer { region: 8 }).unwrap();
    assert_eq!(s.players[0].coins, coins, "the Wizards' player is broke");
}

/// Seat `seat`'s active race goes into decline at the start of its turn.
fn decline_seat(s: &mut State, seat: usize) {
    s.current = seat;
    s.phase = Phase::Conquest { ghouls: false };
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    s.apply(seat, Action::Decline).unwrap();
}

#[test]
fn the_ivory_tower_rises_on_a_fortress_else_a_mountain() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Priestesses, Power::Fortified);
    for r in [4, 5, 6] {
        place(&mut s, r, Race::Priestesses, 1);
    }
    s.regions[6].fortress = true;
    decline(&mut s);
    assert_eq!(s.regions_of(Race::Priestesses), vec![6]);
    assert!(s.regions[6].fortress);

    let mut s = packs_game();
    give(&mut s, 0, Race::Priestesses, Power::Stout);
    for r in [5, 4, 6] {
        place(&mut s, r, Race::Priestesses, 2);
    }
    decline(&mut s);
    assert_eq!((s.regions_of(Race::Priestesses), tokens(&s, 4)), (vec![4], 3));
    // It may still be taken, by a Dragon too.
    give(&mut s, 1, Race::Ratmen, Power::DragonMaster);
    place(&mut s, 10, Race::Ratmen, 1);
    s.apply(1, Action::Dragon { region: 4 }).unwrap();
    assert!(s.regions_of(Race::Priestesses).is_empty());
}

#[test]
fn white_ladies_in_decline_shrug_off_dragons_and_lava() {
    let mut s = packs_game();
    give(&mut s, 0, Race::WhiteLadies, Power::Stout);
    place(&mut s, 6, Race::WhiteLadies, 1);
    decline(&mut s);
    give(&mut s, 1, Race::Ratmen, Power::DragonMaster);
    place(&mut s, 7, Race::Ratmen, 1);
    place(&mut s, 4, Race::Ratmen, 1);
    assert_eq!(s.apply(1, Action::Dragon { region: 6 }), Err(Error::Immune));
    s.players[1].active.as_mut().unwrap().power = Power::Lava;
    s.phase = Phase::Redeploy;
    assert_eq!(s.apply(1, Action::Lava { region: 6 }), Err(Error::Immune));
    // Active, they are fair game.
    let mut s = packs_game();
    give(&mut s, 0, Race::WhiteLadies, Power::Stout);
    place(&mut s, 6, Race::WhiteLadies, 1);
    give(&mut s, 1, Race::Ratmen, Power::Stout);
    place(&mut s, 7, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(1, 6), Ok(3));
}

#[test]
fn historians_are_paid_when_they_decline_and_no_more_after() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ratmen, Power::Historian);
    place(&mut s, 7, Race::Ratmen, 1);
    let coins = s.players[0].coins;
    decline(&mut s);
    // A coin for the region, one for the Historians' own decline.
    assert_eq!(s.players[0].coins, coins + 1 + 1);
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 13, Race::Humans, 1);
    let coins = s.players[0].coins;
    decline_seat(&mut s, 1);
    assert_eq!(s.players[0].coins, coins, "in decline, the Historians are done");
}

#[test]
fn peace_lovers_may_attack_races_in_decline() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 6, Race::Humans, 1);
    decline_seat(&mut s, 1);
    give(&mut s, 0, Race::Ratmen, Power::PeaceLoving);
    place(&mut s, 7, Race::Ratmen, 1);
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    assert_eq!(s.turn_coins(0), 2 + PEACE_COINS);
}

#[test]
fn mercenaries_are_hired_after_the_die() {
    let (mut hired, mut lost) = (false, false);
    for seed in 0..30 {
        let mut s = packs_seed(seed);
        give(&mut s, 0, Race::Ratmen, Power::Mercenary);
        place(&mut s, 5, Race::Ratmen, 1);
        // Mountain 4 costs 3; one token in hand needs the die, or a coin.
        s.players[0].active.as_mut().unwrap().hand = 1;
        let coins = s.players[0].coins;
        s.apply(0, Action::Gamble { region: 4 }).unwrap();
        let Some(&Event::Gambled { die, won, .. }) = s.log.iter().rev().find(|e| matches!(e, Event::Gambled { .. })) else {
            panic!("no die")
        };
        assert!(won, "a coin always makes up the two missing");
        let paid = coins - s.players[0].coins;
        assert_eq!(paid, (die < 2) as u32, "die {die}");
        hired |= paid == 1;
        lost |= paid == 0;
        assert_eq!(s.regions[4].race(), Some(Race::Ratmen));
    }
    assert!(hired && lost);
}

// ----- Royal Bonus, A Spider's Web ---------------------------------------------------------------

#[test]
fn shrubmen_forests_stay_immune_in_decline() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Shrubmen, Power::Stout);
    place(&mut s, 8, Race::Shrubmen, 1);
    place(&mut s, 6, Race::Shrubmen, 1);
    decline(&mut s);
    give(&mut s, 1, Race::Ratmen, Power::Stout);
    place(&mut s, 7, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(1, 8), Err(Error::Immune));
    assert!(s.conquest_cost(1, 6).is_ok());
}

#[test]
fn a_behemoths_region_taken_loses_only_its_token() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Ratmen, Power::Behemoth);
    place(&mut s, 6, Race::Ratmen, 1);
    place(&mut s, 5, Race::Ratmen, 1);
    s.regions[6].behemoths = 1;
    let a = s.players[1].active.as_mut().unwrap();
    (a.hand, a.behemoths) = (0, 1);
    give(&mut s, 0, Race::Wizards, Power::Stout);
    s.players[0].active.as_mut().unwrap().hand = 20;
    place(&mut s, 7, Race::Wizards, 1);
    // 2 + 1 Ratman + a Behemoth worth one Swamp.
    assert_eq!(s.conquest_cost(0, 6), Ok(2 + 1 + 1));
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    let a = s.players[1].active.unwrap();
    assert_eq!((a.hand, a.behemoths), (0, 2));
}

#[test]
fn igors_collect_every_token_lost_their_own_too() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Skags, Power::Stout);
    place(&mut s, 6, Race::Skags, 1);
    s.regions[6].loot = true;
    s.loot[6] = Some(Loot::SkagAttack);
    give(&mut s, 0, Race::Igors, Power::Stout);
    place(&mut s, 7, Race::Igors, 1);
    place(&mut s, 5, Race::Humans, 1);
    s.players[1].declined.push(Army::new(Race::Humans, Power::Stout, 0));
    s.regions[5].tokens.as_mut().unwrap().declined = true;
    // An Igor lost to a Skag Attack, a Human taken.
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::Conquer { region: 5 }).unwrap();
    assert_eq!(s.players[0].active.unwrap().collected, 2);
}

#[test]
fn winter_stays_one_a_region_and_thaws_when_the_ice_witches_decline() {
    let mut s = packs_game();
    give(&mut s, 0, Race::IceWitches, Power::Stout);
    place(&mut s, 8, Race::IceWitches, 1);
    place(&mut s, 2, Race::IceWitches, 1);
    s.phase = Phase::Redeploy;
    s.players[0].active.as_mut().unwrap().winter = 2;
    s.apply(0, Action::Winter { region: 8 }).unwrap();
    assert_eq!(s.apply(0, Action::Winter { region: 8 }), Err(Error::NotHere));
    assert_eq!(s.apply(0, Action::Winter { region: 13 }), Err(Error::OutOfReach));
    s.apply(0, Action::Winter { region: 9 }).unwrap();
    // Their own Winter costs them nothing.
    assert_eq!(s.turn_coins(0), 2);
    decline(&mut s);
    assert!(!s.regions[8].winter && !s.regions[9].winter);
}

#[test]
fn skags_keep_their_loot_at_decline_or_at_the_end() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Skags, Power::Stout);
    for (r, loot) in [(7, Loot::Coins(2)), (6, Loot::SkagAttack), (5, Loot::Coins(3))] {
        place(&mut s, r, Race::Skags, 1);
        s.regions[r].loot = true;
        s.loot[r] = Some(loot);
    }
    let coins = s.players[0].coins;
    decline(&mut s);
    assert_eq!(s.players[0].coins, coins + 3 + 2 + 3);
    assert!([7, 6, 5].iter().all(|&r| !s.regions[r].loot && s.loot[r].is_none()));

    // Still active when the game ends: turned over then.
    let mut s = packs_game();
    give(&mut s, 0, Race::Skags, Power::Stout);
    place(&mut s, 7, Race::Skags, 1);
    s.regions[7].loot = true;
    s.loot[7] = Some(Loot::Coins(3));
    s.turn = s.turns();
    end_turn(&mut s, 0);
    let coins = s.players[0].coins;
    pass(&mut s, 1);
    assert_eq!(s.phase, Phase::Over);
    assert_eq!(s.players[0].coins, coins + 3);
}

// ----- Sky Islands -------------------------------------------------------------------------------

#[test]
fn scavengers_drop_their_captives_when_they_decline() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 6, Race::Humans, 1);
    decline_seat(&mut s, 1);
    give(&mut s, 0, Race::Scavengers, Power::Stout);
    place(&mut s, 7, Race::Scavengers, 1);
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    assert_eq!(s.regions[6].captive, Some(Race::Humans));
    // The Humans have no region of their own left, but stay on the map.
    end_turn(&mut s, 0);
    assert_eq!(s.owner(Race::Humans), Some((1, true)));
    decline(&mut s);
    assert_eq!(s.regions[6].captive, None);
    assert_eq!(s.turn_coins(1), 0);
}

#[test]
fn storm_giants_have_two_bolts_for_mountains() {
    let mut s = packs_game();
    give(&mut s, 0, Race::StormGiants, Power::Stout);
    place(&mut s, 3, Race::StormGiants, 1);
    place(&mut s, 18, Race::StormGiants, 1);
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 4, Race::Humans, 5);
    s.current = 0;
    assert_eq!(s.apply(0, Action::Lightning { region: 5 }), Err(Error::NotHere));
    s.apply(0, Action::Lightning { region: 4 }).unwrap();
    s.apply(0, Action::Lightning { region: 19 }).unwrap();
    assert_eq!(s.apply(0, Action::Lightning { region: 1 }), Err(Error::PowerUsed));
    assert_eq!(s.regions_of(Race::StormGiants), vec![3, 4, 18, 19]);
}

#[test]
fn the_drakons_dragons_fly_away_at_their_next_turn() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Drakons, Power::Stout);
    s.apply(0, Action::Dragon { region: 7 }).unwrap();
    give(&mut s, 1, Race::Ratmen, Power::Stout);
    place(&mut s, 5, Race::Ratmen, 1);
    s.current = 1;
    assert_eq!(s.conquest_cost(1, 7), Err(Error::Immune), "immune until then");
    s.current = 0;
    s.phase = Phase::Conquest { ghouls: false };
    s.players[1].active = None;
    s.regions[5].tokens = None;
    end_turn(&mut s, 0);
    pass(&mut s, 1);
    assert!(!s.regions[7].dragon);
    assert_eq!(s.this_turn.drakons, 0, "three more this turn");
}

#[test]
fn wendigos_clear_races_in_decline_and_elves_alike() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 14, Race::Humans, 1);
    place(&mut s, 13, Race::Humans, 1);
    decline_seat(&mut s, 1);
    s.regions[14].tokens.as_mut().unwrap().count = 3;
    give(&mut s, 0, Race::Wendigos, Power::Stout);
    s.apply(0, Action::Wendigo { region: 14 }).unwrap();
    assert_eq!((s.players[1].declined[0].hand, s.on_map(Race::Humans)), (0, 1));

    let mut s = packs_game();
    give(&mut s, 1, Race::Elves, Power::Stout);
    place(&mut s, 3, Race::Elves, 2);
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Wendigos, Power::Stout);
    s.apply(0, Action::Wendigo { region: 3 }).unwrap();
    assert_eq!(hand(&s, 1), 1, "Elves lose one too");
}

#[test]
fn five_zeppelins_a_turn() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ratmen, Power::Zeppelined);
    s.players[0].active.as_mut().unwrap().hand = 40;
    for (i, r) in [2, 6, 10, 14, 18].into_iter().enumerate() {
        s.apply(0, Action::Zeppelin { region: r }).unwrap();
        assert_eq!(s.this_turn.zeppelins, i as u8 + 1);
    }
    assert_eq!(s.apply(0, Action::Zeppelin { region: 21 }), Err(Error::PowerUsed));
}

#[test]
fn trade_pacts_are_limited_by_the_hagglers_coins() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ratmen, Power::Haggling);
    s.players[0].coins = 1;
    s.apply(0, Action::Pact { seat: 1, give: true }).unwrap();
    assert_eq!(s.apply(0, Action::Pact { seat: 1, give: true }), Err(Error::NotEnoughCoins));
    s.players[0].coins = 10;
    for _ in 1..PACTS {
        s.apply(0, Action::Pact { seat: 1, give: true }).unwrap();
    }
    assert_eq!(s.apply(0, Action::Pact { seat: 1, give: true }), Err(Error::NoneLeft));
}

#[test]
fn a_fired_cannon_stays_and_cannons_shield_their_regions() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ratmen, Power::Gunner);
    place(&mut s, 6, Race::Ratmen, 1);
    place(&mut s, 7, Race::Ratmen, 1);
    s.regions[6].cannon = Some(false);
    s.regions[7].cannon = Some(false);
    s.apply(0, Action::Fire { region: 6 }).unwrap();
    assert_eq!(s.apply(0, Action::Fire { region: 6 }), Err(Error::PowerUsed));
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(s.apply(0, Action::Cannon { region: 6 }), Err(Error::PowerUsed), "it fired: it stays");
    s.apply(0, Action::Cannon { region: 7 }).unwrap();
    give(&mut s, 1, Race::Wizards, Power::Stout);
    place(&mut s, 5, Race::Wizards, 1);
    assert_eq!(s.conquest_cost(1, 6), Err(Error::Immune));
    assert!(s.conquest_cost(1, 7).is_ok(), "its Cannon moved");
}
