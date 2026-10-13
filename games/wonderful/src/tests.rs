use super::*;

// ----- helpers ---------------------------------------------------------

/// The first copy of the design called `name`.
fn id(name: &str) -> CardId {
    let at = catalogue().iter().position(|c| c.name == name).unwrap_or_else(|| panic!("no card {name}"));
    CardId(at as u16)
}

/// Another copy of the design called `name` (it must have more than one).
fn twin(name: &str) -> CardId {
    let first = id(name);
    (0..CARDS as u16)
        .map(CardId)
        .find(|&c| c != first && c.def().name == name)
        .unwrap_or_else(|| panic!("{name} has a single copy"))
}

/// A game in which everybody has drafted their first card each time.
fn to_planning(n: usize, seed: u64) -> State {
    to_planning_with(n, seed, Options::default())
}

fn to_planning_with(n: usize, seed: u64, options: Options) -> State {
    let mut s = State::with_options(n, seed, options);
    for _ in 0..DRAFT_PICKS {
        for seat in 0..n {
            let card = s.players[seat].hand[0];
            s.apply(seat, Action::Draft { card }).unwrap();
        }
    }
    assert_eq!(s.phase, Phase::Planning);
    s
}

/// Planning with nothing drafted, built or finished: a blank slate for
/// setting up a production scenario by hand.
/// Seat `i` plays Empire `i`: Noram States, the Republic of Europe, the
/// Federation of Asia...
fn blank(n: usize) -> State {
    let mut s = to_planning(n, 7);
    for p in &mut s.players {
        p.drafted.clear();
        p.buildings.clear();
        p.empire.clear();
    }
    s.empires = (0..n).collect();
    s
}

fn start_production(s: &mut State) {
    for seat in 0..s.seats() {
        s.apply(seat, Action::Ready).unwrap();
    }
}

/// Everybody gives up what is left of the current production step.
fn skip_step(s: &mut State) {
    for seat in 0..s.seats() {
        if !matches!(s.phase, Phase::Production { .. }) {
            return; // the last "ready" ended the round
        }
        if s.players[seat].choose {
            s.apply(seat, Action::Choose { token: Token::General }).unwrap();
        }
        s.apply(seat, Action::Ready).unwrap();
    }
}

fn skip_to(s: &mut State, step: u8) {
    while s.phase != (Phase::Production { step }) {
        assert!(matches!(s.phase, Phase::Production { .. }), "left production: {:?}", s.phase);
        skip_step(s);
    }
}

/// Everybody passes until the round is over. (The wrap-up step is skipped
/// by the game itself when nobody has anything to place.)
fn finish_round(s: &mut State) {
    let round = s.round;
    while s.round == round && s.phase != Phase::Over {
        skip_step(s);
    }
}

fn all_cards(s: &State) -> Vec<CardId> {
    let mut v = s.deck.clone();
    v.extend(&s.ascension_deck);
    v.extend(&s.discard);
    for p in &s.players {
        v.extend(&p.hand);
        v.extend(p.picked);
        v.extend(&p.drafted);
        v.extend(p.buildings.iter().map(|b| b.card));
        v.extend(&p.empire);
    }
    v
}

/// No card is ever lost or duplicated.
fn check_cards(s: &State) {
    let mut v = all_cards(s);
    let total = DECK_SIZE + if s.options.ascension { ASCENSION_DECK_SIZE } else { 0 };
    assert_eq!(v.len(), total);
    v.sort_by_key(|c| c.0);
    v.dedup();
    assert_eq!(v.len(), total, "a card is in two places");
}

// ----- the catalogue -----------------------------------------------------

#[test]
fn the_catalogue_is_the_base_game() {
    assert_eq!(catalogue().len(), DESIGNS + ASCENSION_DESIGNS);
    let all: Vec<&Card> = catalogue().iter().filter(|c| !c.ascension).collect();
    assert_eq!(all.len(), DESIGNS);
    assert_eq!(DECK_SIZE, 150);
    // Designs and cards of each type, Structures to Discoveries.
    let designs = Kind::ALL.map(|k| all.iter().filter(|c| c.kind == k).count());
    let cards = Kind::ALL.map(|k| all.iter().filter(|c| c.kind == k).map(|c| c.copies as usize).sum::<usize>());
    assert_eq!(designs, [9, 9, 23, 20, 17]);
    assert_eq!(cards, [50, 31, 23, 29, 17]);
    let mut names: Vec<_> = catalogue().iter().map(|c| c.name).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), DESIGNS + ASCENSION_DESIGNS, "names must be unique, in both decks");
    for c in all {
        let total = c.cost.total();
        assert!((2..=8).contains(&total), "{} costs {total}", c.name);
        assert!(c.vp <= 15, "{}", c.name);
        assert!(c.vp == 0 || (c.combo.is_none() && c.per_token.is_none()), "{} scores two ways", c.name);
    }
    // A few cards checked against the printed ones.
    let time_travel = id("Time Travel").def();
    assert_eq!(time_travel.cost, Cost { res: [0, 0, 5, 0, 0], krystallium: 3, ..Cost::default() });
    assert_eq!((time_travel.vp, time_travel.recycle), (15, Res::Exploration));
    let zeppelin = id("Zeppelin").def();
    assert_eq!((zeppelin.kind, zeppelin.cost.res, zeppelin.produces), (Kind::Vehicle, [0, 2, 0, 0, 0], [0, 0, 0, 0, 1]));
    assert_eq!(id("Fountain of Youth").def().bonus, &[Bonus::Krystallium; 3]);
}

#[test]
fn every_resource_and_every_character_can_be_had() {
    let all = catalogue();
    for res in Res::ALL {
        assert!(all.iter().any(|c| c.produces[res.index()] > 0), "nothing produces {res:?}");
        assert!(all.iter().any(|c| c.recycle == res), "nothing recycles into {res:?}");
        assert!(all.iter().any(|c| c.cost.res[res.index()] > 0), "nothing costs {res:?}");
    }
    assert!(all.iter().any(|c| c.cost.generals > 0));
    assert!(all.iter().any(|c| c.cost.financiers > 0));
    assert!(all.iter().any(|c| c.cost.krystallium > 0));
    for bonus in [Bonus::Krystallium, Bonus::Token(Token::General), Bonus::Token(Token::Financier)] {
        assert!(all.iter().any(|c| c.bonus.contains(&bonus)), "nobody gives {bonus:?}");
    }
}

#[test]
fn every_design_has_its_copies_in_the_deck() {
    // One of each design first, in catalogue order...
    for (i, card) in catalogue().iter().enumerate() {
        assert_eq!(CardId(i as u16).def(), card);
    }
    // ...then the extra copies.
    for card in catalogue() {
        let n = (0..CARDS as u16).filter(|&i| CardId(i).def().name == card.name).count();
        assert_eq!(n, card.copies as usize, "{}", card.name);
    }
    assert_eq!(twin("Recycling Plant").def().name, "Recycling Plant");
    assert_ne!(id("Recycling Plant"), twin("Recycling Plant"));
}

#[test]
fn cost_arithmetic() {
    let cost = Cost { res: [3, 0, 1, 0, 0], krystallium: 1, generals: 1, financiers: 0 };
    assert_eq!(cost.total(), 6);
    let half = Cost { res: [1, 5, 1, 0, 0], krystallium: 0, generals: 0, financiers: 3 };
    let left = cost.minus(&half);
    assert_eq!(left, Cost { res: [2, 0, 0, 0, 0], krystallium: 1, generals: 1, financiers: 0 });
    assert!(!left.is_zero());
    assert!(cost.minus(&cost).is_zero());
}

#[test]
fn supremacy_characters() {
    assert_eq!(Res::Materials.supremacy_token(), Some(Token::Financier));
    assert_eq!(Res::Energy.supremacy_token(), Some(Token::General));
    assert_eq!(Res::Gold.supremacy_token(), Some(Token::Financier));
    assert_eq!(Res::Exploration.supremacy_token(), Some(Token::General));
    assert_eq!(Res::Science.supremacy_token(), None);
}

// ----- dealing -----------------------------------------------------------

#[test]
fn a_new_game_deals_a_hand_to_everybody() {
    for n in MIN_PLAYERS..=Options::default().max_players() {
        let s = State::new(n, 5);
        let size = if n == 2 { 10 } else { 7 };
        assert_eq!(hand_sizes(n, Options::default()), (size, 0));
        assert_eq!((s.round, s.phase), (1, Phase::Draft));
        assert_eq!(s.deck.len(), DECK_SIZE - n * size);
        for p in &s.players {
            assert_eq!(p.hand.len(), size);
            assert!(p.picked.is_none() && p.drafted.is_empty());
        }
        assert_eq!(s.log, vec![Event::RoundStarted(1)]);
        check_cards(&s);
    }
}

#[test]
fn the_seed_decides_the_deal() {
    let a = State::new(3, 1);
    let b = State::new(3, 1);
    let c = State::new(3, 2);
    assert_eq!(a, b);
    assert_ne!(a.players[0].hand, c.players[0].hand);
}

#[test]
#[should_panic]
fn one_player_is_not_enough() {
    State::new(1, 0);
}

#[test]
#[should_panic]
fn six_players_do_not_fit() {
    State::new(6, 0);
}

// ----- the draft -----------------------------------------------------------

#[test]
fn hands_go_to_the_next_seat_in_round_one() {
    let mut s = State::new(4, 11);
    let before: Vec<Vec<CardId>> = s.players.iter().map(|p| p.hand.clone()).collect();
    let picks: Vec<CardId> = before.iter().map(|h| h[0]).collect();
    for (seat, &card) in picks.iter().enumerate() {
        s.apply(seat, Action::Draft { card }).unwrap();
    }
    for (seat, &pick) in picks.iter().enumerate() {
        let from = (seat + 3) % 4;
        assert_eq!(s.players[seat].hand, before[from][1..], "seat {seat} should get seat {from}'s hand");
        assert_eq!(s.players[seat].drafted, vec![pick]);
    }
}

