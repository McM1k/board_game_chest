//! The expansions' races, powers and the Sky Islands. Most tests use the
//! two-player board (see the list at the top of `tests.rs`).

use super::*;

/// A two-player game with every expansion's races and powers but the base
/// board, and no Lost Tribes.
fn packs_game() -> State {
    let options = Options { sky: None, ..Options::all() };
    no_tribes(State::with_options(2, 1, options))
}

/// Ends seat's turn: done conquering, every token in hand on its first
/// region, end turn.
fn end_turn(s: &mut State, seat: usize) {
    if matches!(s.phase, Phase::Conquest { .. }) {
        s.apply(seat, Action::EndConquests).unwrap();
    }
    if matches!(s.phase, Phase::Conquest { .. }) {
        s.apply(seat, Action::EndConquests).unwrap();
    }
    while !s.moves(seat).can_end_turn {
        let m = s.moves(seat);
        let put = m.list(RegionMove::Put);
        let action = if m.end_turn.as_deref() == Some(Error::RestAmazons.message()) && hand(s, seat) < AMAZON_EXTRA {
            Action::Take { region: m.list(RegionMove::Take)[0] }
        } else {
            Action::Put { region: put[0] }
        };
        s.apply(seat, action).unwrap();
    }
    s.apply(seat, Action::EndTurn).unwrap();
}

/// Lets the other seat of a two-player game pass a turn (with no race of
/// its own, it picks the top combo and does nothing with it).
fn pass(s: &mut State, seat: usize) {
    while s.phase == Phase::Reinforce {
        let who = s.actor().unwrap();
        let region = s.moves(who).list(RegionMove::Put)[0];
        s.apply(who, Action::Put { region }).unwrap();
    }
    assert_eq!(s.current, seat);
    if s.phase == Phase::Pick {
        s.apply(seat, Action::Pick { offer: 0 }).unwrap();
    }
    end_turn(s, seat);
    while s.phase == Phase::Reinforce {
        let who = s.actor().unwrap();
        let region = s.moves(who).list(RegionMove::Put)[0];
        s.apply(who, Action::Put { region }).unwrap();
    }
}

// ----- the lobby's options ---------------------------------------------------------------

#[test]
fn each_expansion_adds_its_races_and_powers() {
    let counts = [
        (Pack::Cursed, 2, 5),
        (Pack::GrandDames, 3, 2),
        (Pack::BeNotAfraid, 5, 5),
        (Pack::RoyalBonus, 3, 3),
        (Pack::SpidersWeb, 3, 3),
        (Pack::SkyIslands, 7, 7),
    ];
    for (pack, races, powers) in counts {
        let mut options = Options::default();
        options.set(pack, true);
        assert!(options.has(pack));
        assert_eq!(options.races().len(), 14 + races, "{pack:?}");
        assert_eq!(options.powers().len(), 20 + powers, "{pack:?}");
        options.set(pack, false);
        assert_eq!(options, Options::default());
    }
    let all = Options::all();
    assert_eq!((all.races().len(), all.powers().len()), (37, 45));
    assert_eq!((Options::default().min_players(), Options::default().max_players()), (2, 5));
    assert_eq!((all.min_players(), all.max_players()), (3, 6));
    // Every race and power belongs to a pack, and the box holds enough tokens.
    for race in Race::ALL {
        assert!(race.supply() >= race.tokens() + 5, "{race:?}");
        assert!(!race.blurb().is_empty());
    }
    for power in Power::ALL {
        assert!(!power.blurb().is_empty());
    }
    let s = State::with_options(3, 2, Options { sky: None, ..Options::all() });
    assert!(s.races.iter().chain(s.offers.iter().map(|o| &o.race)).any(|r| r.pack() != Pack::Base));
}

#[test]
fn options_survive_json_and_old_payloads_parse() {
    let all = Options::all();
    let json = serde_json::to_string(&all).unwrap();
    assert_eq!(serde_json::from_str::<Options>(&json).unwrap(), all);
    assert_eq!(serde_json::from_str::<Options>("{}").unwrap(), Options::default());
}

// ----- the Sky Islands ---------------------------------------------------------------------

#[test]
fn the_sky_islands_sit_beside_the_board_for_one_player_fewer() {
    for (players, sky) in [(3, Sky::Three), (4, Sky::Two), (5, Sky::Three), (6, Sky::Two)] {
        let options = Options { sky: Some(sky), ..Options::default() };
        let s = State::with_options(players, 9, options);
        let ground = Map::for_players(players - 1).regions.len();
        assert_eq!(s.board.ground, players - 1);
        assert_eq!(s.regions.len(), ground + sky.regions().len());
        assert_eq!(s.turns(), [10, 9, 8, 8][players - 3]);
        assert!(s.board.regions[ground..].iter().all(|r| r.sky));
        assert!((0..s.regions.len()).filter(|&r| s.board.regions[r].sky).all(|r| !s.board.border(r)));
        // The Beanstalk and the Stairway: inland, apart, joined to their landings.
        assert_eq!(s.board.access.len(), 2);
        let [(a, la), (b, lb)] = [s.board.access[0], s.board.access[1]];
        assert!(!s.board.border(a) && !s.board.border(b));
        assert!(s.board.distance(a, b) >= 3, "{players}: {a} and {b}");
        assert_eq!([la - ground, lb - ground], sky.landings());
        assert!(s.board.adjacent(a, la) && s.board.adjacent(lb, b));
        // Every island is reachable from the ground.
        for island in &s.board.islands {
            assert!(island.iter().all(|&r| s.board.distance(a, r) < usize::MAX));
        }
    }
}

