//! Rules tests. Most use the two-player board, whose regions are:
//!
//! ```text
//!  0 Sea (edge)            8 Forest, magic, tribe   16 Swamp, magic, tribe
//!  1 Mountain, mine, cav.  9 Mountain, mine         17 Hill, cavern
//!  2 Farmland, magic      10 Hill, cavern, tribe    18 Swamp, mine, tribe
//!  3 Forest, mine         11 Lake                   19 Mountain
//!  4 Mountain             12 Hill, tribe            20 Swamp
//!  5 Swamp, cavern, tribe 13 Farmland, tribe        21 Forest (on the sea)
//!  6 Farmland             14 Forest, tribe          22 Sea (edge)
//!  7 Hill                 15 Farmland, magic, tribe
//! ```
//!
//! Regions 4, 6, 10, 12, 14 and 15 are inland: not on the edge, nor on a
//! sea that reaches it.

use crate::*;

mod expansions;

/// Seat `seat` plays `race` with `power`, holding all of its tokens, at the
/// start of its conquests. The combo is taken out of the column and stacks.
pub(crate) fn give(s: &mut State, seat: usize, race: Race, power: Power) {
    s.offers.retain(|o| o.race != race && o.power != power);
    s.races.retain(|&r| r != race);
    s.powers.retain(|&p| p != power);
    let mut army = Army::new(race, power, race.tokens() + power.tokens());
    if power == Power::Behemoth {
        army.behemoths = BEHEMOTHS;
    }
    s.players[seat].active = Some(army);
    s.current = seat;
    s.phase = Phase::Conquest { ghouls: false };
    s.this_turn = Turn::default();
}

/// Puts `count` tokens of `race` in `region`, as they would stand there.
pub(crate) fn place(s: &mut State, region: usize, race: Race, count: u8) {
    let declined = s.owner(race).is_some_and(|(_, d)| d);
    s.regions[region] = RegionState { tokens: Some(Tokens { race, count, declined }), ..RegionState::default() };
}

pub(crate) fn hand(s: &State, seat: usize) -> u8 {
    s.players[seat].active.map_or(0, |a| a.hand)
}

pub(crate) fn tokens(s: &State, region: usize) -> u8 {
    s.regions[region].tokens.map_or(0, |t| t.count)
}

/// A two-player game where nobody has played yet, with no Lost Tribes.
pub(crate) fn empty_game() -> State {
    no_tribes(State::new(2, 1))
}

/// The same game without its Lost Tribes.
pub(crate) fn no_tribes(mut s: State) -> State {
    for r in &mut s.regions {
        r.lost_tribe = false;
    }
    s
}

#[test]
fn a_new_game_sets_up_the_board_and_the_column() {
    for players in 2..=5 {
        let s = State::new(players, 9);
        let map = Map::for_players(players);
        assert_eq!(s.regions.len(), map.regions.len());
        assert_eq!((s.board.ground, s.board.sky), (players, None));
        assert_eq!(s.offers.len(), OFFERS);
        assert_eq!(s.races.len(), 14 - OFFERS);
        assert_eq!(s.powers.len(), 20 - OFFERS);
        assert!(s.races.iter().chain(s.offers.iter().map(|o| &o.race)).all(|r| r.pack() == Pack::Base));
        assert_eq!(s.turns(), Map::for_players(players).turns);
        assert!(s.players.iter().all(|p| p.coins == START_COINS && p.active.is_none()));
        assert_eq!((s.turn, s.current, s.phase), (1, 0, Phase::Pick));
        for (r, def) in s.regions.iter().zip(map.regions) {
            assert_eq!(r.lost_tribe, def.lost_tribe());
            assert!(r.tokens.is_none());
        }
        // The box holds 18 Lost Tribes and 9 Mountains.
        assert!(map.regions.iter().filter(|r| r.lost_tribe()).count() <= 18);
        assert!(map.regions.iter().filter(|r| r.mountain()).count() <= 9);
    }
    assert_eq!([2, 3, 4, 5].map(|n| Map::for_players(n).turns), [10, 10, 9, 8]);
}

#[test]
fn a_combo_can_be_put_on_top_of_the_column_to_try_it_out() {
    for options in [Options::default(), Options::all()] {
        for seed in 0..20 {
            let mut s = State::with_options(options.min_players(), seed, options);
            assert!(s.put_on_top(Some(Race::Ghouls), Some(Power::Spirit)));
            assert_eq!((s.offers[0].race, s.offers[0].power), (Race::Ghouls, Power::Spirit));
            assert_eq!(s.offers.len(), OFFERS);
            // What was on top went where they came from: every badge is
            // still somewhere, once (`check` does the banners).
            check(&s);
            let mut powers: Vec<Power> = s.powers.iter().copied().chain(s.offers.iter().map(|o| o.power)).collect();
            powers.sort();
            assert_eq!(powers, options.powers());
            s.apply(0, Action::Pick { offer: 0 }).unwrap();
            let army = s.players[0].active.unwrap();
            assert_eq!((army.race, army.power, army.hand), (Race::Ghouls, Power::Spirit, 5 + 5));
        }
    }
    // Only the power: the race on top stays.
    let mut s = State::new(2, 1);
    let race = s.offers[0].race;
    assert!(s.put_on_top(None, Some(Power::Spirit)));
    assert_eq!((s.offers[0].race, s.offers[0].power), (race, Power::Spirit));
    // Not in this game (Cursed! is off): nothing changes.
    let mut s = State::new(2, 1);
    let before = s.clone();
    assert!(!s.put_on_top(Some(Race::Goblins), Some(Power::Spirit)));
    assert_eq!(s, before);
}

#[test]
fn the_boards_are_consistent() {
    for map in &MAPS {
        let n = map.regions.len();
        let board = Board::new(map.players, None, &[]);
        for (i, r) in map.regions.iter().enumerate() {
            assert!(!r.adjacent.is_empty(), "{} players: region {i} borders nothing", map.players);
            for &j in r.adjacent {
                assert!((j as usize) < n && j as usize != i);
                assert!(board.adjacent(j as usize, i), "{} players: {i} and {j} aren't mutual", map.players);
            }
            assert!(!(r.terrain.is_water() && r.lost_tribe()));
        }
        // Every board has its seas and lake, and the same number of mines,
        // magic sources and caverns.
        assert_eq!(map.regions.iter().filter(|r| r.terrain.is_water()).count(), 3);
        let mines = map.regions.iter().filter(|r| r.mine()).count();
        assert_eq!(map.regions.iter().filter(|r| r.magic()).count(), mines);
        assert_eq!(map.regions.iter().filter(|r| r.cavern()).count(), mines);
        // Everything is reachable from everything over land and water.
        let mut seen = vec![false; n];
        let mut todo = vec![0];
        while let Some(i) = todo.pop() {
            if !std::mem::replace(&mut seen[i], true) {
                todo.extend(map.regions[i].adjacent.iter().map(|&j| j as usize));
            }
        }
        assert!(seen.iter().all(|&s| s), "{} players: the board is in pieces", map.players);
    }
    assert_eq!(MAPS.map(|m| m.regions.len()), [23, 30, 40, 47]);
}