#[test]
fn hands_go_to_the_previous_seat_in_round_two() {
    let mut s = State::new(4, 11);
    s.round = 2;
    assert!(!s.passes_to_next());
    let before: Vec<Vec<CardId>> = s.players.iter().map(|p| p.hand.clone()).collect();
    for (seat, hand) in before.iter().enumerate() {
        s.apply(seat, Action::Draft { card: hand[0] }).unwrap();
    }
    for seat in 0..4 {
        let from = (seat + 1) % 4;
        assert_eq!(s.players[seat].hand, before[from][1..], "seat {seat} should get seat {from}'s hand");
    }
    s.round = 3;
    assert!(s.passes_to_next());
}

#[test]
fn nothing_moves_until_everybody_has_picked() {
    let mut s = State::new(3, 4);
    let hands: Vec<Vec<CardId>> = s.players.iter().map(|p| p.hand.clone()).collect();
    let card = hands[0][2];
    s.apply(0, Action::Draft { card }).unwrap();
    // Seat 0 holds the others back and keeps its pick face down.
    assert_eq!(s.players[0].picked, Some(card));
    assert!(s.players[0].drafted.is_empty());
    assert_eq!(s.players[1].hand, hands[1]);
    assert_eq!(s.apply(0, Action::Draft { card: hands[0][0] }), Err(Error::AlreadyPicked));
    s.apply(1, Action::Draft { card: hands[1][0] }).unwrap();
    assert_eq!(s.players[2].hand, hands[2], "still waiting for seat 2");
    s.apply(2, Action::Draft { card: hands[2][0] }).unwrap();
    assert!(s.players.iter().all(|p| p.picked.is_none() && p.drafted.len() == 1));
}

#[test]
fn a_pick_must_come_from_your_own_hand() {
    let mut s = State::new(3, 4);
    let other = s.players[1].hand[0];
    assert_eq!(s.apply(0, Action::Draft { card: other }), Err(Error::NotInHand));
    assert_eq!(s.apply(0, Action::Draft { card: CardId(9999) }), Err(Error::NotInHand));
    assert_eq!(s.apply(7, Action::Ready), Err(Error::UnknownSeat));
}

#[test]
fn only_draft_moves_work_during_the_draft() {
    let mut s = State::new(3, 4);
    let card = s.players[0].hand[0];
    assert_eq!(s.apply(0, Action::Build { card }), Err(Error::WrongPhase));
    assert_eq!(s.apply(0, Action::Recycle { card, to: Target::Empire }), Err(Error::WrongPhase));
    assert_eq!(s.apply(0, Action::Ready), Err(Error::WrongPhase));
    assert_eq!(
        s.apply(0, Action::Place { piece: Piece::Cube(Res::Gold), target: Target::Empire }),
        Err(Error::WrongPhase)
    );
    assert_eq!(s.apply(0, Action::Choose { token: Token::General }), Err(Error::WrongPhase));
    assert_eq!(s.apply(0, Action::Scrap { card }), Err(Error::WrongPhase));
}

#[test]
fn seven_picks_end_the_draft() {
    for n in 3..=Options::default().max_players() {
        let s = to_planning(n, 21);
        for p in &s.players {
            assert_eq!(p.drafted.len(), DRAFT_PICKS);
            assert!(p.hand.is_empty() && p.picked.is_none() && !p.ready);
        }
        assert!(s.discard.is_empty(), "nothing is left over with {n} players");
        check_cards(&s);
    }
}

#[test]
fn with_two_players_the_leftovers_are_discarded() {
    let s = to_planning(2, 21);
    for p in &s.players {
        assert_eq!(p.drafted.len(), DRAFT_PICKS);
        assert!(p.hand.is_empty());
    }
    assert_eq!(s.discard.len(), 6, "3 cards each");
    assert!(s.players.iter().all(|p| p.empire_cubes == 0), "no recycling bonus");
    check_cards(&s);
}

// ----- planning ------------------------------------------------------------

#[test]
fn building_puts_a_card_under_construction() {
    let mut s = to_planning(3, 9);
    let card = s.players[1].drafted[3];
    s.apply(1, Action::Build { card }).unwrap();
    assert_eq!(s.players[1].drafted.len(), 6);
    assert_eq!(s.players[1].buildings, vec![Building { card, filled: Cost::default(), jokers: [0; 5] }]);
    // It can't be built twice, and it isn't someone else's.
    assert_eq!(s.apply(1, Action::Build { card }), Err(Error::NotDrafted));
    let theirs = s.players[0].drafted[0];
    assert_eq!(s.apply(1, Action::Build { card: theirs }), Err(Error::NotDrafted));
}

#[test]
fn recycling_gives_its_cube_to_the_empire() {
    let mut s = to_planning(3, 9);
    let card = s.players[0].drafted[0];
    s.apply(0, Action::Recycle { card, to: Target::Empire }).unwrap();
    assert_eq!(s.players[0].empire_cubes, 1);
    assert_eq!(s.players[0].drafted.len(), 6);
    assert_eq!(s.discard, vec![card]);
    check_cards(&s);
}

#[test]
fn a_recycled_cube_must_fit_where_it_goes() {
    let mut s = blank(2);
    s.players[0].drafted = vec![id("Recycling Plant"), id("Wind Turbines"), id("Zeppelin")];
    s.apply(0, Action::Build { card: id("Recycling Plant") }).unwrap();
    let discarded = s.discard.len(); // the two-player draft already discarded some
    // Wind Turbines recycle into Energy; the Recycling Plant only has spaces for Materials.
    let to = Target::Card(id("Recycling Plant"));
    assert_eq!(s.apply(0, Action::Recycle { card: id("Wind Turbines"), to }), Err(Error::InvalidTarget));
    // Nothing happened: the card is still there and not discarded.
    assert!(s.players[0].drafted.contains(&id("Wind Turbines")));
    assert_eq!(s.discard.len(), discarded);
    // A building you don't have is no target either.
    let nowhere = Target::Card(id("Nuclear Plant"));
    assert_eq!(s.apply(0, Action::Recycle { card: id("Wind Turbines"), to: nowhere }), Err(Error::InvalidTarget));
    assert_eq!(s.apply(0, Action::Recycle { card: id("Wind Turbines"), to: Target::Empire }), Ok(()));
    assert_eq!(s.discard.len(), discarded + 1);
}

#[test]
fn recycling_can_finish_a_building_at_once() {
    let mut s = blank(2);
    s.players[0].buildings = vec![Building {
        card: id("Wind Turbines"),
        filled: Cost { res: [1, 0, 0, 0, 0], ..Cost::default() },
        jokers: [0; 5],
    }];
    s.players[0].drafted = vec![id("Recycling Plant")]; // recycles into Materials
    s.apply(0, Action::Recycle { card: id("Recycling Plant"), to: Target::Card(id("Wind Turbines")) }).unwrap();
    assert!(s.players[0].buildings.is_empty());
    assert_eq!(s.players[0].empire, vec![id("Wind Turbines")]);
    assert!(s.log.contains(&Event::Completed { seat: 0, card: id("Wind Turbines") }));
    // It produces from now on (Noram States has no Energy icon of its own).
    assert_eq!(s.production(0, Res::Energy), 1);
}

#[test]
fn finishing_a_building_pays_its_bonus() {
    let almost = |card: &str, missing: usize| Building {
        card: id(card),
        filled: {
            let mut filled = id(card).def().cost;
            filled.res[missing] -= 1;
            filled
        },
        jokers: [0; 5],
    };
    let mut s = blank(2);
    s.players[0].buildings = vec![
        almost("Transmutation", Res::Gold.index()),           // a Krystallium
        almost("Industrial Complex", Res::Materials.index()), // a Financier
        almost("Military Base", Res::Materials.index()),      // a General
        almost("Genetic Upgrades", Res::Science.index()),     // two Financiers
    ];
    // Recycling into Gold, Materials, Materials and Science.
    s.players[0].drafted =
        vec![id("Financial Center"), id("Recycling Plant"), twin("Recycling Plant"), id("Research Center")];
    let finish = |s: &mut State, card: CardId, to: &str| {
        s.apply(0, Action::Recycle { card, to: Target::Card(id(to)) }).unwrap();
    };
    finish(&mut s, id("Financial Center"), "Transmutation");
    assert_eq!(s.players[0].krystallium, 1);
    finish(&mut s, id("Recycling Plant"), "Industrial Complex");
    assert_eq!(s.players[0].financiers, 1);
    finish(&mut s, twin("Recycling Plant"), "Military Base");
    assert_eq!(s.players[0].generals, 1);
    finish(&mut s, id("Research Center"), "Genetic Upgrades");
    assert_eq!(s.players[0].financiers, 3);
    assert_eq!(s.players[0].empire.len(), 4);
}

#[test]
fn scrapping_loses_what_was_placed_and_sends_the_recycling_cube_to_the_empire() {
    let mut s = blank(2);
    let placed = Cost { res: [3, 1, 0, 0, 0], krystallium: 1, ..Cost::default() };
    s.players[0].buildings = vec![
        Building { card: id("Juggernaut"), filled: placed, jokers: [0; 5] },
        Building { card: id("Recycling Plant"), filled: Cost::default(), jokers: [0; 5] },
    ];
    s.players[0].empire_cubes = 4;
    s.apply(0, Action::Scrap { card: id("Juggernaut") }).unwrap();
    assert_eq!(s.players[0].buildings.len(), 1);
    assert_eq!(s.discard.last(), Some(&id("Juggernaut")));
    // Its cube made the fifth on the Empire; the Krystallium on it is gone.
    assert_eq!((s.players[0].empire_cubes, s.players[0].krystallium), (0, 1));
    assert_eq!(s.apply(0, Action::Scrap { card: id("Juggernaut") }), Err(Error::NotUnderConstruction));
    assert_eq!(s.apply(0, Action::Scrap { card: id("Zeppelin") }), Err(Error::NotUnderConstruction));
}