#[test]
fn the_sky_boards_are_consistent() {
    for sky in [Sky::Two, Sky::Three] {
        let regions = sky.regions();
        for (i, r) in regions.iter().enumerate() {
            for &j in r.adjacent {
                assert!(regions[j as usize].adjacent.contains(&(i as u8)), "{sky:?}: {i} and {j}");
            }
        }
        let mut all: Vec<u8> = sky.islands().concat();
        all.sort();
        assert_eq!(all, (0..regions.len() as u8).collect::<Vec<_>>());
        // The cloud bridges join every island.
        let board = Board::new(2, Some(sky), &[4, 6]);
        let n = 23;
        assert!((n..board.len()).all(|r| board.distance(n, r) < usize::MAX));
    }
    assert_eq!(Sky::Three.islands().iter().map(|i| i.len()).collect::<Vec<_>>(), [2, 4, 3]);
}

#[test]
fn races_reach_the_sky_through_the_beanstalk_and_islands_held_whole_pay() {
    let mut s = no_tribes(State::with_options(3, 4, Options { sky: Some(Sky::Three), ..Options::default() }));
    let (ground, landing) = s.board.access[0];
    let sky_region = s.board.len() - 1;
    give(&mut s, 0, Race::Ratmen, Power::Commando);
    // Nobody starts in the sky.
    assert_eq!(s.conquest_cost(0, sky_region), Err(Error::NotABorder));
    place(&mut s, ground, Race::Ratmen, 1);
    assert!(s.conquest_cost(0, landing).is_ok(), "the Beanstalk leads there");
    // Storm Giants may start up there.
    let mut s = no_tribes(State::with_options(3, 4, Options { sky: Some(Sky::Three), ..Options::default() }));
    give(&mut s, 0, Race::StormGiants, Power::Commando);
    assert!(s.conquest_cost(0, sky_region).is_ok());
    // The smallest island, held whole: one coin more.
    let first = s.board.islands[0].clone();
    give(&mut s, 1, Race::Ratmen, Power::Stout);
    for &r in &first {
        place(&mut s, r, Race::Ratmen, 1);
    }
    assert_eq!(s.turn_coins(1), first.len() as u32 + 1);
    place(&mut s, first[0], Race::StormGiants, 1);
    assert_eq!(s.turn_coins(1), first.len() as u32 - 1);
}

#[test]
fn exploring_pays_for_the_fewer_of_ground_and_sky_regions() {
    let mut s = no_tribes(State::with_options(3, 4, Options::all()));
    give(&mut s, 0, Race::Ratmen, Power::Exploring);
    let sky: Vec<usize> = (0..s.board.len()).filter(|&r| s.board.regions[r].sky).collect();
    for &r in &[7usize, 8, 13] {
        place(&mut s, r, Race::Ratmen, 1);
    }
    // Two sky regions on different islands, so no island is held whole.
    place(&mut s, sky[0], Race::Ratmen, 1);
    place(&mut s, sky[2], Race::Ratmen, 1);
    assert_eq!(s.turn_coins(0), 5 + 2);
}

// ----- Cursed! ---------------------------------------------------------------------------

#[test]
fn skipping_a_cursed_combo_costs_three_coins() {
    let mut s = packs_game();
    s.offers[0].power = Power::Cursed;
    s.players[0].coins = 10;
    assert_eq!(s.pick_cost(0, 2), CURSED_SKIP + 1);
    s.apply(0, Action::Pick { offer: 2 }).unwrap();
    assert_eq!(s.players[0].coins, 10 - 4);
    assert_eq!((s.offers[0].coins, s.offers[1].coins), (3, 1));
    // The Cursed combo itself: the race's tokens only.
    assert_eq!(Power::Cursed.tokens(), 0);
}

#[test]
fn hordes_add_two_tokens_that_vanish_in_decline() {
    let mut s = packs_game();
    s.offers[0] = Offer { race: Race::Ratmen, power: Power::HordesOf, coins: 0, tokens: 0 };
    s.races.retain(|&r| r != Race::Ratmen);
    s.apply(0, Action::Pick { offer: 0 }).unwrap();
    let a = s.players[0].active.unwrap();
    assert_eq!((a.hand, a.extra), (8 + 5 + HORDES, HORDES));
    assert_eq!(s.available(Race::Ratmen), 0, "13 Ratmen in the box, all in hand with the Hordes");
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    s.apply(0, Action::Decline).unwrap();
    assert_eq!(s.players[0].declined[0].extra, 0);
}