#[test]
fn picking_pays_a_coin_to_each_combo_above_and_pockets_the_coins_on_it() {
    let mut s = State::new(2, 4);
    let third = s.offers[2];
    let below = s.offers[3];
    s.apply(0, Action::Pick { offer: 2 }).unwrap();
    assert_eq!(s.players[0].coins, START_COINS - 2);
    let army = s.players[0].active.unwrap();
    assert_eq!((army.race, army.power), (third.race, third.power));
    assert_eq!(army.hand, third.race.tokens() + third.power.tokens());
    // The two above got a coin each; the column slid up and was refilled.
    assert_eq!(s.offers.len(), OFFERS);
    assert_eq!((s.offers[0].coins, s.offers[1].coins), (1, 1));
    assert_eq!(s.offers[2], below);
    assert_eq!(s.phase, Phase::Conquest { ghouls: false });

    // The next player picks the top one and takes its coin, for free.
    s.current = 1;
    s.phase = Phase::Pick;
    s.apply(1, Action::Pick { offer: 0 }).unwrap();
    assert_eq!(s.players[1].coins, START_COINS + 1);

    // Too far down for your coins.
    let mut s = State::new(2, 4);
    s.players[0].coins = 3;
    assert_eq!(s.apply(0, Action::Pick { offer: 4 }), Err(Error::NotEnoughCoins));
    assert_eq!(s.apply(0, Action::Pick { offer: 9 }), Err(Error::NoSuchOffer));
    assert_eq!(s.apply(1, Action::Pick { offer: 0 }), Err(Error::NotYourTurn));
    assert_eq!(s.moves(0).pick, vec![(0, 0), (1, 1), (2, 2), (3, 3)]);
}

#[test]
fn a_race_enters_through_the_border_then_conquers_next_to_its_regions() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Merchant);
    // Inland regions are out of reach at first, and the sea is no place to land.
    assert_eq!(s.apply(0, Action::Conquer { region: 6 }), Err(Error::NotABorder));
    assert_eq!(s.apply(0, Action::Conquer { region: 0 }), Err(Error::Water));
    // Region 21 isn't on the edge but on a sea that is.
    assert!(s.board.border(21) && !s.board.border(12));
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    assert_eq!(tokens(&s, 7), 2);
    assert_eq!(hand(&s, 0), 10 - 2);
    // Now next to region 7 only.
    assert_eq!(s.apply(0, Action::Conquer { region: 13 }), Err(Error::OutOfReach));
    assert_eq!(s.apply(0, Action::Conquer { region: 7 }), Err(Error::AlreadyYours));
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    // A mountain costs one more.
    assert_eq!(s.conquest_cost(0, 4), Ok(3));
    assert_eq!(s.moves(0).conquer().iter().find(|(r, _)| *r == 4), Some(&(4, 3)));
}

#[test]
fn every_region_that_could_be_attacked_is_priced_for_planning() {
    let mut s = empty_game();
    give(&mut s, 1, Race::Wizards, Power::Alchemist);
    place(&mut s, 19, Race::Wizards, 3);
    place(&mut s, 20, Race::Wizards, 1);
    s.regions[20].hero = true;
    give(&mut s, 0, Race::Ratmen, Power::Merchant);
    place(&mut s, 7, Race::Ratmen, 1);
    let m = s.moves(0);
    let price = |r: usize| m.prices.iter().find(|p| p.0 == r).map(|p| p.1);
    // Next door, as the conquests say, and across the board too.
    assert_eq!(price(6), Some(2));
    assert_eq!(m.conquer().iter().find(|c| c.0 == 6), Some(&(6, 2)));
    assert_eq!(price(18), Some(2));
    assert!(!m.conquer().iter().any(|c| c.0 == 18), "out of reach");
    assert_eq!(price(4), Some(3), "a mountain");
    assert_eq!(price(19), Some(2 + 1 + 3), "a mountain with three Wizards");
    // Not ours, not the water, not an immune region.
    assert_eq!((price(7), price(0), price(11), price(20)), (None, None, None, None));
    // More than in hand is still priced.
    s.players[0].active.as_mut().unwrap().hand = 1;
    assert_eq!(s.moves(0).prices.iter().find(|p| p.0 == 19), Some(&(19, 6)));
    // Only while conquering.
    s.phase = Phase::Redeploy;
    assert!(s.moves(0).prices.is_empty());
    assert!(s.moves(1).prices.is_empty(), "not their turn");
}

#[test]
fn defenses_and_defenders_add_to_the_cost() {
    let mut s = empty_game();
    give(&mut s, 1, Race::Wizards, Power::Alchemist);
    place(&mut s, 3, Race::Wizards, 1);
    give(&mut s, 0, Race::Ratmen, Power::Merchant);
    place(&mut s, 2, Race::Ratmen, 1);
    s.regions[12].lost_tribe = true;
    assert_eq!(s.conquest_cost(0, 12), Ok(3));
    // Three Wizards: 2 + 3.
    place(&mut s, 3, Race::Wizards, 3);
    assert_eq!(s.conquest_cost(0, 3), Ok(5));
    s.regions[3].fortress = true;
    s.regions[3].encampments = 2;
    assert_eq!(s.conquest_cost(0, 3), Ok(8));
    // A mountain with a lair and two Trolls... from the next region.
    place(&mut s, 1, Race::Wizards, 2);
    s.regions[1].lair = true;
    assert_eq!(s.conquest_cost(0, 1), Ok(2 + 1 + 1 + 2));
}