#[test]
fn planning_ends_when_everyone_has_decided_everything() {
    let mut s = to_planning(2, 3);
    // Cards left to decide: not ready yet.
    assert_eq!(s.apply(0, Action::Ready), Err(Error::CardsLeftToPlan));
    for seat in 0..2 {
        for card in s.players[seat].drafted.clone() {
            s.apply(seat, Action::Build { card }).unwrap();
        }
    }
    s.apply(0, Action::Ready).unwrap();
    assert_eq!(s.phase, Phase::Planning, "seat 1 is still planning");
    assert!(s.players[0].ready);
    s.apply(1, Action::Ready).unwrap();
    assert!(matches!(s.phase, Phase::Production { .. }));
    check_cards(&s);
}

// ----- production ----------------------------------------------------------

#[test]
fn production_adds_the_empire_cards_and_scaling_icons() {
    let mut s = blank(2);
    // Noram States starts with 3 Materials and 1 Gold, the Republic of
    // Europe with 2 Materials, 1 Energy and 1 Science.
    assert_eq!(Res::ALL.map(|r| s.production(0, r)), [3, 0, 0, 1, 0]);
    assert_eq!(Res::ALL.map(|r| s.production(1, r)), [2, 1, 1, 0, 0]);
    // Recycling Plant 2 + Industrial Complex 1 + Robot Assistants (1 per
    // Structure: the other two).
    s.players[0].empire = vec![id("Recycling Plant"), id("Industrial Complex"), id("Robot Assistants")];
    assert_eq!(s.production(0, Res::Materials), 3 + 2 + 1 + 2);
    // Zeppelin 1 + Super-Sonar (1 per Vehicle: the Zeppelin, not itself).
    s.players[0].empire.extend([id("Zeppelin"), id("Super-Sonar")]);
    assert_eq!(s.production(0, Res::Exploration), 1 + 1);
    // Another Structure: its own icons and one more for the Robot Assistants.
    s.players[0].empire.push(twin("Recycling Plant"));
    assert_eq!(s.production(0, Res::Materials), 3 + 2 + 1 + 2 + (2 + 1));
}

#[test]
fn the_best_producer_takes_the_character() {
    let mut s = blank(2);
    start_production(&mut s);
    // Materials: Noram States makes 3, the Republic of Europe 2.
    assert_eq!(s.phase, Phase::Production { step: 0 });
    assert_eq!((s.players[0].financiers, s.players[0].generals), (1, 0));
    assert_eq!(s.players[1].generals + s.players[1].financiers, 0);
    assert_eq!((s.players[0].pool, s.players[0].produced), (3, 3));
    assert_eq!((s.players[1].pool, s.players[1].produced), (2, 2));
    assert!(!s.players[0].ready && !s.players[1].ready, "both have cubes to place");
    assert!(s.log.contains(&Event::Supremacy { res: Res::Materials, seats: vec![0] }));
    skip_to(&mut s, 1);
    // Energy: only the Republic of Europe makes any.
    assert_eq!(s.players[1].generals, 1);
    assert!(s.players[0].ready, "has nothing to do in this step");
}

#[test]
fn each_resource_gives_its_own_character() {
    let mut s = blank(2);
    s.players[1].empire = vec![id("Blackbeard's Treasure")]; // Gold 1, Exploration 1
    start_production(&mut s);
    assert_eq!(s.players[0].financiers, 1, "Materials: a Financier");
    skip_to(&mut s, 2);
    assert_eq!(s.players[1].generals, 1, "Energy: a General");
    // Science: seat 1 chooses (and `skip_step` picks a General).
    skip_to(&mut s, 3);
    assert_eq!(s.players[1].generals, 2);
    // Gold: 1 each, a tie.
    assert!(s.log.contains(&Event::Supremacy { res: Res::Gold, seats: vec![] }));
    assert_eq!(s.players[0].financiers + s.players[1].financiers, 1);
    skip_to(&mut s, 4);
    // Exploration: only seat 1 makes any.
    assert_eq!(s.players[1].generals, 3, "Exploration: a General");
    assert!(s.log.contains(&Event::Supremacy { res: Res::Exploration, seats: vec![1] }));
}

#[test]
fn the_round_keeps_who_took_each_character_for_the_board() {
    let mut s = blank(2);
    s.players[1].empire = vec![id("Blackbeard's Treasure")]; // Gold 1, Exploration 1
    assert!(s.awards.is_empty());
    start_production(&mut s);
    assert_eq!(s.awards, vec![vec![0]], "Materials: decided as the step starts");
    skip_to(&mut s, 4);
    // Energy and Science to seat 1, Gold tied, Exploration to seat 1.
    assert_eq!(s.awards, vec![vec![0], vec![1], vec![1], vec![], vec![1]]);
    assert_eq!(s.view_for(0).awards, s.awards);
    finish_round(&mut s);
    assert_eq!((s.round, s.phase), (2, Phase::Draft));
    assert!(s.awards.is_empty() && s.view_for(1).awards.is_empty(), "a new round, a new board");
}

#[test]
fn a_tie_for_the_most_gives_nobody_a_character() {
    let mut s = blank(2);
    s.players[1].empire = vec![id("Industrial Complex")]; // Materials 2 + 1, like Noram's 3
    start_production(&mut s);
    assert_eq!((s.players[0].produced, s.players[1].produced), (3, 3));
    assert!(s.log.contains(&Event::Supremacy { res: Res::Materials, seats: vec![] }));
    for p in &s.players {
        assert_eq!(p.generals + p.financiers, 0, "nobody took the Materials character");
    }
}

#[test]
fn the_empires_are_the_seven_side_a_ones() {
    for empire in EMPIRES {
        let icons: u32 = empire.base.iter().map(|&n| n as u32).sum();
        assert!((3..=4).contains(&icons), "{}", empire.name);
        assert!(empire.combo.is_some() != empire.per_token.is_some(), "{} scores one way", empire.name);
    }
    for res in Res::ALL {
        assert!(EMPIRES.iter().filter(|e| !e.ascension).any(|e| e.base[res.index()] > 0), "{res:?}");
    }
    let mut names: Vec<_> = EMPIRES.iter().map(|e| e.name).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 7);
    // Five come with the base game, two with Corruption & Ascension.
    let expansion: Vec<_> = EMPIRES.iter().filter(|e| e.ascension).map(|e| e.name).collect();
    assert_eq!(expansion, ["Nations of Oceania", "North Hegemony"]);
}

#[test]
fn the_science_winner_chooses_the_character() {
    let mut s = blank(2);
    s.players[0].empire = vec![id("Research Center")]; // Science 2 against 1
    start_production(&mut s);
    skip_to(&mut s, 2);
    assert!(s.players[0].choose);
    assert!(!s.players[1].choose);
    assert_eq!(s.apply(0, Action::Ready), Err(Error::MustChoose));
    assert_eq!(s.apply(1, Action::Choose { token: Token::General }), Err(Error::NothingToChoose));
    s.apply(0, Action::Choose { token: Token::Financier }).unwrap();
    assert!(!s.players[0].choose);
    assert_eq!(s.players[0].financiers, 2, "one for Materials, one chosen");
    assert!(s.log.contains(&Event::Chose { seat: 0, token: Token::Financier }));
    assert_eq!(s.apply(0, Action::Choose { token: Token::General }), Err(Error::NothingToChoose));
    // The cubes are still to be placed.
    assert!(!s.players[0].ready);
}

#[test]
fn cubes_come_from_the_pool() {
    let mut s = blank(2);
    // Noram States makes 3 Materials; the Recycling Plant needs 2.
    s.players[0].buildings = vec![Building { card: id("Recycling Plant"), filled: Cost::default(), jokers: [0; 5] }];
    start_production(&mut s);
    assert_eq!(s.players[0].pool, 3);

    let materials = Piece::Cube(Res::Materials);
    let plant = Target::Card(id("Recycling Plant"));
    // Wrong resource for this step, and a building that isn't there.
    assert_eq!(
        s.apply(0, Action::Place { piece: Piece::Cube(Res::Energy), target: plant }),
        Err(Error::NothingToPlace)
    );
    assert_eq!(
        s.apply(0, Action::Place { piece: materials, target: Target::Card(id("Nuclear Plant")) }),
        Err(Error::InvalidTarget)
    );
    assert_eq!(s.players[0].pool, 3, "refused moves cost nothing");

    s.apply(0, Action::Place { piece: materials, target: plant }).unwrap();
    assert_eq!(s.players[0].pool, 2);
    assert_eq!(s.players[0].buildings[0].filled.res[0], 1);
    s.apply(0, Action::Place { piece: materials, target: plant }).unwrap();
    assert!(s.players[0].buildings.is_empty(), "the Recycling Plant is built as soon as it is full");
    assert!(s.players[0].empire.contains(&id("Recycling Plant")));
    // It is gone now, so the last cube goes to the Empire.
    assert_eq!(s.apply(0, Action::Place { piece: materials, target: plant }), Err(Error::InvalidTarget));
    s.apply(0, Action::Place { piece: materials, target: Target::Empire }).unwrap();
    assert_eq!(s.players[0].empire_cubes, 1);
    assert!(!s.players[0].ready, "everything placed, but Done not pressed");
    s.apply(1, Action::Ready).unwrap();
    assert_eq!(s.phase, Phase::Production { step: 0 }, "seat 0 hasn't pressed Done");
    s.apply(0, Action::Ready).unwrap();
    assert_eq!(s.phase, Phase::Production { step: 1 });
    assert_eq!(s.players[0].pool, 0, "no Energy, and nothing left over");
}

#[test]
fn saying_ready_puts_the_cubes_left_on_the_empire() {
    let mut s = blank(2);
    start_production(&mut s);
    assert_eq!(s.players[0].pool, 3);
    s.apply(0, Action::Ready).unwrap();
    s.apply(1, Action::Ready).unwrap();
    assert_eq!(s.players[0].empire_cubes, 3, "nothing placed, so all on the Empire");
    // Energy now, where only seat 1 makes anything.
    assert_eq!(s.phase, Phase::Production { step: 1 });
    assert_eq!((s.players[0].produced, s.players[0].pool), (0, 0));
    assert_eq!((s.players[1].produced, s.players[1].pool), (1, 1));
}