#[test]
fn marauding_conquers_a_second_time() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ratmen, Power::Marauding);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.players[0].active.as_mut().unwrap().hand = 1;
    assert!(s.moves(0).gamble().is_empty(), "the die comes after the second round");
    s.apply(0, Action::EndConquests).unwrap();
    // Back in hand, one left per region, and conquering again.
    assert_eq!(s.phase, Phase::Conquest { ghouls: false });
    assert_eq!((tokens(&s, 7), tokens(&s, 6), hand(&s, 0)), (1, 1, 1 + 1 + 1));
    s.players[0].active.as_mut().unwrap().hand = 1;
    assert!(!s.moves(0).gamble().is_empty());
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(s.phase, Phase::Redeploy);
}

#[test]
fn ransacking_corrupt_and_scarecrows_move_coins() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Scarecrows, Power::Corrupt);
    place(&mut s, 6, Race::Scarecrows, 1);
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Ratmen, Power::Ransacking);
    place(&mut s, 7, Race::Ratmen, 1);
    s.players[0].coins = 10;
    s.players[1].coins = 10;
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    // Corrupt: 1 to the victim; Ransacking: 1 back; Scarecrows: 1 from the bank.
    assert_eq!((s.players[0].coins, s.players[1].coins), (10 - 1 + 1 + 1, 10 + 1 - 1));
    let paid: Vec<Why> = s.log.iter().filter_map(|e| if let Event::Paid { why, .. } = e { Some(*why) } else { None }).collect();
    assert_eq!(paid, [Why::Corrupt, Why::Ransacking, Why::Scarecrows]);
}

#[test]
fn were_creatures_hunt_at_night() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ratmen, Power::Were);
    place(&mut s, 5, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(0, 4), Ok(3), "turn 1: day");
    s.turn = 2;
    assert_eq!(s.conquest_cost(0, 4), Ok(1));
}

#[test]
fn goblins_prey_on_races_in_decline_and_kobolds_never_go_alone() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Humans, Power::Alchemist);
    place(&mut s, 6, Race::Humans, 1);
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    s.current = 1;
    s.apply(1, Action::Decline).unwrap();
    give(&mut s, 0, Race::Goblins, Power::Stout);
    place(&mut s, 7, Race::Goblins, 1);
    assert_eq!(s.conquest_cost(0, 6), Ok(2), "2 + 1 Human - 1");

    let mut s = packs_game();
    give(&mut s, 0, Race::Kobolds, Power::Commando);
    place(&mut s, 7, Race::Kobolds, 2);
    assert_eq!(s.conquest_cost(0, 6), Ok(2), "Commando can't take them below two");
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(s.apply(0, Action::Take { region: 6 }), Err(Error::LastToken));
    s.apply(0, Action::Put { region: 6 }).unwrap();
    s.apply(0, Action::Take { region: 6 }).unwrap();
}

// ----- Grand Dames ------------------------------------------------------------------------

#[test]
fn gypsies_leave_a_coin_where_they_leave() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Gypsies, Power::Stout);
    place(&mut s, 7, Race::Gypsies, 1);
    place(&mut s, 6, Race::Gypsies, 1);
    s.apply(0, Action::Abandon { region: 7 }).unwrap();
    assert_eq!(s.conquest_cost(0, 7), Err(Error::Abandoned));
    assert_eq!(s.turn_coins(0), 1 + 1);
}

#[test]
fn priestesses_retire_to_an_ivory_tower() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Priestesses, Power::Stout);
    for r in [7, 6, 5] {
        place(&mut s, r, Race::Priestesses, 2);
    }
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    assert_eq!(s.moves(0).list(RegionMove::Tower), vec![5, 6, 7]);
    s.apply(0, Action::Tower { region: 6 }).unwrap();
    assert_eq!(s.regions_of(Race::Priestesses), vec![6]);
    assert_eq!(tokens(&s, 6), 3);
    // A coin per Priestess in the tower.
    assert_eq!(s.turn_coins(0), 3);
}

#[test]
fn white_ladies_and_shrubmen_are_out_of_reach() {
    let mut s = packs_game();
    give(&mut s, 1, Race::WhiteLadies, Power::Stout);
    place(&mut s, 6, Race::WhiteLadies, 1);
    s.current = 1;
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    s.apply(1, Action::Decline).unwrap();
    give(&mut s, 0, Race::Ratmen, Power::Commando);
    place(&mut s, 7, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(0, 6), Err(Error::Immune));

    let mut s = packs_game();
    give(&mut s, 1, Race::Shrubmen, Power::Stout);
    place(&mut s, 8, Race::Shrubmen, 1);
    place(&mut s, 6, Race::Shrubmen, 1);
    give(&mut s, 0, Race::Ratmen, Power::Commando);
    place(&mut s, 7, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(0, 8), Err(Error::Immune), "a Forest");
    assert!(s.conquest_cost(0, 6).is_ok(), "a Farmland");
}