#[test]
fn powers_and_races_take_tokens_off_the_cost_but_never_below_one() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Commando);
    place(&mut s, 7, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(0, 6), Ok(1));
    assert_eq!(s.conquest_cost(0, 5), Ok(1));

    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Mounted);
    place(&mut s, 7, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(0, 6), Ok(1), "farmland");
    assert_eq!(s.conquest_cost(0, 8), Ok(2), "forest");

    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Underworld);
    place(&mut s, 7, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(0, 5), Ok(1), "a cavern next door");
    assert_eq!(s.conquest_cost(0, 10), Err(Error::OutOfReach));
    // Holding a cavern, every cavern is next door.
    place(&mut s, 5, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(0, 17), Ok(1));
    assert_eq!(s.conquest_cost(0, 1), Ok(2), "a cavern on a mountain");

    let mut s = empty_game();
    give(&mut s, 0, Race::Tritons, Power::Merchant);
    place(&mut s, 2, Race::Tritons, 1);
    assert_eq!(s.conquest_cost(0, 3), Ok(1), "on the lake");
    assert_eq!(s.conquest_cost(0, 1), Ok(2), "a mountain on the sea");

    let mut s = empty_game();
    give(&mut s, 0, Race::Giants, Power::Merchant);
    place(&mut s, 7, Race::Giants, 1);
    assert_eq!(s.conquest_cost(0, 6), Ok(2));
    place(&mut s, 4, Race::Giants, 1);
    assert_eq!(s.conquest_cost(0, 6), Ok(1), "next to their mountain");
    assert_eq!(s.conquest_cost(0, 15), Ok(1));
}

#[test]
fn defenders_lose_a_token_and_put_the_others_back_after_the_turn() {
    let mut s = empty_game();
    give(&mut s, 1, Race::Wizards, Power::Alchemist);
    place(&mut s, 6, Race::Wizards, 3);
    place(&mut s, 15, Race::Wizards, 1);
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Ratmen, Power::Merchant);
    place(&mut s, 7, Race::Ratmen, 1);
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    assert_eq!(hand(&s, 0), 10 - 5);
    assert_eq!(tokens(&s, 6), 5);
    // One Wizard is gone for good, two are in hand.
    assert_eq!(hand(&s, 1), 2);
    assert_eq!(s.this_turn.attacked, vec![1]);
    assert!(matches!(
        s.log.last(),
        Some(Event::Conquered { seat: 0, region: 6, foe: Some(Foe::Race { seat: 1, race: Race::Wizards }), .. })
    ));
    s.apply(0, Action::EndConquests).unwrap();
    while hand(&s, 0) > 0 {
        s.apply(0, Action::Put { region: 7 }).unwrap();
    }
    s.apply(0, Action::EndTurn).unwrap();
    // Now the Wizards put their two back, then it's their turn.
    assert_eq!((s.phase, s.actor()), (Phase::Reinforce, Some(1)));
    assert_eq!(s.moves(1).list(RegionMove::Put), vec![15]);
    assert_eq!(s.apply(1, Action::Put { region: 7 }), Err(Error::NotYours));
    s.apply(1, Action::Put { region: 15 }).unwrap();
    s.apply(1, Action::Put { region: 15 }).unwrap();
    assert_eq!(tokens(&s, 15), 1);
    assert_eq!(s.current, 1);
    assert_eq!(s.phase, Phase::Conquest { ghouls: false });
    // Readying: two lifted from region 15 (three there), one stays.
    assert_eq!(hand(&s, 1), 2);
}

#[test]
fn elves_lose_no_token_and_a_lone_defender_is_simply_gone() {
    let mut s = empty_game();
    give(&mut s, 1, Race::Elves, Power::Alchemist);
    place(&mut s, 6, Race::Elves, 2);
    place(&mut s, 20, Race::Elves, 1);
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Ratmen, Power::Merchant);
    place(&mut s, 7, Race::Ratmen, 1);
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    assert_eq!(hand(&s, 1), 2, "both Elves kept");

    let mut s = empty_game();
    give(&mut s, 1, Race::Humans, Power::Alchemist);
    place(&mut s, 6, Race::Humans, 1);
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Ratmen, Power::Merchant);
    place(&mut s, 7, Race::Ratmen, 1);
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    assert_eq!(hand(&s, 1), 0);
    assert_eq!(s.on_map(Race::Humans), 0);
}

#[test]
fn the_last_conquest_can_be_tried_with_the_die() {
    // Try many seeds: the die decides, the rules stay the same.
    let (mut won, mut lost) = (0, 0);
    for seed in 0..40 {
        let mut s = State::new(2, seed);
        for r in &mut s.regions {
            r.lost_tribe = false;
        }
        give(&mut s, 0, Race::Ratmen, Power::Merchant);
        place(&mut s, 7, Race::Ratmen, 1);
        s.players[0].active.as_mut().unwrap().hand = 1;
        assert_eq!(s.apply(0, Action::Conquer { region: 6 }), Err(Error::NotEnoughTokens));
        assert_eq!(s.moves(0).gamble(), vec![(5, 2), (6, 2), (8, 2)]);
        // A mountain costs 3: 2 missing, still within the die's reach.
        place(&mut s, 3, Race::Ratmen, 1);
        assert!(s.moves(0).gamble().contains(&(4, 3)));
        s.apply(0, Action::Gamble { region: 6 }).unwrap();
        let Some(&Event::Gambled { die, won: w, .. }) = s.log.iter().rev().find(|e| matches!(e, Event::Gambled { .. }))
        else {
            panic!("no die in the log");
        };
        assert!(DIE.contains(&die));
        assert_eq!(w, die >= 1);
        if w {
            won += 1;
            assert_eq!(tokens(&s, 6), 1);
            assert_eq!(hand(&s, 0), 0);
        } else {
            lost += 1;
            assert_eq!(tokens(&s, 6), 0);
            assert_eq!(hand(&s, 0), 1, "the token comes back to redeploy");
        }
        // Either way, the conquests are over.
        assert_eq!(s.phase, Phase::Redeploy);
    }
    assert!(won > 5 && lost > 5, "won {won}, lost {lost}");

    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Merchant);
    place(&mut s, 7, Race::Ratmen, 1);
    s.players[0].active.as_mut().unwrap().hand = 2;
    assert_eq!(s.apply(0, Action::Gamble { region: 6 }), Err(Error::EnoughTokens));
    place(&mut s, 6, Race::Wizards, 1);
    give(&mut s, 1, Race::Wizards, Power::Alchemist);
    place(&mut s, 6, Race::Wizards, 5);
    s.current = 0;
    s.phase = Phase::Conquest { ghouls: false };
    assert_eq!(s.apply(0, Action::Gamble { region: 6 }), Err(Error::TooFar));
}