#[test]
fn five_cubes_on_the_empire_make_krystallium() {
    let mut s = blank(2);
    s.players[0].empire = vec![id("Recycling Plant"), twin("Recycling Plant")]; // 3 + 2 + 2
    start_production(&mut s);
    assert_eq!(s.players[0].pool, 7);
    for _ in 0..5 {
        s.apply(0, Action::Place { piece: Piece::Cube(Res::Materials), target: Target::Empire }).unwrap();
    }
    // At once, and for good: undo stops there.
    assert_eq!((s.players[0].empire_cubes, s.players[0].krystallium), (0, 1));
    assert_eq!(s.apply(0, Action::Undo), Err(Error::NothingToUndo));
    assert_eq!(s.players[0].pool, 2);
    for _ in 0..2 {
        s.apply(0, Action::Place { piece: Piece::Cube(Res::Materials), target: Target::Empire }).unwrap();
    }
    assert_eq!((s.players[0].empire_cubes, s.players[0].krystallium), (2, 1));
    s.apply(0, Action::Undo).unwrap();
    assert_eq!((s.players[0].empire_cubes, s.players[0].pool), (1, 1));
}

#[test]
fn krystallium_stands_in_for_any_cube_but_not_for_characters() {
    let mut s = blank(2);
    // Seat 1 makes less Materials than seat 0, so it takes no character at
    // the start. Center of the Earth: 5 Exploration and two Generals.
    s.players[1].buildings = vec![Building { card: id("Center of the Earth"), filled: Cost::default(), jokers: [0; 5] }];
    s.players[1].krystallium = 5;
    s.players[1].generals = 2;
    s.players[1].financiers = 1;
    start_production(&mut s);
    let center = Target::Card(id("Center of the Earth"));
    // No Energy space on it, no Krystallium space, nor a character space for a Financier.
    for piece in [Piece::Krystallium(Some(Res::Energy)), Piece::Krystallium(None), Piece::Financier] {
        assert_eq!(s.apply(1, Action::Place { piece, target: center }), Err(Error::InvalidTarget), "{piece:?}");
    }
    // Krystallium doesn't go on the Empire.
    assert_eq!(
        s.apply(1, Action::Place { piece: Piece::Krystallium(Some(Res::Materials)), target: Target::Empire }),
        Err(Error::InvalidTarget)
    );
    for _ in 0..5 {
        s.apply(1, Action::Place { piece: Piece::Krystallium(Some(Res::Exploration)), target: center }).unwrap();
    }
    assert_eq!(s.players[1].krystallium, 0);
    for _ in 0..2 {
        assert_eq!(s.players[1].buildings.len(), 1, "still needs its Generals");
        s.apply(1, Action::Place { piece: Piece::General, target: center }).unwrap();
    }
    assert!(s.players[1].buildings.is_empty());
    assert!(s.players[1].empire.contains(&id("Center of the Earth")));
    assert_eq!(s.players[1].generals, 0);
}

#[test]
fn some_spaces_take_only_krystallium() {
    let mut s = blank(2);
    // Secret Society: 3 Gold (all placed here) and a Krystallium.
    let gold_placed = Cost { res: [0, 0, 0, 3, 0], ..Cost::default() };
    s.players[0].buildings = vec![
        Building { card: id("Secret Society"), filled: gold_placed, jokers: [0; 5] },
        Building { card: id("Recycling Plant"), filled: Cost::default(), jokers: [0; 5] },
    ];
    s.players[0].krystallium = 2;
    let society = Target::Card(id("Secret Society"));
    assert_eq!(s.players[0].buildings[0].remaining(), Cost { krystallium: 1, ..Cost::default() });
    // Standing in for Gold, there's no space left; and the plant has no Krystallium space.
    assert_eq!(
        s.apply(0, Action::Place { piece: Piece::Krystallium(Some(Res::Gold)), target: society }),
        Err(Error::InvalidTarget)
    );
    let plant = Target::Card(id("Recycling Plant"));
    assert_eq!(s.apply(0, Action::Place { piece: Piece::Krystallium(None), target: plant }), Err(Error::InvalidTarget));
    // In planning already.
    s.apply(0, Action::Place { piece: Piece::Krystallium(None), target: society }).unwrap();
    assert!(s.players[0].empire.contains(&id("Secret Society")));
    assert_eq!(s.players[0].krystallium, 1);
}

#[test]
fn you_cannot_place_what_you_do_not_hold() {
    let mut s = blank(2);
    s.players[1].buildings = vec![Building { card: id("Center of the Earth"), filled: Cost::default(), jokers: [0; 5] }];
    start_production(&mut s);
    let center = Target::Card(id("Center of the Earth"));
    assert_eq!(s.apply(1, Action::Place { piece: Piece::General, target: center }), Err(Error::NothingToPlace));
    assert_eq!(
        s.apply(1, Action::Place { piece: Piece::Krystallium(Some(Res::Exploration)), target: center }),
        Err(Error::NothingToPlace)
    );
    // Cubes are only held during their own production step.
    s.players[1].buildings.push(Building { card: id("Zeppelin"), filled: Cost::default(), jokers: [0; 5] });
    let zeppelin = Target::Card(id("Zeppelin"));
    assert_eq!(s.apply(1, Action::Place { piece: Piece::Cube(Res::Energy), target: zeppelin }), Err(Error::NothingToPlace));
}

#[test]
fn a_building_finished_in_one_step_produces_in_the_next() {
    let mut s = blank(2);
    s.players[0].buildings = vec![Building {
        card: id("Wind Turbines"),
        filled: Cost { res: [1, 0, 0, 0, 0], ..Cost::default() },
        jokers: [0; 5],
    }];
    start_production(&mut s);
    assert_eq!(s.players[0].produced, 3);
    // Finish the Wind Turbines (Energy 1) during Materials...
    s.apply(0, Action::Place { piece: Piece::Cube(Res::Materials), target: Target::Card(id("Wind Turbines")) })
        .unwrap();
    skip_to(&mut s, 1);
    // ...and they already produce in the Energy step (Noram States has no Energy icon).
    assert_eq!(s.players[0].produced, 1);
}

#[test]
fn only_players_with_something_to_place_wait_in_the_wrap_up() {
    let mut s = blank(2);
    s.players[0].buildings = vec![Building { card: id("Recycling Plant"), filled: Cost::default(), jokers: [0; 5] }];
    s.players[0].krystallium = 1;
    s.players[1].krystallium = 1; // nothing to put it on
    start_production(&mut s);
    skip_to(&mut s, WRAP_UP);
    assert!(!s.players[0].ready);
    assert!(s.players[1].ready);
    assert_eq!(s.round, 1);
    let plant = Target::Card(id("Recycling Plant"));
    s.apply(0, Action::Place { piece: Piece::Krystallium(Some(Res::Materials)), target: plant }).unwrap();
    assert_eq!(s.phase, Phase::Production { step: WRAP_UP }, "still going: it may place more");
    s.apply(0, Action::Ready).unwrap();
    assert_eq!((s.round, s.phase), (2, Phase::Draft));
}

#[test]
fn the_round_ends_by_dealing_the_next_one() {
    let mut s = blank(2);
    s.players[0].buildings = vec![Building { card: id("Recycling Plant"), filled: Cost::default(), jokers: [0; 5] }];
    s.players[0].krystallium = 2;
    start_production(&mut s);
    skip_to(&mut s, WRAP_UP);
    s.apply(0, Action::Ready).unwrap();
    assert_eq!((s.round, s.phase), (2, Phase::Draft));
    // The round's unplaced cubes (3 Materials, 1 more) went on the Empire;
    // buildings and Krystallium stay.
    assert_eq!(s.players[0].empire_cubes, 4);
    assert_eq!(s.players[0].buildings.len(), 1);
    assert_eq!(s.players[0].krystallium, 2);
    // New hands, nobody ready, nothing picked.
    for p in &s.players {
        assert_eq!(p.hand.len(), 10);
        assert!(p.picked.is_none() && p.drafted.is_empty() && !p.ready && p.pool == 0);
    }
    assert_eq!(s.deck.len(), DECK_SIZE - 2 * 2 * 10);
    assert_eq!(s.log.last(), Some(&Event::RoundStarted(2)));
}

#[test]
fn a_round_with_nothing_to_wrap_up_skips_the_step() {
    let mut s = blank(2);
    start_production(&mut s);
    finish_round(&mut s);
    assert_eq!((s.round, s.phase), (2, Phase::Draft));
}

// ----- placing until Done ----------------------------------------------------

#[test]
fn the_step_ends_when_everybody_has_pressed_done() {
    let mut s = blank(2);
    start_production(&mut s);
    // Seat 0 places all of its Materials: that alone doesn't end its step.
    for _ in 0..3 {
        s.apply(0, Action::Place { piece: Piece::Cube(Res::Materials), target: Target::Empire }).unwrap();
    }
    assert_eq!(s.players[0].pool, 0);
    assert!(!s.players[0].ready);
    s.apply(1, Action::Ready).unwrap();
    assert_eq!(s.phase, Phase::Production { step: 0 });
    s.apply(0, Action::Ready).unwrap();
    assert_eq!(s.phase, Phase::Production { step: 1 });
}