#[test]
fn historians_count_the_races_in_decline() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 13, Race::Humans, 1);
    s.current = 1;
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    s.apply(1, Action::Decline).unwrap();
    // Seat 0 picks the Historians: one race in decline, one coin.
    s.offers[0] = Offer { race: Race::Ratmen, power: Power::Historian, coins: 0, tokens: 0 };
    s.races.retain(|&r| r != Race::Ratmen);
    let before = s.players[0].coins;
    s.apply(0, Action::Pick { offer: 0 }).unwrap();
    assert_eq!(s.players[0].coins, before + 1);
    // Another race goes into decline: one more.
    give(&mut s, 1, Race::Wizards, Power::Stout);
    place(&mut s, 2, Race::Wizards, 1);
    s.current = 1;
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    s.apply(1, Action::Decline).unwrap();
    assert_eq!(s.players[0].coins, before + 2);
}

#[test]
fn peace_lovers_are_paid_for_leaving_everyone_alone() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 6, Race::Humans, 1);
    give(&mut s, 0, Race::Ratmen, Power::PeaceLoving);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    assert_eq!(s.turn_coins(0), 1 + PEACE_COINS);
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    assert_eq!(s.turn_coins(0), 2);
}

// ----- Be Not Afraid... ----------------------------------------------------------------------

#[test]
fn barbarians_and_pixies_sit_out_instead_of_redeploying() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Barbarians, Power::Stout);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    assert!(s.moves(0).list(RegionMove::Put).is_empty() && s.moves(0).can_end_turn);
    s.apply(0, Action::EndTurn).unwrap();
    let a = s.players[0].active.unwrap();
    assert_eq!((a.hand, a.resting), (0, 9 + 4 - 2));

    let mut s = packs_game();
    give(&mut s, 0, Race::Pixies, Power::Stout);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    s.apply(0, Action::EndTurn).unwrap();
    assert_eq!((tokens(&s, 7), tokens(&s, 6)), (1, 1));
    assert_eq!(s.players[0].active.unwrap().resting, 11 + 4 - 2);
}

#[test]
fn a_skipped_homunculi_combo_grows() {
    let mut s = packs_game();
    s.offers[0] = Offer { race: Race::Homunculi, power: Power::Stout, coins: 0, tokens: 0 };
    s.races.retain(|&r| r != Race::Homunculi);
    s.apply(0, Action::Pick { offer: 1 }).unwrap();
    assert_eq!((s.offers[0].coins, s.offers[0].tokens), (1, 1));
    end_turn(&mut s, 0);
    s.apply(1, Action::Pick { offer: 0 }).unwrap();
    let a = s.players[1].active.unwrap();
    assert_eq!(a.hand, 5 + 4 + 1);
    assert_eq!(s.players[1].coins, START_COINS + 1);
}

#[test]
fn leprechauns_pots_of_gold_pay_out_or_get_taken() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Leprechauns, Power::Stout);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    s.apply(0, Action::Pot { region: 7 }).unwrap();
    s.apply(0, Action::Pot { region: 6 }).unwrap();
    while hand(&s, 0) > 0 {
        s.apply(0, Action::Put { region: 7 }).unwrap();
    }
    s.apply(0, Action::EndTurn).unwrap();
    let after_turn = s.players[0].coins;
    // Seat 1 takes region 6 and its pot.
    give(&mut s, 1, Race::Ratmen, Power::Commando);
    s.phase = Phase::Conquest { ghouls: false };
    place(&mut s, 5, Race::Ratmen, 1);
    let before = s.players[1].coins;
    s.apply(1, Action::Conquer { region: 6 }).unwrap();
    assert_eq!(s.players[1].coins, before + 1);
    end_turn(&mut s, 1);
    while s.phase == Phase::Reinforce {
        s.apply(0, Action::Put { region: 7 }).unwrap();
    }
    // Seat 0's next turn: the pot left in region 7 pays.
    assert_eq!(s.current, 0);
    assert_eq!(s.players[0].coins, after_turn + 1);
    assert!(!s.regions[7].pot);
    assert_eq!(s.players[0].active.unwrap().pots, 2);
}

#[test]
fn pygmies_breed_when_they_fall() {
    let mut raised = 0;
    for seed in 0..20 {
        let mut s = no_tribes(State::with_options(2, seed, Options { sky: None, ..Options::all() }));
        give(&mut s, 1, Race::Pygmies, Power::Stout);
        place(&mut s, 6, Race::Pygmies, 2);
        place(&mut s, 5, Race::Pygmies, 1);
        s.players[1].active.as_mut().unwrap().hand = 0;
        give(&mut s, 0, Race::Ratmen, Power::Stout);
        place(&mut s, 7, Race::Ratmen, 1);
        s.apply(0, Action::Conquer { region: 6 }).unwrap();
        let h = hand(&s, 1);
        assert!((1..=1 + DIE_MAX).contains(&h));
        raised += (h - 1) as u32;
    }
    assert!(raised > 5, "{raised}");
}