#[test]
fn following_turns_ready_the_troops_and_regions_can_be_abandoned() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Merchant);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(s.apply(0, Action::EndTurn), Err(Error::TokensInHand));
    s.apply(0, Action::Take { region: 7 }).unwrap();
    assert_eq!(s.apply(0, Action::Take { region: 7 }), Err(Error::LastToken));
    while hand(&s, 0) > 0 {
        s.apply(0, Action::Put { region: 6 }).unwrap();
    }
    s.apply(0, Action::Take { region: 6 }).unwrap();
    s.apply(0, Action::Put { region: 7 }).unwrap();
    assert_eq!((tokens(&s, 7), tokens(&s, 6)), (2, 8));
    s.apply(0, Action::EndTurn).unwrap();
    // Merchant: 2 regions, +2.
    assert_eq!(s.players[0].coins, START_COINS + 4);
    assert!(matches!(s.log.last(), Some(Event::Scored { seat: 0, coins: 4 })));

    // Seat 1 picks and passes.
    assert_eq!((s.current, s.phase), (1, Phase::Pick));
    s.apply(1, Action::Pick { offer: 0 }).unwrap();
    s.apply(1, Action::EndConquests).unwrap();
    let regions_1 = s.regions_of(s.players[1].active.unwrap().race);
    assert!(regions_1.is_empty());
    s.apply(1, Action::EndTurn).unwrap();

    // Back to seat 0, on turn 2: one token stays in each region.
    assert_eq!((s.turn, s.current, s.phase), (2, 0, Phase::Conquest { ghouls: false }));
    assert_eq!((tokens(&s, 7), tokens(&s, 6)), (1, 1));
    assert_eq!(hand(&s, 0), 8);
    s.apply(0, Action::Abandon { region: 7 }).unwrap();
    assert_eq!(hand(&s, 0), 9);
    assert!(s.regions[7].tokens.is_none());
    // Too late to decline now.
    assert_eq!(s.apply(0, Action::Decline), Err(Error::TooLateToDecline));
    s.apply(0, Action::Conquer { region: 5 }).unwrap();
    assert_eq!(s.apply(0, Action::Abandon { region: 5 }), Err(Error::StaysPut));
}

#[test]
fn a_race_wiped_off_the_map_comes_back_as_if_new() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Merchant);
    s.players[0].active.as_mut().unwrap().hand = 3;
    s.this_turn.may_decline = true;
    // Nothing on the map: the border rule applies again.
    assert_eq!(s.apply(0, Action::Conquer { region: 6 }), Err(Error::NotABorder));
    s.apply(0, Action::Conquer { region: 13 }).unwrap();
}

#[test]
fn going_into_decline_flips_one_token_per_region_and_ends_the_turn() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Merchant);
    place(&mut s, 7, Race::Ratmen, 3);
    place(&mut s, 6, Race::Ratmen, 4);
    s.players[0].active.as_mut().unwrap().hand = 2;
    s.this_turn.may_decline = true;
    s.apply(0, Action::Decline).unwrap();
    let p = &s.players[0];
    assert!(p.active.is_none());
    assert_eq!(p.declined.len(), 1);
    assert_eq!(p.declined[0].race, Race::Ratmen);
    assert_eq!((tokens(&s, 7), tokens(&s, 6)), (1, 1));
    assert!(s.regions[7].tokens.unwrap().declined);
    // Scored the two regions, no Merchant bonus: the power is gone.
    assert_eq!(p.coins, START_COINS + 2);
    assert!(s.discarded_powers.contains(&Power::Merchant));
    assert_eq!(s.current, 1);

    // The next decline wipes the older one off the map; its banner goes
    // back under the stack.
    give(&mut s, 0, Race::Wizards, Power::Alchemist);
    place(&mut s, 13, Race::Wizards, 2);
    s.this_turn.may_decline = true;
    s.apply(0, Action::Decline).unwrap();
    assert_eq!(s.players[0].declined.iter().map(|a| a.race).collect::<Vec<_>>(), [Race::Wizards]);
    assert_eq!(s.on_map(Race::Ratmen), 0);
    assert_eq!(s.races.last(), Some(&Race::Ratmen));
    assert!(s.log.contains(&Event::Vanished { seat: 0, race: Race::Ratmen }));
    // Next turn, a new race.
    s.current = 1;
    s.phase = Phase::Redeploy;
    give(&mut s, 1, Race::Humans, Power::Hill);
    s.phase = Phase::Redeploy;
    s.players[1].active.as_mut().unwrap().hand = 0;
    s.apply(1, Action::EndTurn).unwrap();
    assert_eq!((s.current, s.phase), (0, Phase::Pick));
}

#[test]
fn spirit_races_in_decline_stay_beside_another_one() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Spirit);
    place(&mut s, 7, Race::Ratmen, 2);
    s.this_turn.may_decline = true;
    s.apply(0, Action::Decline).unwrap();
    give(&mut s, 0, Race::Wizards, Power::Alchemist);
    place(&mut s, 13, Race::Wizards, 2);
    s.this_turn.may_decline = true;
    s.apply(0, Action::Decline).unwrap();
    assert_eq!(s.players[0].declined.len(), 2);
    // Both score.
    assert_eq!(s.turn_coins(0), 2);
    // A third: the Spirits stay, the Wizards go.
    give(&mut s, 0, Race::Humans, Power::Hill);
    place(&mut s, 21, Race::Humans, 2);
    s.this_turn.may_decline = true;
    s.apply(0, Action::Decline).unwrap();
    let races: Vec<Race> = s.players[0].declined.iter().map(|a| a.race).collect();
    assert_eq!(races, [Race::Ratmen, Race::Humans]);
}