#[test]
fn undo_takes_back_what_was_placed_last() {
    let mut s = blank(2);
    s.players[0].buildings = vec![Building { card: id("Center of the Earth"), filled: Cost::default(), jokers: [0; 5] }];
    s.players[0].krystallium = 1;
    s.players[0].generals = 1;
    s.players[1].buildings = vec![Building { card: id("Recycling Plant"), filled: Cost::default(), jokers: [0; 5] }];
    // Not in planning: what is placed there is final.
    assert_eq!(s.apply(0, Action::Undo), Err(Error::WrongPhase));
    start_production(&mut s);
    assert_eq!(s.apply(0, Action::Undo), Err(Error::NothingToUndo));
    let center = Target::Card(id("Center of the Earth"));
    let financiers = s.players[0].financiers; // the Materials supremacy
    s.apply(0, Action::Place { piece: Piece::Cube(Res::Materials), target: Target::Empire }).unwrap();
    s.apply(0, Action::Place { piece: Piece::Krystallium(Some(Res::Exploration)), target: center }).unwrap();
    s.apply(0, Action::Place { piece: Piece::General, target: center }).unwrap();
    assert_eq!(
        s.view_for(0).placed,
        vec![
            Placement { piece: Piece::Cube(Res::Materials), target: Target::Empire },
            Placement { piece: Piece::Krystallium(Some(Res::Exploration)), target: center },
            Placement { piece: Piece::General, target: center },
        ]
    );
    assert!(s.view_for(1).placed.is_empty(), "your own placements only");
    s.apply(0, Action::Undo).unwrap();
    let p = &s.players[0];
    assert_eq!((p.generals, p.buildings[0].filled.generals), (1, 0));
    assert_eq!(s.players[0].buildings[0].jokers, [0, 0, 0, 0, 1], "a Krystallium on an Exploration space");
    s.apply(0, Action::Undo).unwrap();
    let p = &s.players[0];
    assert_eq!((p.krystallium, p.buildings[0].filled.res[Res::Exploration.index()]), (1, 0));
    assert_eq!(p.buildings[0].jokers, [0; 5]);
    s.apply(0, Action::Undo).unwrap();
    let p = &s.players[0];
    assert_eq!((p.pool, p.empire_cubes, p.financiers), (3, 0, financiers));
    assert_eq!(s.apply(0, Action::Undo), Err(Error::NothingToUndo));
    // Seat 1's placements are its own.
    s.apply(1, Action::Place { piece: Piece::Cube(Res::Materials), target: Target::Card(id("Recycling Plant")) })
        .unwrap();
    assert_eq!(s.apply(0, Action::Undo), Err(Error::NothingToUndo));
    // Done makes them final.
    s.apply(1, Action::Ready).unwrap();
    assert_eq!(s.apply(1, Action::Undo), Err(Error::NothingToUndo));
    assert_eq!(s.players[1].buildings[0].filled.res[0], 1);
}

#[test]
fn a_building_is_built_once_full_and_that_cannot_be_undone() {
    let mut s = blank(2);
    let mut filled = id("Industrial Complex").def().cost; // 3 Materials, 1 Energy: a Financier
    filled.res[Res::Materials.index()] -= 1;
    s.players[0].buildings = vec![Building { card: id("Industrial Complex"), filled, jokers: [0; 5] }];
    start_production(&mut s);
    let financiers = s.players[0].financiers;
    let complex = Target::Card(id("Industrial Complex"));
    s.apply(0, Action::Place { piece: Piece::Cube(Res::Materials), target: Target::Empire }).unwrap();
    s.apply(0, Action::Place { piece: Piece::Cube(Res::Materials), target: complex }).unwrap();
    // Built and paid at once, as in the published game.
    assert!(s.players[0].buildings.is_empty());
    assert_eq!(s.players[0].empire, vec![id("Industrial Complex")]);
    assert_eq!(s.players[0].financiers, financiers + 1);
    assert!(s.log.contains(&Event::Completed { seat: 0, card: id("Industrial Complex") }));
    // Undo stops there: neither that cube nor the one before can be taken back.
    assert_eq!(s.apply(0, Action::Undo), Err(Error::NothingToUndo));
    assert_eq!((s.players[0].empire_cubes, s.players[0].pool), (1, 1));
    // What comes after can.
    s.apply(0, Action::Place { piece: Piece::Cube(Res::Materials), target: Target::Empire }).unwrap();
    s.apply(0, Action::Undo).unwrap();
    assert_eq!((s.players[0].empire_cubes, s.players[0].pool), (1, 1));
    // Done puts the cube left on the Empire, and the step still waits for it.
    assert!(!s.players[0].ready);
    s.apply(0, Action::Ready).unwrap();
    assert_eq!((s.players[0].empire_cubes, s.players[0].pool), (2, 0));
    assert_eq!(s.apply(0, Action::Place { piece: Piece::Cube(Res::Materials), target: Target::Empire }), Err(Error::NothingToPlace));
}

#[test]
fn done_puts_the_cubes_left_on_the_empire() {
    let mut s = blank(2);
    // Noram States makes 3 Materials; nothing to build.
    s.players[0].empire_cubes = 3;
    start_production(&mut s);
    assert_eq!(s.players[0].pool, 3);
    let krystallium = s.players[0].krystallium;
    s.apply(0, Action::Ready).unwrap();
    // Nothing is lost: the five Empire cubes become a Krystallium at once.
    assert_eq!(s.players[0].pool, 0);
    assert_eq!((s.players[0].empire_cubes, s.players[0].krystallium), (1, krystallium + 1));
    assert_eq!(s.apply(0, Action::Undo), Err(Error::NothingToUndo));
}

#[test]
fn placing_after_done_reopens_your_step() {
    let mut s = blank(2);
    s.players[0].buildings = vec![Building { card: id("Center of the Earth"), filled: Cost::default(), jokers: [0; 5] }];
    s.players[0].generals = 1;
    start_production(&mut s);
    s.apply(0, Action::Ready).unwrap();
    assert!(s.players[0].ready);
    s.apply(0, Action::Place { piece: Piece::General, target: Target::Card(id("Center of the Earth")) }).unwrap();
    assert!(!s.players[0].ready, "it must press Done again");
    s.apply(1, Action::Ready).unwrap();
    assert_eq!(s.phase, Phase::Production { step: 0 });
    s.apply(0, Action::Ready).unwrap();
    assert_eq!(s.phase, Phase::Production { step: 1 });
}

#[test]
fn scrapping_a_building_loses_what_was_placed_on_it_this_step() {
    let mut s = blank(2);
    s.players[0].buildings = vec![Building { card: id("Recycling Plant"), filled: Cost::default(), jokers: [0; 5] }];
    s.players[0].empire_cubes = 2;
    start_production(&mut s);
    let plant = Target::Card(id("Recycling Plant"));
    s.apply(0, Action::Place { piece: Piece::Cube(Res::Materials), target: Target::Empire }).unwrap();
    s.apply(0, Action::Place { piece: Piece::Cube(Res::Materials), target: plant }).unwrap();
    // Its recycling cube goes on the Empire; the cube on the plant is lost.
    s.apply(0, Action::Scrap { card: id("Recycling Plant") }).unwrap();
    assert_eq!(s.players[0].placed, vec![Placement { piece: Piece::Cube(Res::Materials), target: Target::Empire }]);
    assert_eq!(s.players[0].empire_cubes, 4);
    s.apply(0, Action::Undo).unwrap();
    assert_eq!((s.players[0].empire_cubes, s.players[0].pool), (3, 2));
    // Two more make five: a Krystallium at once, for good.
    for _ in 0..2 {
        s.apply(0, Action::Place { piece: Piece::Cube(Res::Materials), target: Target::Empire }).unwrap();
    }
    assert_eq!((s.players[0].empire_cubes, s.players[0].krystallium), (0, 1));
    assert_eq!(s.apply(0, Action::Undo), Err(Error::NothingToUndo));
}

// ----- scoring -------------------------------------------------------------

#[test]
fn points_add_up_from_cards_types_and_characters() {
    let mut s = blank(2);
    s.players[0].empire = vec![
        id("Island of Avalon"),       // 7
        id("Bionic Grafts"),          // 4
        id("Transportation Network"), // 1 per Vehicle
        id("Zeppelin"),
        id("Tank Division"),
        id("Security Automatons"), // 1 per General
    ];
    s.players[0].generals = 1;
    s.players[0].financiers = 2;
    let score = s.score_of(0);
    assert_eq!(score.gross, 7 + 4);
    // Two Vehicles, one General, and Noram States' 1 per Financier (2).
    assert_eq!(score.combo, 2 + 1 + 2);
    assert_eq!((score.generals, score.financiers), (1, 2));
    assert_eq!(score.total, 11 + 5 + 1 + 2);
    assert_eq!(score.cards, 6);
    // Cubes, Krystallium and unfinished buildings are worth nothing.
    s.players[0].krystallium = 4;
    s.players[0].empire_cubes = 3;
    s.players[0].buildings = vec![Building { card: id("Time Travel"), filled: Cost::default(), jokers: [0; 5] }];
    assert_eq!(s.score_of(0), score);
}

#[test]
fn combos_count_the_cards_you_own_of_a_type() {
    let mut s = blank(2);
    s.players[0].empire = vec![id("National Monument"), id("Giant Tower"), id("Espionage Agency"), id("Recycling Plant")];
    // National Monument: 2 per Project (it, the Giant Tower and the Espionage Agency).
    let score = s.score_of(0);
    assert_eq!(score.combo, 6);
    assert_eq!(score.gross, 10 + 1);
}

#[test]
fn the_empire_scores_too() {
    let mut s = blank(3);
    // The Federation of Asia: 2 per Project.
    assert_eq!(EMPIRES[2].name, "Federation of Asia");
    s.players[2].empire = vec![id("Giant Tower"), id("Espionage Agency"), id("Zeppelin")];
    assert_eq!(s.score_of(2).combo, 4);
    // The Republic of Europe: 1 per General, on top of the General's own point.
    s.players[1].generals = 3;
    let europe = s.score_of(1);
    assert_eq!((europe.combo, europe.generals, europe.total), (3, 3, 6));
}

fn finished(s: &mut State) {
    s.phase = Phase::Over;
    s.scores = (0..s.seats()).map(|i| s.score_of(i)).collect();
}

#[test]
fn the_most_points_win() {
    let mut s = blank(3);
    s.players[0].empire = vec![id("Island of Avalon")];
    s.players[1].empire = vec![id("Time Travel")];
    s.players[2].empire = vec![id("Bionic Grafts")];
    assert!(s.winners().is_empty(), "nobody wins before the end");
    finished(&mut s);
    assert_eq!(s.winners(), vec![1]);
}