#[test]
fn barricade_imperial_aquatic_khans_and_goldsmith_score() {
    let cases: [(Race, Power, &[usize], u32); 6] = [
        (Race::Ratmen, Power::Barricade, &[5, 6, 7, 8], 4 + BARRICADE_COINS),
        (Race::Ratmen, Power::Barricade, &[5, 6, 7, 8, 10], 5),
        (Race::Ratmen, Power::Imperial, &[5, 6, 7, 8, 10], 5 + 2),
        // Coastal 3 and 4, inland 6 and 7.
        (Race::Ratmen, Power::Aquatic, &[3, 4, 6, 7], 2 * 2),
        // Hill 7 and Farmland 6, then Swamp 5 and Forest 8.
        (Race::Khans, Power::Stout, &[5, 6, 7, 8], 2 * 2),
        // Mines 1 and 3, then 2 and 4.
        (Race::Ratmen, Power::Goldsmith, &[1, 2, 3, 4], 2 * 3),
    ];
    for (race, power, regions, coins) in cases {
        let mut s = packs_game();
        give(&mut s, 0, race, power);
        for &r in regions {
            place(&mut s, r, race, 1);
        }
        assert_eq!(s.turn_coins(0), coins, "{race:?} {power:?} {regions:?}");
    }
}

#[test]
fn the_catapult_fires_over_a_region() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ratmen, Power::Catapult);
    place(&mut s, 7, Race::Ratmen, 2);
    place(&mut s, 8, Race::Ratmen, 1);
    // Mountain 4, two away from 7 and from 8: 3 - 1. Farther, out of range.
    let range = s.moves(0).costs(RegionMove::Catapult);
    assert!(range.contains(&(4, 2)) && !range.iter().any(|&(r, _)| r == 12));
    s.apply(0, Action::Catapult { region: 4 }).unwrap();
    // It fired from the region with fewer tokens, which is now immune.
    assert!(s.regions[8].catapult && !s.regions[7].catapult);
    assert_eq!(s.apply(0, Action::Catapult { region: 10 }), Err(Error::PowerUsed));
    give(&mut s, 1, Race::Wizards, Power::Stout);
    place(&mut s, 6, Race::Wizards, 1);
    s.current = 1;
    assert_eq!(s.conquest_cost(1, 8), Err(Error::Immune));
    assert!(s.conquest_cost(1, 7).is_ok());
}

#[test]
fn mercenaries_cost_a_coin_and_two_tokens_less() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ratmen, Power::Mercenary);
    place(&mut s, 5, Race::Ratmen, 1);
    assert!(s.moves(0).costs(RegionMove::Hire).contains(&(4, 1)));
    let coins = s.players[0].coins;
    s.apply(0, Action::Hire { region: 4 }).unwrap();
    assert_eq!((s.players[0].coins, tokens(&s, 4)), (coins - 1, 1));
    s.players[0].coins = 0;
    assert_eq!(s.apply(0, Action::Hire { region: 6 }), Err(Error::NotEnoughCoins));
}

// ----- Royal Bonus ---------------------------------------------------------------------------

#[test]
fn fauns_and_their_victims_both_gain_tokens() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 6, Race::Humans, 2);
    place(&mut s, 13, Race::Humans, 1);
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Fauns, Power::Stout);
    place(&mut s, 7, Race::Fauns, 1);
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    let before = hand(&s, 0);
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(hand(&s, 0), before + 1);
    // 2 Humans, 1 lost, 1 kept, 1 more from the Fauns.
    assert_eq!(hand(&s, 1), 1 + 1);
}

#[test]
fn igors_collect_the_fallen() {
    let mut s = packs_game();
    s.regions[6].lost_tribe = true;
    s.regions[5].lost_tribe = true;
    give(&mut s, 1, Race::Igors, Power::Stout);
    place(&mut s, 13, Race::Igors, 1);
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Ratmen, Power::Stout);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::Conquer { region: 5 }).unwrap();
    assert_eq!(s.players[1].active.unwrap().collected, 2);
    end_turn(&mut s, 0);
    // Two players: two collected, one new Igor.
    assert_eq!(s.current, 1);
    let a = s.players[1].active.unwrap();
    assert_eq!((a.collected, a.hand), (0, 1));
}

#[test]
fn behemoths_are_worth_the_swamps() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ratmen, Power::Behemoth);
    place(&mut s, 5, Race::Ratmen, 1);
    place(&mut s, 20, Race::Ratmen, 1);
    place(&mut s, 7, Race::Ratmen, 1);
    // Two Swamps: a Behemoth is worth 2, Mountain 4 costs 3 - 2.
    assert!(s.moves(0).costs(RegionMove::Behemoth).contains(&(4, 1)));
    s.apply(0, Action::Behemoth { region: 4 }).unwrap();
    assert_eq!((s.regions[4].behemoths, s.players[0].active.unwrap().behemoths), (1, 1));
    // It defends too: 2 + 1 Ratman + 1 Mountain + 2.
    give(&mut s, 1, Race::Wizards, Power::Stout);
    place(&mut s, 6, Race::Wizards, 1);
    s.current = 1;
    assert_eq!(s.conquest_cost(1, 4), Ok(2 + 1 + 1 + 2));
}