#[test]
fn ghouls_keep_every_token_in_decline_and_go_on_conquering_first() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Ghouls, Power::Merchant);
    place(&mut s, 7, Race::Ghouls, 4);
    place(&mut s, 6, Race::Ghouls, 3);
    // Readied at the start of the turn: they put it all back to decline.
    s.players[0].active.as_mut().unwrap().hand = 0;
    s.this_turn.lifted = vec![];
    s.this_turn.may_decline = true;
    s.apply(0, Action::Decline).unwrap();
    assert_eq!((tokens(&s, 7), tokens(&s, 6)), (4, 3));

    // Seat 1 plays; then seat 0 picks a new race, and the Ghouls go first.
    give(&mut s, 1, Race::Wizards, Power::Alchemist);
    s.phase = Phase::Redeploy;
    s.players[1].active.as_mut().unwrap().hand = 0;
    s.apply(1, Action::EndTurn).unwrap();
    assert_eq!((s.current, s.phase), (0, Phase::Pick));
    s.apply(0, Action::Pick { offer: 0 }).unwrap();
    assert_eq!(s.phase, Phase::Conquest { ghouls: true });
    let ghouls = s.players[0].declined[0];
    assert_eq!(ghouls.hand, 5);
    assert_eq!((tokens(&s, 7), tokens(&s, 6)), (1, 1));
    assert_eq!(s.apply(0, Action::Decline), Err(Error::TooLateToDecline));
    s.apply(0, Action::Conquer { region: 8 }).unwrap();
    assert_eq!(tokens(&s, 8), 2);
    assert!(s.regions[8].tokens.unwrap().declined);
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(s.phase, Phase::Conquest { ghouls: false });
    s.apply(0, Action::EndConquests).unwrap();
    // The Ghouls redeploy too.
    assert_eq!(s.apply(0, Action::EndTurn), Err(Error::TokensInHand));
    assert!(s.moves(0).can(RegionMove::Put, 8));

    // Ghouls that don't conquer put their tokens back where they were.
    let mut s = empty_game();
    give(&mut s, 0, Race::Ghouls, Power::Merchant);
    place(&mut s, 7, Race::Ghouls, 4);
    s.players[0].active.as_mut().unwrap().hand = 0;
    s.this_turn.may_decline = true;
    s.apply(0, Action::Decline).unwrap();
    give(&mut s, 1, Race::Wizards, Power::Alchemist);
    s.phase = Phase::Redeploy;
    s.players[1].active.as_mut().unwrap().hand = 0;
    s.apply(1, Action::EndTurn).unwrap();
    s.apply(0, Action::Pick { offer: 0 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(tokens(&s, 7), 4);
}

#[test]
fn race_bonuses_are_scored_at_the_end_of_the_turn() {
    let cases: [(Race, Power, &[usize], u32); 8] = [
        (Race::Humans, Power::Hill, &[2, 6, 7], 3 + 2 + 1),
        (Race::Wizards, Power::Alchemist, &[2, 8, 7], 3 + 2 + 2),
        (Race::Dwarves, Power::Merchant, &[1, 3, 7], 3 + 2 + 3),
        (Race::Ratmen, Power::Forest, &[3, 8, 21], 3 + 3),
        (Race::Ratmen, Power::Swamp, &[5, 20, 21], 3 + 2),
        (Race::Ratmen, Power::Wealthy, &[5], 1 + 7),
        (Race::Ratmen, Power::Fortified, &[5, 7], 2 + 1),
        (Race::Ratmen, Power::Stout, &[5, 7], 2),
    ];
    for (race, power, regions, coins) in cases {
        let mut s = empty_game();
        give(&mut s, 0, race, power);
        for &r in regions {
            place(&mut s, r, race, 1);
        }
        s.regions[5].fortress = power == Power::Fortified;
        assert_eq!(s.turn_coins(0), coins, "{race:?} {power:?}");
    }
    // Wealthy pays once.
    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Wealthy);
    place(&mut s, 5, Race::Ratmen, 1);
    s.players[0].active.as_mut().unwrap().turns = 1;
    assert_eq!(s.turn_coins(0), 1);
    // Dwarves keep their mines in decline.
    let mut s = empty_game();
    give(&mut s, 0, Race::Dwarves, Power::Merchant);
    place(&mut s, 1, Race::Dwarves, 1);
    place(&mut s, 3, Race::Dwarves, 1);
    s.this_turn.may_decline = true;
    s.apply(0, Action::Decline).unwrap();
    assert_eq!(s.players[0].coins, START_COINS + 4);
}

#[test]
fn orcs_and_pillaging_score_the_occupied_regions_they_took() {
    let mut s = empty_game();
    s.regions[5].lost_tribe = true;
    give(&mut s, 0, Race::Orcs, Power::Pillaging);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Conquer { region: 5 }).unwrap();
    assert_eq!(s.this_turn.plundered, 1);
    // 2 regions, +1 Orcs, +1 Pillaging.
    assert_eq!(s.turn_coins(0), 4);
}

#[test]
fn amazons_set_four_tokens_aside_after_each_turn() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Amazons, Power::Merchant);
    assert_eq!(hand(&s, 0), 12);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    // Everything on the map: four have to stay in hand.
    while hand(&s, 0) > 0 {
        s.apply(0, Action::Put { region: 6 }).unwrap();
    }
    assert_eq!(s.apply(0, Action::EndTurn), Err(Error::RestAmazons));
    for _ in 0..4 {
        s.apply(0, Action::Take { region: 6 }).unwrap();
    }
    s.apply(0, Action::EndTurn).unwrap();
    let a = s.players[0].active.unwrap();
    assert_eq!((a.hand, a.resting), (0, 4));
    assert_eq!(s.on_map(Race::Amazons), 8);
    // They come back when readying.
    s.current = 1;
    give(&mut s, 1, Race::Wizards, Power::Alchemist);
    s.phase = Phase::Redeploy;
    s.players[1].active.as_mut().unwrap().hand = 0;
    s.apply(1, Action::EndTurn).unwrap();
    assert_eq!(hand(&s, 0), 4 + 6);
}

#[test]
fn skeletons_raise_a_token_for_every_two_occupied_regions_taken() {
    let mut s = empty_game();
    for r in [7, 6, 5] {
        s.regions[r].lost_tribe = true;
    }
    give(&mut s, 0, Race::Skeletons, Power::Commando);
    // Three Lost Tribes, 2 tokens each with Commando: 10 - 6.
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::Conquer { region: 5 }).unwrap();
    assert_eq!(hand(&s, 0), 4);
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(hand(&s, 0), 5);
    assert!(s.log.contains(&Event::Raised { seat: 0, race: Race::Skeletons, tokens: 1 }));
}