#[test]
fn ties_go_to_the_most_cards_then_the_most_characters_then_are_shared() {
    // Same points: a 2-point card against a General (1, and 1 more from the
    // Republic of Europe).
    let mut s = blank(2);
    s.players[0].empire = vec![id("Blackbeard's Treasure")];
    s.players[1].generals = 1;
    finished(&mut s);
    assert_eq!((s.scores[0].total, s.scores[1].total), (2, 2));
    assert_eq!(s.winners(), vec![0], "more finished cards");

    // Same points, same number of cards: 1 + 2 characters against 3 points.
    let mut s = blank(2);
    s.players[0].empire = vec![id("Human Cloning")];
    s.players[0].generals = 2;
    s.players[1].empire = vec![id("Cities of Gold")];
    finished(&mut s);
    assert_eq!((s.scores[0].total, s.scores[1].total), (3, 3));
    assert_eq!(s.winners(), vec![0], "more characters");

    // Everything level: they share the win.
    let mut s = blank(2);
    s.players[0].empire = vec![id("Recycling Plant")];
    s.players[1].empire = vec![twin("Recycling Plant")];
    finished(&mut s);
    assert_eq!(s.winners(), vec![0, 1]);
}

// ----- whole games ---------------------------------------------------------

/// A simple legal player: drafts what is first, builds two cards out of
/// three and recycles the rest into the Empire, puts every cube on the
/// first building with room (else the Empire) and uses what it holds.
fn bot(s: &State, seat: usize, turn: usize) -> Option<Action> {
    let p = &s.players[seat];
    match s.phase {
        Phase::Draft => {
            let card = *p.hand.get(turn % p.hand.len().max(1))?;
            p.picked.is_none().then_some(Action::Draft { card })
        }
        Phase::Planning => match p.drafted.first() {
            Some(&card) if !turn.is_multiple_of(3) => Some(Action::Build { card }),
            Some(&card) => {
                let res = card.def().recycle;
                let to = p
                    .buildings
                    .iter()
                    .find(|b| b.remaining().res[res.index()] > 0)
                    .map_or(Target::Empire, |b| Target::Card(b.card));
                Some(Action::Recycle { card, to })
            }
            None => (!p.ready).then_some(Action::Ready),
        },
        Phase::Production { step } if (step as usize) < Res::ALL.len() => {
            if p.choose {
                let token = if turn.is_multiple_of(2) { Token::General } else { Token::Financier };
                return Some(Action::Choose { token });
            }
            if p.ready {
                return None;
            }
            if p.pool == 0 {
                return Some(Action::Ready);
            }
            let res = Res::ALL[step as usize];
            let target = p
                .buildings
                .iter()
                .find(|b| b.remaining().res[res.index()] > 0)
                .map_or(Target::Empire, |b| Target::Card(b.card));
            Some(Action::Place { piece: Piece::Cube(res), target })
        }
        Phase::Production { .. } => {
            if p.ready {
                return None;
            }
            for b in &p.buildings {
                let left = b.remaining();
                let target = Target::Card(b.card);
                if p.krystallium > 0 && left.krystallium > 0 {
                    return Some(Action::Place { piece: Piece::Krystallium(None), target });
                }
                if p.krystallium > 0 {
                    if let Some(res) = Res::ALL.iter().find(|r| left.res[r.index()] > 0) {
                        return Some(Action::Place { piece: Piece::Krystallium(Some(*res)), target });
                    }
                }
                if p.generals > 0 && left.generals > 0 {
                    return Some(Action::Place { piece: Piece::General, target });
                }
                if p.financiers > 0 && left.financiers > 0 {
                    return Some(Action::Place { piece: Piece::Financier, target });
                }
            }
            Some(Action::Ready)
        }
        Phase::Over => None,
    }
}

/// A player that thinks a little: it drafts cheap, productive cards, builds
/// only what it can plausibly finish, aims recycling cubes and produced
/// cubes at the building closest to done, and spends characters and
/// Krystallium at the end of the round.
const OPEN_BUILDINGS: usize = 3;

fn smart_bot(s: &State, seat: usize, _turn: usize) -> Option<Action> {
    let p = &s.players[seat];
    let value = |id: CardId| -> f64 {
        let c = id.def();
        let icons: u32 = c.produces.iter().map(|&n| n as u32).sum::<u32>() + c.scaled.map_or(0, |_| 2);
        let points = c.vp as f64 + c.combo.map_or(0.0, |(_, v)| v as f64 * 3.0) + c.per_token.map_or(0.0, |(_, v)| v as f64 * 2.0);
        let late = (ROUNDS - s.round) as f64; // production is worth more early
        (icons as f64 * (1.0 + late) * 1.5 + points) / c.cost.total() as f64
    };
    // The building that needs `res` and is closest to done.
    let nearest = |res: Res| {
        p.buildings
            .iter()
            .filter(|b| b.remaining().res[res.index()] > 0)
            .min_by_key(|b| b.remaining().total())
            .map(|b| Target::Card(b.card))
    };
    match s.phase {
        Phase::Draft => {
            let card = *p.hand.iter().max_by(|a, b| value(**a).total_cmp(&value(**b)))?;
            p.picked.is_none().then_some(Action::Draft { card })
        }
        Phase::Planning => {
            // Build the best cards that are cheap enough for this stage.
            let limit = 4 + 2 * (s.round as u32 - 1);
            let mut by_value = p.drafted.clone();
            by_value.sort_by(|a, b| value(*b).total_cmp(&value(*a)));
            // Only a few buildings at a time, so the cubes can finish them.
            let wanted = by_value
                .iter()
                .copied()
                .find(|c| c.def().cost.total() <= limit)
                .filter(|_| p.buildings.len() < OPEN_BUILDINGS);
            match (wanted, by_value.first()) {
                (Some(card), _) => Some(Action::Build { card }),
                (None, Some(&card)) => {
                    let to = nearest(card.def().recycle).unwrap_or(Target::Empire);
                    Some(Action::Recycle { card, to })
                }
                (None, None) => (!p.ready).then_some(Action::Ready),
            }
        }
        Phase::Production { step } if (step as usize) < Res::ALL.len() => {
            if p.choose {
                return Some(Action::Choose { token: Token::General });
            }
            if p.ready {
                return None;
            }
            if p.pool == 0 {
                return Some(Action::Ready);
            }
            let res = Res::ALL[step as usize];
            let target = nearest(res).unwrap_or(Target::Empire);
            Some(Action::Place { piece: Piece::Cube(res), target })
        }
        Phase::Production { .. } => {
            if p.ready {
                return None;
            }
            let mut order: Vec<&Building> = p.buildings.iter().collect();
            order.sort_by_key(|b| b.remaining().total());
            for b in order {
                let left = b.remaining();
                let target = Target::Card(b.card);
                if p.generals > 0 && left.generals > 0 {
                    return Some(Action::Place { piece: Piece::General, target });
                }
                if p.financiers > 0 && left.financiers > 0 {
                    return Some(Action::Place { piece: Piece::Financier, target });
                }
                if p.krystallium > 0 && left.krystallium > 0 {
                    return Some(Action::Place { piece: Piece::Krystallium(None), target });
                }
                if p.krystallium > 0 {
                    if let Some(res) = Res::ALL.iter().find(|r| left.res[r.index()] > 0) {
                        return Some(Action::Place { piece: Piece::Krystallium(Some(*res)), target });
                    }
                }
            }
            Some(Action::Ready)
        }
        Phase::Over => None,
    }
}

fn play_out(n: usize, seed: u64) -> State {
    play_with(n, seed, bot)
}

fn play_smart(n: usize, seed: u64) -> State {
    play_with(n, seed, smart_bot)
}

fn play_with(n: usize, seed: u64, bot: fn(&State, usize, usize) -> Option<Action>) -> State {
    play_with_options(n, seed, Options::default(), bot)
}

fn play_with_options(n: usize, seed: u64, options: Options, bot: fn(&State, usize, usize) -> Option<Action>) -> State {
    let mut s = State::with_options(n, seed, options);
    let mut turn = seed as usize;
    for _ in 0..50_000 {
        if s.phase == Phase::Over {
            break;
        }
        let mut moved = false;
        for seat in 0..n {
            if let Some(action) = bot(&s, seat, turn) {
                let phase = s.phase;
                s.apply(seat, action.clone())
                    .unwrap_or_else(|e| panic!("seat {seat} {action:?} in {phase:?}: {e:?}"));
                moved = true;
                turn += 1;
                check_cards(&s);
                assert!(s.log.len() <= 24);
            }
        }
        assert!(moved, "deadlock in {:?}, round {}", s.phase, s.round);
    }
    assert_eq!(s.phase, Phase::Over, "the game never ended");
    s
}

#[test]
fn whole_games_end_for_every_player_count() {
    for n in MIN_PLAYERS..=Options::default().max_players() {
        for seed in 0..6 {
            for s in [play_out(n, seed), play_smart(n, seed)] {
                assert_eq!((s.round, s.scores.len()), (ROUNDS, n));
                assert!(!s.winners().is_empty());
                for p in &s.players {
                    assert!(p.hand.is_empty() && p.drafted.is_empty() && p.picked.is_none());
                }
                // Four rounds of drafting used this much of the deck.
                let dealt = ROUNDS as usize * n * hand_sizes(n, Options::default()).0;
                assert_eq!(s.deck.len(), DECK_SIZE - dealt);
                // Every score is the sum of its parts.
                for (i, score) in s.scores.iter().enumerate() {
                    assert_eq!(*score, s.score_of(i));
                    assert_eq!(score.total, score.gross + score.combo + score.generals + score.financiers);
                    assert_eq!(score.cards as usize, s.players[i].empire.len());
                }
            }
        }
    }
}

#[test]
fn games_are_reproducible_from_the_seed() {
    assert_eq!(play_out(4, 3).scores, play_out(4, 3).scores);
}