#[test]
fn fireballs_come_from_magic_sources_and_burn_through_defenses() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ratmen, Power::Fireball);
    // Magic sources 2 and 16.
    for region in [2, 12, 13, 16] {
        s.apply(0, Action::Conquer { region }).unwrap();
    }
    assert!(s.moves(0).list(RegionMove::Fireball).is_empty(), "none yet");
    end_turn(&mut s, 0);
    assert_eq!(s.players[0].active.unwrap().fireballs, 2);
    pass(&mut s, 1);
    // Mountain 1 costs 3: one Fireball, one token.
    assert!(s.moves(0).costs(RegionMove::Fireball).contains(&(1, 1)));
    s.apply(0, Action::Fireball { region: 1 }).unwrap();
    assert_eq!(s.players[0].active.unwrap().fireballs, 1);
}

// ----- A Spider's Web -------------------------------------------------------------------------

#[test]
fn ice_witches_spread_winter() {
    let mut s = packs_game();
    give(&mut s, 0, Race::IceWitches, Power::Stout);
    s.apply(0, Action::Conquer { region: 8 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(s.players[0].active.unwrap().winter, 1);
    // Next to them, in someone else's region.
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 6, Race::Humans, 1);
    s.current = 0;
    s.phase = Phase::Redeploy;
    s.apply(0, Action::Winter { region: 6 }).unwrap();
    assert!(s.regions[6].winter);
    assert_eq!(s.players[0].active.unwrap().winter, 0);
    // The Humans' Farmland: 1 + 1, less 1 for the Winter.
    assert_eq!(s.turn_coins(1), 1);
    // And it is harder to take: 2 + 1 Human + 1.
    s.phase = Phase::Conquest { ghouls: false };
    assert_eq!(s.conquest_cost(0, 6), Ok(4));
}

#[test]
fn skags_hide_loot_that_bites() {
    let mut found = (false, false);
    for seed in 0..30 {
        let mut s = no_tribes(State::with_options(2, seed, Options { sky: None, ..Options::all() }));
        give(&mut s, 1, Race::Skags, Power::Stout);
        s.current = 1;
        s.apply(1, Action::Conquer { region: 7 }).unwrap();
        let loot = s.loot[7].expect("loot under the conquest");
        assert!(s.regions[7].loot);
        assert_eq!(s.view_for(1).loot, vec![(7, loot)]);
        assert!(s.view_for(0).loot.is_empty(), "the others only see the marker");
        give(&mut s, 0, Race::Ratmen, Power::Stout);
        place(&mut s, 6, Race::Ratmen, 1);
        s.current = 0;
        s.phase = Phase::Conquest { ghouls: false };
        let coins = s.players[0].coins;
        let h = hand(&s, 0);
        s.apply(0, Action::Conquer { region: 7 }).unwrap();
        match loot {
            Loot::SkagAttack => {
                found.0 = true;
                assert_eq!(s.regions[7].race(), Some(Race::Skags));
                assert_eq!(hand(&s, 0), h - 1);
                assert_eq!(s.conquest_cost(0, 7), Err(Error::Repelled));
            }
            Loot::Coins(n) => {
                found.1 = true;
                assert_eq!(s.regions[7].race(), Some(Race::Ratmen));
                assert_eq!(s.players[0].coins, coins + n as u32);
            }
        }
    }
    assert!(found.0 && found.1);
}

#[test]
fn slingmen_shoot_over_a_region() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Slingmen, Power::Stout);
    place(&mut s, 7, Race::Slingmen, 1);
    assert_eq!(s.conquest_cost(0, 12), Err(Error::OutOfReach), "three away");
    assert_eq!(s.conquest_cost(0, 4), Ok(3), "two away, over 5 or 6");
    let coins = s.players[0].coins;
    s.apply(0, Action::Conquer { region: 4 }).unwrap();
    assert_eq!(s.players[0].coins, coins + 1);
    assert!(s.log.iter().any(|e| matches!(e, Event::Conquered { region: 4, how: How::Sling, .. })));
}

#[test]
fn copycats_borrow_a_power_from_the_column() {
    let mut s = packs_game();
    s.offers[0].power = Power::Commando;
    give(&mut s, 0, Race::Ratmen, Power::Copycat);
    place(&mut s, 5, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(0, 4), Ok(3));
    assert!(!s.moves(0).copy.is_empty());
    s.apply(0, Action::Copy { offer: Some(0) }).unwrap();
    assert_eq!(s.conquest_cost(0, 4), Ok(2));
    assert_eq!(s.apply(0, Action::Copy { offer: Some(1) }), Err(Error::PowerUsed));
    // Whoever picks the power takes it back.
    end_turn(&mut s, 0);
    s.apply(1, Action::Pick { offer: 0 }).unwrap();
    assert_eq!(s.players[0].active.unwrap().copied, None);
}