#[test]
fn sorcerers_replace_a_lone_enemy_token_once_per_opponent() {
    let mut s = empty_game();
    give(&mut s, 1, Race::Humans, Power::Hill);
    place(&mut s, 6, Race::Humans, 1);
    place(&mut s, 8, Race::Humans, 1);
    place(&mut s, 5, Race::Humans, 2);
    s.players[1].active.as_mut().unwrap().hand = 0;
    give(&mut s, 0, Race::Sorcerers, Power::Merchant);
    place(&mut s, 7, Race::Sorcerers, 1);
    assert_eq!(s.moves(0).list(RegionMove::Enchant), vec![6, 8]);
    assert_eq!(s.apply(0, Action::Enchant { region: 5 }), Err(Error::NotAlone));
    s.apply(0, Action::Enchant { region: 6 }).unwrap();
    assert_eq!(s.regions[6].tokens.map(|t| (t.race, t.count)), Some((Race::Sorcerers, 1)));
    assert_eq!(hand(&s, 0), 7, "the token comes from the box");
    assert_eq!(hand(&s, 1), 0, "and the Human is gone");
    assert_eq!(s.apply(0, Action::Enchant { region: 8 }), Err(Error::PowerUsed));
    // An encampment doesn't leave the token alone any more.
    let mut s2 = s.clone();
    s2.this_turn.enchanted.clear();
    s2.regions[8].encampments = 1;
    assert_eq!(s2.apply(0, Action::Enchant { region: 8 }), Err(Error::NotAlone));
}

#[test]
fn dragons_heroes_and_holes_make_regions_immune() {
    let mut s = empty_game();
    give(&mut s, 1, Race::Humans, Power::DragonMaster);
    place(&mut s, 6, Race::Humans, 1);
    place(&mut s, 20, Race::Humans, 1);
    give(&mut s, 0, Race::Ratmen, Power::Merchant);
    place(&mut s, 7, Race::Ratmen, 1);
    s.current = 1;
    s.phase = Phase::Conquest { ghouls: false };
    // The dragon takes the Ratmen's region with a single token.
    place(&mut s, 7, Race::Ratmen, 5);
    s.apply(1, Action::Dragon { region: 7 }).unwrap();
    assert_eq!(tokens(&s, 7), 1);
    assert!(s.regions[7].dragon);
    assert_eq!(s.apply(1, Action::Dragon { region: 8 }), Err(Error::PowerUsed));
    // Nobody can touch it.
    s.current = 0;
    s.phase = Phase::Conquest { ghouls: false };
    place(&mut s, 8, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(0, 7), Err(Error::Immune));

    let mut s = empty_game();
    give(&mut s, 0, Race::Halflings, Power::Merchant);
    // Halflings land anywhere, and dig in their first two regions.
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::Conquer { region: 4 }).unwrap();
    s.apply(0, Action::Conquer { region: 10 }).unwrap();
    assert_eq!([6, 4, 10].map(|r| s.regions[r].hole), [true, true, false]);
    give(&mut s, 1, Race::Ratmen, Power::Merchant);
    place(&mut s, 7, Race::Ratmen, 1);
    assert_eq!(s.conquest_cost(1, 6), Err(Error::Immune));
    // Abandoning a hole loses it for good.
    s.current = 0;
    s.phase = Phase::Conquest { ghouls: false };
    s.this_turn = Turn::default();
    s.apply(0, Action::Abandon { region: 6 }).unwrap();
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    assert!(!s.regions[6].hole);

    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Heroic);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::Conquer { region: 5 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    while hand(&s, 0) > 0 {
        s.apply(0, Action::Put { region: 5 }).unwrap();
    }
    assert_eq!(s.apply(0, Action::EndTurn), Err(Error::PlaceHeroes));
    s.apply(0, Action::Hero { region: 7 }).unwrap();
    s.apply(0, Action::Hero { region: 6 }).unwrap();
    assert_eq!(s.apply(0, Action::Hero { region: 5 }), Err(Error::NoneLeft));
    s.apply(0, Action::Hero { region: 6 }).unwrap();
    s.apply(0, Action::Hero { region: 5 }).unwrap();
    s.apply(0, Action::EndTurn).unwrap();
    give(&mut s, 1, Race::Wizards, Power::Merchant);
    place(&mut s, 8, Race::Wizards, 1);
    assert_eq!(s.conquest_cost(1, 7), Err(Error::Immune));
    assert!(s.conquest_cost(1, 6).is_ok());
}

#[test]
fn trolls_lairs_and_bivouacs_add_to_the_defense() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Trolls, Power::Bivouacking);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    assert!(s.regions[7].lair);
    s.apply(0, Action::EndConquests).unwrap();
    for _ in 0..5 {
        s.apply(0, Action::Camp { region: 7 }).unwrap();
    }
    assert_eq!(s.apply(0, Action::Camp { region: 7 }), Err(Error::NoneLeft));
    s.apply(0, Action::Uncamp { region: 7 }).unwrap();
    while hand(&s, 0) > 0 {
        s.apply(0, Action::Put { region: 7 }).unwrap();
    }
    s.apply(0, Action::EndTurn).unwrap();
    give(&mut s, 1, Race::Ratmen, Power::Merchant);
    place(&mut s, 6, Race::Ratmen, 1);
    // 2 + 10 Trolls + the lair + 4 encampments.
    assert_eq!(s.conquest_cost(1, 7), Ok(2 + 10 + 1 + 4));
    // In decline, the lair stays.
    s.current = 0;
    s.phase = Phase::Conquest { ghouls: false };
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    s.apply(0, Action::Decline).unwrap();
    assert!(s.regions[7].lair);
    assert_eq!(s.regions[7].encampments, 0);
}

#[test]
fn fortresses_are_built_one_a_turn() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Fortified);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    s.apply(0, Action::Fortify { region: 7 }).unwrap();
    assert_eq!(s.apply(0, Action::Fortify { region: 6 }), Err(Error::PowerUsed));
    assert_eq!(s.moves(0).list(RegionMove::Fortify), Vec::<usize>::new());
    assert_eq!(s.turn_coins(0), 2 + 1);
}