#[test]
fn a_sensible_player_builds_a_good_number_of_cards() {
    for n in MIN_PLAYERS..=Options::default().max_players() {
        let s = play_smart(n, 1);
        for (i, p) in s.players.iter().enumerate() {
            assert!(p.empire.len() >= 3, "seat {i} of {n} finished only {}", p.empire.len());
        }
        assert!(s.scores.iter().all(|sc| sc.total >= 4), "{:?}", s.scores);
    }
}

#[test]
fn nothing_works_once_the_game_is_over() {
    let mut s = play_out(2, 1);
    let card = CardId(0);
    for action in [
        Action::Ready,
        Action::Draft { card },
        Action::Build { card },
        Action::Choose { token: Token::General },
        Action::Place { piece: Piece::General, target: Target::Empire },
    ] {
        assert_eq!(s.apply(0, action), Err(Error::GameOver));
    }
}

#[test]
fn the_log_keeps_only_the_latest_events() {
    let s = play_smart(3, 2);
    assert_eq!(s.log.len(), 24);
    assert_ne!(s.log.first(), Some(&Event::RoundStarted(1)), "the oldest events were dropped");
}

// ----- views -----------------------------------------------------------------

#[test]
fn a_view_hides_what_other_players_hold() {
    let mut s = State::new(3, 8);
    let secret_pick = s.players[0].hand[0];
    s.apply(0, Action::Draft { card: secret_pick }).unwrap();

    // The owner sees their pick and their hand...
    let mine = s.view_for(0);
    assert_eq!(mine.picked, Some(secret_pick));
    assert_eq!(mine.hand, s.players[0].hand);

    // ...the others only see that a pick was made.
    let theirs = s.view_for(1);
    assert!(theirs.players[0].picked);
    assert!(!theirs.players[1].picked);
    assert_eq!(theirs.picked, None);
    assert_eq!(theirs.hand, s.players[1].hand, "your own hand, not seat 0's");
    assert_eq!(theirs.players[0].hand_size, 6);

    // In the JSON, the hidden parts of a player are counts and flags, never cards.
    let json = serde_json::to_value(&theirs).unwrap();
    let other = &json["players"][0];
    assert!(other["hand_size"].is_number());
    assert!(other["picked"].is_boolean());
    assert!(other["drafted"].is_number());
    assert!(other.get("hand").is_none());
}

#[test]
fn the_draft_area_stays_private_until_cards_are_built() {
    let mut s = to_planning(3, 8);
    let kept = s.players[0].drafted.clone();
    let seen = s.view_for(1);
    assert_eq!(seen.players[0].drafted, 7);
    assert!(seen.players[0].buildings.is_empty());
    assert_eq!(s.view_for(0).drafted, kept);
    // Once built, everybody sees the building and its progress.
    s.apply(0, Action::Build { card: kept[0] }).unwrap();
    let seen = s.view_for(2);
    assert_eq!(seen.players[0].drafted, 6);
    assert_eq!(seen.players[0].buildings.len(), 1);
    assert_eq!(seen.players[0].buildings[0].card, kept[0]);
}

#[test]
fn a_view_shows_the_table() {
    let mut s = blank(2);
    s.players[0].empire = vec![id("Island of Avalon")];
    s.players[0].generals = 1;
    let v = s.view_for(1);
    assert_eq!((v.you, v.round, v.phase), (1, 1, Phase::Planning));
    assert!(v.passes_to_next);
    assert_eq!(v.players[0].production, [3, 0, 1, 1, 0]);
    assert_eq!(v.players[0].points, 7 + 1);
    assert_eq!(v.players[0].generals, 1);
    assert_eq!(v.deck, s.deck.len());
    assert!(v.scores.is_empty() && v.winners.is_empty());
}

#[test]
fn scores_and_winners_appear_when_the_game_is_over() {
    let s = play_out(3, 4);
    let v = s.view_for(2);
    assert_eq!(v.phase, Phase::Over);
    assert_eq!(v.scores, s.scores);
    assert_eq!(v.winners, s.winners());
}

#[test]
fn views_survive_json_in_every_phase() {
    let mut s = State::new(3, 6);
    let check = |s: &State| {
        for seat in 0..s.seats() {
            let view = s.view_for(seat);
            let json = serde_json::to_string(&view).unwrap();
            let back: View = serde_json::from_str(&json).unwrap();
            assert_eq!(back, view, "{json}");
        }
    };
    check(&s);
    for _ in 0..DRAFT_PICKS {
        for seat in 0..3 {
            let card = s.players[seat].hand[0];
            s.apply(seat, Action::Draft { card }).unwrap();
        }
    }
    check(&s);
    for seat in 0..3 {
        for card in s.players[seat].drafted.clone() {
            s.apply(seat, Action::Build { card }).unwrap();
        }
        s.apply(seat, Action::Ready).unwrap();
    }
    check(&s);
    check(&play_out(3, 6));
    check(&play_with_options(7, 6, Options { ascension: true }, bot));
}

#[test]
fn actions_survive_json() {
    let actions = vec![
        Action::Draft { card: CardId(3) },
        Action::Build { card: CardId(4) },
        Action::Recycle { card: CardId(5), to: Target::Empire },
        Action::Scrap { card: CardId(6) },
        Action::Place { piece: Piece::Krystallium(Some(Res::Gold)), target: Target::Card(CardId(8)) },
        Action::Place { piece: Piece::Krystallium(None), target: Target::Card(CardId(7)) },
        Action::Place { piece: Piece::Cube(Res::Science), target: Target::Empire },
        Action::Place { piece: Piece::General, target: Target::Card(CardId(9)) },
        Action::Choose { token: Token::Financier },
        Action::Undo,
        Action::Ready,
    ];
    for action in actions {
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(serde_json::from_str::<Action>(&json).unwrap(), action, "{json}");
    }
}

#[test]
fn every_error_has_a_message() {
    for e in [
        Error::GameOver,
        Error::UnknownSeat,
        Error::WrongPhase,
        Error::AlreadyPicked,
        Error::NotInHand,
        Error::NotDrafted,
        Error::NotUnderConstruction,
        Error::InvalidTarget,
        Error::NothingToPlace,
        Error::CardsLeftToPlan,
        Error::MustChoose,
        Error::NothingToChoose,
        Error::NothingToUndo,
    ] {
        assert!(e.message().ends_with('.'), "{e:?}");
    }
}

/// A tuning aid for whoever swaps in other cards: how many cards and points
/// the two bots end with. Run it with
/// `cargo test -p wonderful-core economy_report -- --ignored --nocapture`.
#[test]
#[ignore]
fn economy_report() {
    for options in [Options::default(), Options { ascension: true }] {
        for n in MIN_PLAYERS..=options.max_players() {
            economy_line(n, options);
        }
    }
}

fn economy_line(n: usize, options: Options) {
    {
        let bots = [("dumb", bot as fn(&State, usize, usize) -> Option<Action>), ("smart", smart_bot)];
        for (name, play) in bots {
            let (mut cards, mut scores) = (vec![], vec![]);
            for seed in 0..20 {
                let s = play_with_options(n, seed, options, play);
                for (i, p) in s.players.iter().enumerate() {
                    cards.push(p.empire.len());
                    scores.push(s.scores[i].total as usize);
                }
            }
            let avg = |v: &Vec<usize>| v.iter().sum::<usize>() as f64 / v.len() as f64;
            let deck = if options.ascension { "C&A " } else { "base" };
            println!(
                "{deck} n={n} {name:5} cards avg {:.1} min {} max {} | score avg {:.1} min {} max {}",
                avg(&cards), cards.iter().min().unwrap(), cards.iter().max().unwrap(),
                avg(&scores), scores.iter().min().unwrap(), scores.iter().max().unwrap()
            );
        }
    }
}

// ----- Corruption & Ascension ------------------------------------------------

#[test]
fn the_ascension_deck_is_corruption_and_ascension() {
    let all: Vec<&Card> = catalogue().iter().filter(|c| c.ascension).collect();
    assert_eq!(all.len(), ASCENSION_DESIGNS);
    assert_eq!(all.iter().map(|c| c.copies as usize).sum::<usize>(), ASCENSION_DECK_SIZE);
    let designs = Kind::ALL.map(|k| all.iter().filter(|c| c.kind == k).count());
    let cards = Kind::ALL.map(|k| all.iter().filter(|c| c.kind == k).map(|c| c.copies as usize).sum::<usize>());
    assert_eq!(designs, [8, 6, 8, 9, 7]);
    assert_eq!(cards, [19, 12, 8, 10, 7]);
    assert_eq!((deck(false).len(), deck(true).len()), (DECK_SIZE, ASCENSION_DECK_SIZE));
    assert!(deck(true).iter().all(|c| c.def().ascension));
    // Corruption and duos are the expansion's alone.
    assert!(catalogue().iter().filter(|c| !c.ascension).all(|c| c.corruption == [0; 5] && c.duo.is_none()));
    assert!(all.iter().filter(|c| c.corruption != [0; 5]).count() >= 10);
    for c in &all {
        let ways = [c.vp > 0, c.combo.is_some(), c.per_token.is_some(), c.duo.is_some()];
        assert!(ways.iter().filter(|&&w| w).count() <= 1, "{} scores two ways", c.name);
    }
    // Checked against the rulebook's examples and the printed cards.
    let research_project = (Per::Kind(Kind::Research), Per::Kind(Kind::Project), 9);
    assert_eq!(id("Immortality").def().duo, Some(research_project));
    let structure_general = (Per::Kind(Kind::Structure), Per::Token(Token::General), 5);
    assert_eq!(id("High-Security Prison").def().duo, Some(structure_general));
    let patrol = id("Border Patrol").def();
    assert_eq!((patrol.produces, patrol.corruption, patrol.copies), ([0, 0, 0, 1, 0], [0, 0, 0, 0, 1], 3));
}