#[test]
fn lava_drives_everyone_out_until_the_next_turn() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 6, Race::Humans, 3);
    place(&mut s, 13, Race::Humans, 1);
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Ratmen, Power::Lava);
    place(&mut s, 4, Race::Ratmen, 2);
    s.apply(0, Action::EndConquests).unwrap();
    assert!(s.moves(0).can(RegionMove::Lava, 6));
    s.apply(0, Action::Lava { region: 6 }).unwrap();
    assert_eq!(s.regions[6].lava, Some(0));
    assert_eq!(hand(&s, 1), 3, "no token lost");
    assert_eq!(s.apply(0, Action::Lava { region: 10 }), Err(Error::NoneLeft), "one Mountain, one Lava");
    end_turn(&mut s, 0);
    while s.phase == Phase::Reinforce {
        s.apply(1, Action::Put { region: 13 }).unwrap();
    }
    assert_eq!(s.conquest_cost(1, 6), Err(Error::Immune));
    end_turn(&mut s, 1);
    // Seat 0's turn again: the lava has cooled.
    assert_eq!(s.regions[6].lava, None);
}

#[test]
fn soul_touch_brings_the_old_race_back() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Humans, Power::Merchant);
    place(&mut s, 6, Race::Humans, 2);
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    s.apply(0, Action::Decline).unwrap();
    give(&mut s, 0, Race::Ratmen, Power::SoulTouch);
    place(&mut s, 13, Race::Ratmen, 3);
    s.current = 0;
    s.phase = Phase::Conquest { ghouls: false };
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    s.apply(0, Action::Decline).unwrap();
    let p = &s.players[0];
    let back = p.active.expect("the Humans are back");
    assert_eq!((back.race, back.power), (Race::Humans, Power::Merchant));
    assert_eq!(back.hand, 5 + 2 - 1);
    assert_eq!(p.declined.iter().map(|a| a.race).collect::<Vec<_>>(), [Race::Ratmen]);
    assert!(!s.regions[6].tokens.unwrap().declined);
    assert!(s.log.contains(&Event::Revived { seat: 0, race: Race::Humans }));
}

// ----- Sky Islands' races and powers -----------------------------------------------------------

#[test]
fn escargots_are_paid_at_the_start_of_their_turn() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Escargots, Power::Merchant);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    assert_eq!(s.turn_coins(0), 2, "only Merchant's coins at the end");
    end_turn(&mut s, 0);
    let coins = s.players[0].coins;
    pass(&mut s, 1);
    assert_eq!(s.players[0].coins, coins + 2);
}

#[test]
fn scavengers_keep_their_captives() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 6, Race::Humans, 1);
    s.current = 1;
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    s.apply(1, Action::Decline).unwrap();
    give(&mut s, 0, Race::Scavengers, Power::Stout);
    place(&mut s, 7, Race::Scavengers, 1);
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    assert_eq!(s.regions[6].captive, Some(Race::Humans));
    assert_eq!(s.turn_coins(1), 1, "the Humans still score there");
    give(&mut s, 1, Race::Ratmen, Power::Stout);
    place(&mut s, 5, Race::Ratmen, 1);
    s.current = 1;
    assert_eq!(s.conquest_cost(1, 6), Ok(2 + 3 + 1), "three Scavengers and their captive");
}

#[test]
fn storm_giants_drakons_wendigos_and_zeppelins() {
    let mut s = packs_game();
    give(&mut s, 0, Race::StormGiants, Power::Stout);
    place(&mut s, 3, Race::StormGiants, 1);
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 4, Race::Humans, 5);
    s.current = 0;
    s.apply(0, Action::Lightning { region: 4 }).unwrap();
    assert_eq!(tokens(&s, 4), 1);
    assert_eq!(s.this_turn.bolts, 1);

    let mut s = packs_game();
    give(&mut s, 0, Race::Drakons, Power::Stout);
    place(&mut s, 7, Race::Drakons, 1);
    let before = hand(&s, 0);
    s.apply(0, Action::Dragon { region: 6 }).unwrap();
    assert_eq!(hand(&s, 0), before - 2);
    assert!(s.regions[6].dragon);
    s.apply(0, Action::Dragon { region: 5 }).unwrap();
    s.apply(0, Action::Dragon { region: 8 }).unwrap();
    assert_eq!(s.apply(0, Action::Dragon { region: 10 }), Err(Error::PowerUsed));

    let mut s = packs_game();
    give(&mut s, 1, Race::Humans, Power::Stout);
    place(&mut s, 14, Race::Humans, 3);
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Wendigos, Power::Stout);
    s.apply(0, Action::Wendigo { region: 14 }).unwrap();
    assert!(s.regions[14].tokens.is_none());
    assert_eq!((hand(&s, 1), hand(&s, 0)), (2, 6 + 4 + 1));
    assert_eq!(s.apply(0, Action::Wendigo { region: 3 }), Err(Error::PowerUsed));

    let (mut crashed, mut flew) = (false, false);
    for seed in 0..20 {
        let mut s = no_tribes(State::with_options(2, seed, Options { sky: None, ..Options::all() }));
        give(&mut s, 0, Race::Ratmen, Power::Zeppelined);
        s.apply(0, Action::Zeppelin { region: 15 }).unwrap();
        let Some(&Event::Zeppelin { die, .. }) = s.log.iter().rev().find(|e| matches!(e, Event::Zeppelin { .. })) else {
            panic!("no zeppelin")
        };
        if die == 0 {
            crashed = true;
            assert_eq!(s.regions[15].burning, Some(0));
            assert_eq!(hand(&s, 0), 8 + 5 - 1);
        } else {
            flew = true;
            assert_eq!(tokens(&s, 15), 1, "2 - the die, at least 1");
        }
    }
    assert!(crashed && flew);
}