#[test]
fn a_diplomat_makes_peace_with_someone_it_did_not_attack() {
    let mut s = empty_game();
    give(&mut s, 1, Race::Humans, Power::Hill);
    place(&mut s, 6, Race::Humans, 1);
    s.players[1].active.as_mut().unwrap().hand = 5;
    give(&mut s, 0, Race::Ratmen, Power::Diplomat);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(s.moves(0).allies, vec![1]);
    s.apply(0, Action::Ally { seat: Some(1) }).unwrap();
    while hand(&s, 0) > 0 {
        s.apply(0, Action::Put { region: 7 }).unwrap();
    }
    s.apply(0, Action::EndTurn).unwrap();
    // The Humans can't attack the Ratmen...
    s.current = 1;
    s.phase = Phase::Conquest { ghouls: false };
    assert_eq!(s.conquest_cost(1, 7), Err(Error::AtPeace));
    // ...until the Ratmen's next turn.
    s.phase = Phase::Redeploy;
    s.players[1].active.as_mut().unwrap().hand = 0;
    s.apply(1, Action::EndTurn).unwrap();
    assert_eq!(s.players[0].active.unwrap().ally, None);

    // No peace with someone you attacked.
    let mut s = empty_game();
    give(&mut s, 1, Race::Humans, Power::Hill);
    place(&mut s, 6, Race::Humans, 1);
    give(&mut s, 0, Race::Ratmen, Power::Diplomat);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::Conquer { region: 6 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(s.apply(0, Action::Ally { seat: Some(1) }), Err(Error::Attacked));
}

#[test]
fn seafaring_races_take_the_seas_flying_ones_land_anywhere() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Seafaring);
    s.apply(0, Action::Conquer { region: 0 }).unwrap();
    s.apply(0, Action::Conquer { region: 2 }).unwrap();
    assert_eq!(s.conquest_cost(0, 3), Ok(2));
    // In decline they keep the sea and score it.
    s.this_turn = Turn { may_decline: true, ..Turn::default() };
    s.apply(0, Action::Decline).unwrap();
    assert!(s.regions[0].tokens.is_some_and(|t| t.declined));

    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Flying);
    s.apply(0, Action::Conquer { region: 15 }).unwrap();
    s.apply(0, Action::Conquer { region: 1 }).unwrap();
    assert_eq!(s.apply(0, Action::Conquer { region: 11 }), Err(Error::Water));
}

#[test]
fn berserk_rolls_before_each_conquest() {
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..30 {
        let mut s = State::new(2, seed);
        for r in &mut s.regions {
            r.lost_tribe = false;
        }
        give(&mut s, 0, Race::Ratmen, Power::Berserk);
        s.apply(0, Action::Conquer { region: 3 }).unwrap();
        assert!(s.moves(0).gamble().is_empty());
        s.apply(0, Action::Roll).unwrap();
        assert_eq!(s.apply(0, Action::Roll), Err(Error::PowerUsed));
        let die = s.this_turn.berserk.unwrap();
        seen.insert(die);
        let cost = s.conquest_cost(0, 4).unwrap();
        assert_eq!(cost, 3u8.saturating_sub(die).max(1));
        s.apply(0, Action::Conquer { region: 4 }).unwrap();
        assert_eq!(tokens(&s, 4), cost);
        assert_eq!(s.this_turn.berserk, None);
    }
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), vec![0, 1, 2, 3]);
}

#[test]
fn stout_races_go_into_decline_after_a_turn_of_conquests() {
    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Stout);
    s.apply(0, Action::Conquer { region: 7 }).unwrap();
    s.apply(0, Action::EndConquests).unwrap();
    while hand(&s, 0) > 0 {
        s.apply(0, Action::Put { region: 7 }).unwrap();
    }
    assert!(s.moves(0).decline);
    s.apply(0, Action::Decline).unwrap();
    assert_eq!(s.players[0].coins, START_COINS + 1);
    assert!(s.players[0].active.is_none());
    assert_eq!(tokens(&s, 7), 1);
    // Others can't.
    let mut s = empty_game();
    give(&mut s, 0, Race::Ratmen, Power::Merchant);
    s.apply(0, Action::EndConquests).unwrap();
    assert_eq!(s.apply(0, Action::Decline), Err(Error::TooLateToDecline));
}

#[test]
fn coins_stay_hidden_until_the_end() {
    let mut s = State::new(3, 2);
    s.players[1].coins = 12;
    let v = s.view_for(0);
    assert_eq!(v.players[0].coins, Some(START_COINS));
    assert_eq!(v.players[1].coins, None);
    assert_eq!(v.board, 3);
    assert_eq!(v.turns, 10);
    assert_eq!(v.map().len(), 30);
    assert_eq!(v.actor, Some(0));
    assert!(!v.moves.pick.is_empty());
    assert!(s.view_for(1).moves.pick.is_empty(), "not their turn");
    s.phase = Phase::Over;
    assert_eq!(s.view_for(0).players[1].coins, Some(12));
    assert_eq!(s.view_for(0).winners, vec![1]);
}

#[test]
fn views_survive_json() {
    let mut s = State::new(4, 3);
    s.apply(0, Action::Pick { offer: 1 }).unwrap();
    let r = s.moves(0).conquer()[0].0;
    s.apply(0, Action::Conquer { region: r }).unwrap();
    let v = s.view_for(2);
    let json = serde_json::to_string(&v).unwrap();
    let back: View = serde_json::from_str(&json).unwrap();
    assert_eq!(back, v);
}

/// A small seeded generator for the bots.
pub(crate) struct Dice(u64);

impl Dice {
    pub(crate) fn new(seed: u64) -> Dice {
        Dice(seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407))
    }

    /// A number below `n` (0 when `n` is 0).
    pub(crate) fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n.max(1)
    }

    fn any<T: Copy>(&mut self, items: &[T]) -> Option<T> {
        (!items.is_empty()).then(|| items[self.below(items.len())])
    }
}