#[test]
fn the_expansion_deals_from_both_decks_for_up_to_seven() {
    let ca = Options { ascension: true };
    assert_eq!((Options::default().max_players(), ca.max_players()), (5, MAX_PLAYERS));
    for (n, sizes) in [(2, (8, 4)), (3, (6, 3)), (4, (6, 3)), (5, (6, 2)), (6, (6, 2)), (7, (5, 2))] {
        assert_eq!(hand_sizes(n, ca), sizes);
        let s = State::with_options(n, 3, ca);
        for p in &s.players {
            let from_ca = p.hand.iter().filter(|c| c.def().ascension).count();
            assert_eq!((p.hand.len() - from_ca, from_ca), sizes, "{n} players");
        }
        assert_eq!(s.deck.len(), DECK_SIZE - n * sizes.0);
        assert_eq!(s.ascension_deck.len(), ASCENSION_DECK_SIZE - n * sizes.1);
        check_cards(&s);
    }
    // Without the expansion nothing comes from its deck.
    let s = State::new(4, 3);
    assert!(s.ascension_deck.is_empty());
    assert!(s.players.iter().flat_map(|p| &p.hand).all(|c| !c.def().ascension));
}

#[test]
#[should_panic]
fn six_players_need_the_expansion() {
    State::with_options(6, 0, Options::default());
}

#[test]
#[should_panic]
fn eight_players_never_fit() {
    State::with_options(8, 0, Options { ascension: true });
}

#[test]
fn after_seven_picks_the_rest_of_a_bigger_hand_is_discarded() {
    // Two players with the expansion: 12 cards each, 7 kept, 5 discarded.
    let s = to_planning_with(2, 5, Options { ascension: true });
    for p in &s.players {
        assert_eq!(p.drafted.len(), DRAFT_PICKS);
        assert!(p.hand.is_empty());
    }
    assert_eq!(s.discard.len(), 2 * 5);
    assert!(s.players.iter().all(|p| p.empire_cubes == 0), "no recycling bonus");
    check_cards(&s);
}

#[test]
fn corruption_takes_production_down_but_never_below_zero() {
    let mut s = blank(2);
    // Noram States: 3 Materials and 1 Gold. Lawless Zone: +1 Materials, +1
    // Energy, and one Gold corrupted.
    s.players[0].empire = vec![id("Lawless Zone")];
    assert_eq!(Res::ALL.map(|r| s.production(0, r)), [4, 1, 0, 0, 0]);
    // Raiders: +2 Exploration and another Gold corrupted, which can't take
    // Gold below zero.
    s.players[0].empire.push(id("Raiders"));
    assert_eq!(Res::ALL.map(|r| s.production(0, r)), [4, 1, 0, 0, 2]);
}

#[test]
fn corrupted_production_decides_the_supremacy() {
    let mut s = blank(2);
    // Noram States' 3 Materials, less the Robot Factory's corrupted one, tie
    // the Republic of Europe's 2: nobody takes the character.
    s.players[0].empire = vec![id("Robot Factory")];
    start_production(&mut s);
    assert_eq!((s.players[0].produced, s.players[1].produced), (2, 2));
    assert!(s.log.contains(&Event::Supremacy { res: Res::Materials, seats: vec![] }));
    assert_eq!(s.players[0].financiers + s.players[1].financiers, 0);
}

#[test]
fn from_six_players_the_two_best_take_the_character() {
    let nobody: Vec<usize> = Vec::new();
    // One place below six players.
    assert_eq!(supremacy(&[3, 2, 1, 0, 0]), vec![0]);
    assert_eq!(supremacy(&[3, 3, 1, 0, 0]), nobody);
    // Two places from six on.
    assert_eq!(supremacy(&[1, 4, 0, 2, 0, 0]), vec![1, 3], "the two best");
    assert_eq!(supremacy(&[4, 4, 1, 0, 0, 0]), vec![0, 1], "two tied for first take both places");
    assert_eq!(supremacy(&[4, 2, 2, 0, 0, 0, 1]), vec![0], "a tie for second: only the first");
    assert_eq!(supremacy(&[4, 4, 4, 0, 0, 0]), nobody, "three tied for first: nobody");
    assert_eq!(supremacy(&[4, 0, 0, 0, 0, 0]), vec![0], "nobody second with nothing made");
    assert_eq!(supremacy(&[0; 7]), nobody);
}

#[test]
fn with_six_players_both_science_winners_choose() {
    let mut s = to_planning_with(6, 7, Options { ascension: true });
    for p in &mut s.players {
        p.drafted.clear();
        p.buildings.clear();
        p.empire.clear();
    }
    s.empires = (0..6).collect();
    // Science: seat 0 makes 4, seat 1 (the Republic of Europe) 1 + 2, seat 3
    // (the Panafrican Union) 2.
    s.players[0].empire = vec![id("Research Center"), twin("Research Center")];
    s.players[1].empire = vec![id("Neuroscience")]; // 1 per Research, itself included
    s.players[1].empire.push(id("Supercomputer")); // +1 Science, and a second Research
    start_production(&mut s);
    skip_to(&mut s, 2);
    assert_eq!((0..6).map(|i| s.production(i, Res::Science)).collect::<Vec<_>>(), [4, 4, 0, 2, 0, 0]);
    // Seats 0 and 1 tie for first: both take the character, each choosing.
    let choosing: Vec<usize> = (0..6).filter(|&i| s.players[i].choose).collect();
    assert_eq!(choosing, vec![0, 1]);
    assert!(s.log.contains(&Event::Supremacy { res: Res::Science, seats: vec![0, 1] }));
    let before = (s.players[0].generals, s.players[1].financiers);
    s.apply(0, Action::Choose { token: Token::General }).unwrap();
    s.apply(1, Action::Choose { token: Token::Financier }).unwrap();
    assert_eq!((s.players[0].generals, s.players[1].financiers), (before.0 + 1, before.1 + 1));
}

#[test]
fn duos_score_each_pair_of_both() {
    let mut s = blank(2);
    // The rulebook's example: Immortality, 9 VP per duo of a Research and a
    // Project. Two Research and one Project make one duo.
    s.players[0].empire = vec![id("Immortality"), id("Neuroscience"), id("Giant Tower")];
    assert_eq!(s.score_of(0).combo, 9);
    // A second Project makes a second duo.
    s.players[0].empire.push(id("Espionage Agency"));
    assert_eq!(s.score_of(0).combo, 18);
    // Characters pair up too: Orbital Station, 6 per Financier and General.
    let mut s = blank(2);
    s.players[1].empire = vec![id("Orbital Station")];
    s.players[1].generals = 2;
    s.players[1].financiers = 3;
    // Two duos, plus the Republic of Europe's 1 per General.
    assert_eq!(s.score_of(1).combo, 2 * 6 + 2);
}

#[test]
fn empires_are_dealt_at_random_from_the_box() {
    let dealt = |options: Options, n: usize| -> Vec<Vec<usize>> {
        (0..30).map(|seed| State::with_options(n, seed, options).empires).collect()
    };
    // The base game deals only its five, one per seat, not always the same.
    let base = dealt(Options::default(), 3);
    assert!(base.iter().flatten().all(|&e| !EMPIRES[e].ascension));
    assert!(base.iter().any(|e| e != &base[0]), "always the same Empires");
    for e in &base {
        let mut d = e.clone();
        d.sort_unstable();
        d.dedup();
        assert_eq!(d.len(), 3, "one Empire per seat");
    }
    // The expansion's two can be dealt too, and with seven players all are.
    let ca = Options { ascension: true };
    assert!(dealt(ca, 3).iter().flatten().any(|&e| EMPIRES[e].ascension));
    let mut all = State::with_options(7, 1, ca).empires;
    all.sort_unstable();
    assert_eq!(all, (0..7).collect::<Vec<_>>());
    // The view says who plays which.
    let s = State::with_options(4, 2, ca);
    let v = s.view_for(0);
    assert_eq!(v.players.iter().map(|p| p.empire_card).collect::<Vec<_>>(), s.empires);
    assert!(v.options.ascension);
}

#[test]
fn whole_games_end_with_the_expansion_for_every_player_count() {
    let ca = Options { ascension: true };
    for n in MIN_PLAYERS..=MAX_PLAYERS {
        for seed in 0..4 {
            for s in [play_with_options(n, seed, ca, bot), play_with_options(n, seed, ca, smart_bot)] {
                assert_eq!((s.round, s.scores.len()), (ROUNDS, n));
                assert!(!s.winners().is_empty());
                let (base, extra) = hand_sizes(n, ca);
                assert_eq!(s.deck.len(), DECK_SIZE - ROUNDS as usize * n * base);
                assert_eq!(s.ascension_deck.len(), ASCENSION_DECK_SIZE - ROUNDS as usize * n * extra);
                for (i, score) in s.scores.iter().enumerate() {
                    assert_eq!(*score, s.score_of(i));
                }
            }
        }
    }
}

#[test]
fn krystallium_and_characters_held_are_never_lost() {
    let mut s = blank(2);
    // Lunar Base has a Krystallium space, so the wrap-up step waits for seat 0.
    let base = id("Lunar Base");
    s.players[0].buildings = vec![Building { card: base, filled: Cost::default(), jokers: [0; 5] }];
    s.players[0].krystallium = 2;
    s.players[0].generals = 1;
    s.players[0].financiers = 1;
    start_production(&mut s);
    // In every step, a Krystallium put on a space and taken back, then Done.
    loop {
        let Phase::Production { step } = s.phase else { break };
        if !s.players[0].ready {
            s.apply(0, Action::Place { piece: Piece::Krystallium(None), target: Target::Card(base) }).unwrap();
            s.apply(0, Action::Undo).unwrap();
            if s.players[0].choose {
                s.apply(0, Action::Choose { token: Token::General }).unwrap();
            }
            s.apply(0, Action::Ready).unwrap();
        }
        if s.phase == (Phase::Production { step }) {
            skip_step(&mut s);
        }
        assert_eq!(s.players[0].krystallium, 2, "after step {step}");
    }
    assert_eq!((s.round, s.phase), (2, Phase::Draft));
    let p = &s.players[0];
    assert_eq!(p.krystallium, 2);
    assert!(p.generals >= 1 && p.financiers >= 1, "characters kept: {} {}", p.generals, p.financiers);
    assert_eq!(p.buildings[0].filled, Cost::default());
}