#[test]
fn airborne_is_cheap_on_the_first_turn_only() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ratmen, Power::Airborne);
    place(&mut s, 5, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(0, 4), Ok(1));
    s.players[0].active.as_mut().unwrap().turns = 1;
    assert_eq!(s.conquest_cost(0, 4), Ok(3));
}

#[test]
fn trade_pacts_pay_whoever_kept_their_word() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ratmen, Power::Haggling);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Pact { seat: 1, give: true }).unwrap();
    s.apply(0, Action::Pact { seat: 1, give: true }).unwrap();
    assert_eq!(s.view_for(1).players[1].pacts, 2);
    end_turn(&mut s, 0);
    // Seat 1 attacks nobody: the haggler pays two.
    pass(&mut s, 1);
    assert_eq!(s.pacts[1], 0);
    assert!(s.log.contains(&Event::Paid { from: Some(0), to: 1, coins: 2, why: Why::Pact }));
    // Next time, seat 1 takes one of the haggler's regions: it pays.
    s.apply(0, Action::Pact { seat: 1, give: true }).unwrap();
    end_turn(&mut s, 0);
    let race = s.players[1].active.unwrap().race;
    s.players[1].active = None;
    give(&mut s, 1, race, Power::Stout);
    s.players[1].active.as_mut().unwrap().hand = 40;
    place(&mut s, 6, race, 1);
    s.apply(1, Action::Conquer { region: 7 }).unwrap();
    end_turn(&mut s, 1);
    assert!(s.log.contains(&Event::Paid { from: Some(1), to: 0, coins: 1, why: Why::Pact }));
}

#[test]
fn gunners_place_cannons_that_fire() {
    let mut s = packs_game();
    give(&mut s, 0, Race::Ratmen, Power::Gunner);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    while hand(&s, 0) > 0 {
        s.apply(0, Action::Put { region: 7 }).unwrap();
    }
    assert_eq!(s.moves(0).end_turn.as_deref(), Some(Error::PlaceCannons.message()));
    s.apply(0, Action::Cannon { region: 7 }).unwrap();
    s.apply(0, Action::Cannon { region: 6 }).unwrap();
    s.apply(0, Action::EndTurn).unwrap();
    pass(&mut s, 1);
    // Cannon in 6 fires: Mountain 4 next to it costs 3 - 2.
    s.apply(0, Action::Fire { region: 6 }).unwrap();
    assert_eq!(s.conquest_cost(0, 4), Ok(1));
}

#[test]
fn racketeers_profit_from_the_column() {
    let mut s = packs_game();
    give(&mut s, 1, Race::Ratmen, Power::Racketeering);
    s.current = 0;
    s.phase = Phase::Pick;
    let coins = s.players[1].coins;
    s.apply(0, Action::Pick { offer: 3 }).unwrap();
    assert_eq!(s.players[1].coins, coins + 3);
    // Going into decline makes the next combo free.
    s.current = 1;
    place(&mut s, 7, Race::Ratmen, 1);
    s.phase = Phase::Conquest { ghouls: false };
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    s.apply(1, Action::Decline).unwrap();
    assert!(s.players[1].free_pick);
    assert_eq!(s.pick_cost(1, 5), 0);
}

#[test]
fn whole_games_with_every_expansion() {
    for players in 3..=6 {
        for seed in 0..6 {
            let s = play_out(players, seed, Options::all());
            assert_eq!(s.phase, Phase::Over);
            assert!(!s.winners().is_empty());
        }
    }
    for players in 2..=5 {
        for seed in 0..6 {
            let options = Options { sky: None, ..Options::all() };
            assert_eq!(play_out(players, 100 + seed, options).phase, Phase::Over);
        }
    }
    let s = play_out(4, 3, Options { sky: Some(Sky::Two), ..Options::default() });
    assert_eq!(s.board.ground, 3);
}