/// What a bot that only sees `moves` does next.
pub(crate) fn bot_move(s: &State, seat: usize, dice: &mut Dice) -> Action {
    let m = s.moves(seat);
    let army = s.players[seat].active;
    let list = |kind: RegionMove| m.list(kind);
    if let Some((offer, _)) = dice.any(&m.pick) {
        return Action::Pick { offer };
    }
    match s.phase {
        Phase::Conquest { .. } => {
            if let Some(offer) = dice.any(&m.copy) {
                if dice.below(3) == 0 {
                    return Action::Copy { offer: Some(offer) };
                }
            }
            if m.decline && army.is_some_and(|a| a.turns >= 2) && dice.below(3) == 0 {
                if let Some(region) = dice.any(&list(RegionMove::Tower)) {
                    return Action::Tower { region };
                }
                return Action::Decline;
            }
            if let Some(seat) = dice.any(&m.give_pact) {
                if dice.below(4) == 0 {
                    return Action::Pact { seat, give: true };
                }
            }
            let specials = [
                RegionMove::Dragon,
                RegionMove::Lightning,
                RegionMove::Zeppelin,
                RegionMove::Enchant,
                RegionMove::Wendigo,
                RegionMove::Catapult,
                RegionMove::Hire,
                RegionMove::Fireball,
                RegionMove::Behemoth,
                RegionMove::Fire,
            ];
            for kind in specials {
                if let Some(region) = dice.any(&list(kind)) {
                    if dice.below(3) == 0 {
                        return kind.action(region);
                    }
                }
            }
            if let Some(region) = dice.any(&list(RegionMove::Conquer)) {
                if dice.below(8) != 0 {
                    return Action::Conquer { region };
                }
            }
            if m.roll && dice.below(2) == 0 {
                return Action::Roll;
            }
            if let Some(region) = dice.any(&list(RegionMove::Gamble)) {
                if dice.below(2) == 0 {
                    return Action::Gamble { region };
                }
            }
            if let Some(region) = dice.any(&list(RegionMove::Abandon)) {
                if dice.below(20) == 0 {
                    return Action::Abandon { region };
                }
            }
            Action::EndConquests
        }
        Phase::Redeploy => {
            let why = m.end_turn.as_deref();
            if why == Some(Error::RestAmazons.message()) {
                let take = army.is_some_and(|a| a.hand < AMAZON_EXTRA);
                return match (take, dice.any(&list(RegionMove::Take)), dice.any(&list(RegionMove::Put))) {
                    (true, Some(region), _) => Action::Take { region },
                    (_, _, Some(region)) => Action::Put { region },
                    (_, Some(region), None) => Action::Take { region },
                    _ => Action::EndTurn,
                };
            }
            if why == Some(Error::PlaceHeroes.message()) {
                let free: Vec<usize> = list(RegionMove::Hero).into_iter().filter(|&r| !s.regions[r].hero).collect();
                return Action::Hero { region: dice.any(&free).expect("a region for a Hero") };
            }
            if why == Some(Error::PlaceCannons.message()) {
                let free: Vec<usize> =
                    list(RegionMove::Cannon).into_iter().filter(|&r| s.regions[r].cannon.is_none()).collect();
                return Action::Cannon { region: dice.any(&free).expect("a region for a Cannon") };
            }
            if !m.can_end_turn {
                return Action::Put { region: dice.any(&list(RegionMove::Put)).expect("somewhere to put tokens") };
            }
            let optional = [
                RegionMove::Camp,
                RegionMove::Fortify,
                RegionMove::Pot,
                RegionMove::Winter,
                RegionMove::Lava,
                RegionMove::Herd,
            ];
            for kind in optional {
                if let Some(region) = dice.any(&list(kind)) {
                    if dice.below(4) == 0 {
                        return kind.action(region);
                    }
                }
            }
            if let Some(ally) = dice.any(&m.allies) {
                if army.is_some_and(|a| a.ally.is_none()) {
                    return Action::Ally { seat: Some(ally) };
                }
            }
            if m.decline && dice.below(4) == 0 {
                return Action::Decline;
            }
            Action::EndTurn
        }
        Phase::Reinforce => Action::Put { region: dice.any(&list(RegionMove::Put)).expect("somewhere to put tokens") },
        Phase::Pick | Phase::Over => Action::EndTurn,
    }
}

/// Plays a whole game with a bot that only sees what `moves` offers,
/// checking after every move that no token is ever made out of nothing.
pub(crate) fn play_out(players: usize, seed: u64, options: Options) -> State {
    let mut s = State::with_options(players, seed, options);
    let mut dice = Dice::new(seed);
    for step in 0..60_000 {
        if s.phase == Phase::Over {
            return s;
        }
        let seat = s.actor().expect("somebody acts");
        let action = bot_move(&s, seat, &mut dice);
        if let Err(e) = s.apply(seat, action) {
            panic!("{players} players, seed {seed}, step {step}: seat {seat} {action:?} refused: {e:?} in {:?}", s.phase);
        }
        check(&s);
    }
    panic!("{players} players, seed {seed}: the game never ended");
}

/// What must always hold.
pub(crate) fn check(s: &State) {
    for race in Race::ALL {
        let owner = s.owner(race);
        let on_map = s.on_map(race);
        if on_map > 0 {
            assert!(owner.is_some(), "{race:?} on the map without an owner");
        }
        let (held, extra) = owner
            .and_then(|(seat, _)| {
                let p = &s.players[seat];
                p.active.iter().chain(&p.declined).find(|a| a.race == race).map(|a| (a.hand + a.resting, a.extra))
            })
            .unwrap_or((0, 0));
        assert!(on_map + held <= race.supply() + extra, "{race:?}: more tokens than the box holds");
    }
    for r in &s.regions {
        if let Some(t) = r.tokens {
            assert!(t.count > 0);
            assert!(!r.lost_tribe);
            assert_eq!(t.declined, s.owner(t.race).unwrap().1);
        }
        assert!(!(r.loot && r.tokens.is_none() && r.lava.is_some()));
    }
    // Every banner is somewhere, once.
    let mut races: Vec<Race> = s.races.clone();
    races.extend(s.offers.iter().map(|o| o.race));
    for p in &s.players {
        races.extend(p.active.iter().chain(&p.declined).map(|a| a.race));
        assert!(p.declined.len() <= 2);
        assert!(p.declined.iter().filter(|a| a.power != Power::Spirit).count() <= 1);
        for a in p.active.iter().chain(&p.declined) {
            let standing: u8 = s.regions_of(a.race).iter().map(|&r| s.regions[r].behemoths).sum();
            assert!(a.behemoths + standing <= BEHEMOTHS);
        }
        if let Some(a) = p.active {
            assert!(s.regions_of(a.race).iter().filter(|&&r| s.regions[r].cannon.is_some()).count() <= CANNONS);
        }
    }
    races.sort();
    assert_eq!(races, s.options.races());
    assert!(s.regions.iter().filter(|r| r.fortress).count() <= FORTRESSES);
    assert!(s.regions.iter().map(|r| r.encampments).sum::<u8>() <= 2 * ENCAMPMENTS);
    assert!(s.pacts.iter().sum::<u8>() <= PACTS);
}

#[test]
fn whole_games_can_be_played_to_the_end_on_every_board() {
    for players in 2..=5 {
        for seed in 0..12 {
            let mut s = play_out(players, seed, Options::default());
            assert_eq!(s.phase, Phase::Over);
            assert_eq!(s.turn, s.turns());
            assert!(!s.winners().is_empty());
            // Every seat scored at least once a turn.
            assert!(s.players.iter().all(|p| p.coins >= s.turns() as u32 / 2));
            assert_eq!(s.log.last(), Some(&Event::GameOver));
            assert_eq!(s.apply(0, Action::EndTurn), Err(Error::GameOver));
        }
    }
}

#[test]
fn the_same_seed_plays_the_same_game() {
    assert_eq!(play_out(3, 7, Options::default()), play_out(3, 7, Options::default()));
    assert_eq!(play_out(4, 7, Options::all()), play_out(4, 7, Options::all()));
}
