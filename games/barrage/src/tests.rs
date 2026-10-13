use crate::*;

/// A tiny generator for the bots, so the tests need no `rand`.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
}

/// A game past the setup: seat `i` takes pair `i` and contract `i`.
fn started(players: usize, seed: u64) -> State {
    let mut s = State::new(players, seed);
    for seat in 0..players {
        s.apply(seat, Action::Choose { pair: 0, contract: 0 }).unwrap_or_else(|_| {
            let pair = s.setup.as_ref().unwrap().pairs.iter().position(|p| p.2.is_none()).unwrap();
            let contract = s.setup.as_ref().unwrap().contracts.iter().position(|c| c.1.is_none()).unwrap();
            s.apply(seat, Action::Choose { pair, contract }).unwrap();
        });
    }
    s
}

/// Gives `seat` a company and officer of our choosing.
fn make(s: &mut State, seat: usize, company: Company, officer: Officer) {
    s.players[seat].company = Some(company);
    s.players[seat].officer = Some(officer);
}

/// Hands the turn to `seat` with nothing waiting.
fn turn_of(s: &mut State, seat: usize) {
    s.todo.clear();
    s.phase = Phase::Actions;
    s.acted = false;
    s.turn = s.order.iter().position(|&x| x == seat).unwrap();
}

fn empty_map(s: &mut State) {
    s.dams = vec![None; DAM_SPACES];
    s.conduits = vec![None; CONDUITS.len()];
    s.powerhouses = vec![None; POWERHOUSES.len()];
    s.headstreams = [0; HEADSTREAMS];
}

fn basic(kind: Kind) -> Option<Tile> {
    Some(Tile::Basic(kind))
}

fn build(kind: Kind, target: Target) -> Action {
    Action::Build(Build { tile: basic(kind), copy: None, target, mixers: false, credits: 0 })
}

// ----- The map and the components -----

#[test]
fn every_river_runs_down_and_off_the_map() {
    for b in 0..BASINS.len() {
        let path = downstream(b);
        assert!(path.len() <= 4, "{path:?}");
        // Downhill only: never back up an area.
        for w in path.windows(2) {
            assert!(BASINS[w[0]].area <= BASINS[w[1]].area);
            assert!(w[1] > w[0]);
        }
        assert_eq!(BASINS[*path.last().unwrap()].flows_to, None);
    }
    assert_eq!(downstream(M2), [M2, H1, P2, B1]);
    assert_eq!(downstream(M1), [M1, P1, B1]);
}

#[test]
fn the_basins_have_the_spaces_the_rulebook_counts() {
    for (b, basin) in BASINS.iter().enumerate() {
        let dams = dams_of(b).count();
        let conduits = conduits_of(b).count();
        let ph = powerhouses_of(b).count();
        let expected = match b {
            M1..=M4 => (2, 2, 0),
            H1..=H3 => (2, 2, 2),
            P1..=P3 => (2, 2, 3),
            _ => (0, 0, 4),
        };
        assert_eq!((dams, conduits, ph), expected, "{}", basin.name);
    }
    // Every conduit leads to powerhouses elsewhere, lower or level.
    for c in CONDUITS {
        assert_ne!(c.basin, c.to);
        assert!(powerhouses_of(c.to).count() > 0);
        assert!((1..=5).contains(&c.value));
        assert!(BASINS[c.basin].area <= BASINS[c.to].area);
    }
    let values: Vec<u8> = (1..=5).map(|v| CONDUITS.iter().filter(|c| c.value == v).count() as u8).collect();
    assert_eq!(values, [2, 4, 7, 4, 3]);
    assert_eq!(POWERHOUSES.iter().filter(|p| p.paid).count(), 12);
}

#[test]
fn the_contracts_and_tiles_are_all_there() {
    assert_eq!(contracts_of(Tier::Starting).len(), 4);
    for tier in Tier::PRIVATE {
        let ids = contracts_of(tier);
        assert_eq!(ids.len(), 15, "{tier:?}");
        let energies: Vec<u8> = ids.iter().map(|&i| CONTRACTS[i as usize].energy).collect();
        let (lo, hi) = match tier {
            Tier::Green => (2, 4),
            Tier::Yellow => (5, 7),
            _ => (8, 10),
        };
        assert!(energies.iter().all(|e| (lo..=hi).contains(e)), "{tier:?}: {energies:?}");
    }
    assert_eq!(contracts_of(Tier::National).len(), 6);
    for id in 0..ADVANCED as u8 {
        assert!((1..=3).contains(&advanced_level(id)));
        assert!(!advanced_effect(id).is_empty());
    }
    assert_eq!(Tile::Advanced(14).name(), "Wild III");
    assert_eq!(Tile::Advanced(5).name(), "Base II");
}

#[test]
fn the_energy_track_pays_credits() {
    let table: Vec<u32> = (0..=31).map(energy_credits).collect();
    assert_eq!(&table[..8], &[3, 1, 2, 2, 3, 3, 3, 4]);
    assert_eq!((table[10], table[11], table[16], table[22], table[29], table[31]), (4, 5, 6, 7, 8, 8));
    assert_eq!((energy_section(5), energy_section(6), energy_section(17), energy_section(45)), (0, 1, 2, 5));
}

#[test]
fn incomes_come_with_the_second_fourth_and_fifth_piece() {
    assert!(incomes(Company::Germany, Kind::Base, 1).is_empty());
    assert_eq!(incomes(Company::Germany, Kind::Base, 2), [Reward::Vp(3)]);
    assert_eq!(incomes(Company::Germany, Kind::Base, 3), [Reward::Vp(3)]);
    assert_eq!(incomes(Company::Germany, Kind::Base, 4), [Reward::Vp(3), Reward::Machinery(2)]);
    assert_eq!(incomes(Company::Usa, Kind::Base, 5), [Reward::Rotate(1), Reward::Credits(6), Reward::Vp(7)]);
    assert_eq!((powerhouse_bonus(1), powerhouse_bonus(2), powerhouse_bonus(4)), (0, 1, 3));
}

// ----- Setup -----

#[test]
fn players_pick_a_company_and_contract_and_the_first_plays_last() {
    let mut s = State::new(3, 4);
    assert_eq!(s.phase, Phase::Setup);
    assert_eq!(s.actor(), Some(0));
    assert_eq!(s.apply(1, Action::Choose { pair: 0, contract: 0 }), Err(Error::NotYourTurn));
    s.apply(0, Action::Choose { pair: 1, contract: 2 }).unwrap();
    assert_eq!(s.apply(1, Action::Choose { pair: 1, contract: 0 }), Err(Error::NoSuchSpace));
    s.apply(1, Action::Choose { pair: 0, contract: 0 }).unwrap();
    s.apply(2, Action::Choose { pair: 2, contract: 1 }).unwrap();
    assert_eq!(s.phase, Phase::Actions);
    assert_eq!(s.round, 1);
    assert_eq!(s.order, [2, 1, 0]);
    assert_eq!(s.actor(), Some(2));
    for p in &s.players {
        assert!(p.company.is_some() && p.officer.is_some());
        assert_eq!(p.contracts.len(), 1);
        assert_eq!((p.vp, p.credits, p.excavators, p.mixers, p.engineers), (10, 6, 6, 4, 12));
        assert_eq!(p.tiles.contains(&Tile::Krylov), p.officer == Some(Officer::Krylov));
    }
    // The National Contracts: one fewer than the players.
    assert_eq!(s.nationals.len(), 2);
    // Three neutral dams, level 1, 2 and 3 from the top, each with a drop.
    let neutral: Vec<(usize, Dam)> =
        s.dams.iter().enumerate().filter_map(|(d, dam)| dam.map(|dam| (d, dam))).collect();
    assert_eq!(neutral.len(), 3);
    for (d, dam) in neutral {
        assert_eq!(dam.owner, None);
        assert!(!dam_paid(d), "on the lower space");
        let level = match BASINS[dam_basin(d)].area {
            Area::Mountains => 1,
            Area::Hills => 2,
            Area::Plains => 3,
        };
        assert_eq!((dam.level, dam.water), (level, 1));
    }
    // Round 1's drops wait on the headstreams.
    let expected: Vec<u8> = s.headstream_tiles.iter().map(|&t| HEADSTREAM_TILES[t][0]).collect();
    assert_eq!(s.headstreams.to_vec(), expected);
    assert_eq!(s.patents.iter().flatten().count(), 3);
    assert!(s.patents.iter().flatten().all(|&t| advanced_level(t) == 1));
}

// ----- Building -----

#[test]
fn a_base_costs_excavators_by_area_and_the_red_space_three_credits() {
    let mut s = started(2, 1);
    empty_map(&mut s);
    let seat = s.current();
    make(&mut s, seat, Company::Italy, Officer::Fiesler);
    turn_of(&mut s, seat);
    let cost = |s: &State, target| s.construction(seat, &Build { tile: basic(Kind::Base), copy: None, target, mixers: false, credits: 0 });
    // Lower (free) spaces: M1 5, H1 4, P1 3 Excavators; 1 Engineer.
    assert_eq!(cost(&s, Target::Base(M1 * 2 + 1)).unwrap(), Cost { engineers: 1, excavators: 5, mixers: 0, credits: 0 });
    assert_eq!(cost(&s, Target::Base(H1 * 2 + 1)).unwrap().excavators, 4);
    assert_eq!(cost(&s, Target::Base(P1 * 2 + 1)).unwrap().excavators, 3);
    assert_eq!(cost(&s, Target::Base(P1 * 2)).unwrap().credits, 3);
    // Wilhelm Adler: 3 anywhere.
    make(&mut s, seat, Company::Italy, Officer::Adler);
    assert_eq!(cost(&s, Target::Base(M1 * 2)).unwrap().excavators, 3);
    s.players[seat].excavators = 2;
    assert_eq!(cost(&s, Target::Base(M1 * 2)), Err(Error::Machineries));
}

#[test]
fn building_fills_the_wheel_and_the_company_board_spaces() {
    let mut s = started(2, 2);
    empty_map(&mut s);
    let seat = s.current();
    make(&mut s, seat, Company::Italy, Officer::Fiesler);
    turn_of(&mut s, seat);
    s.players[seat].credits = 30;
    s.players[seat].excavators = 30;
    s.players[seat].mixers = 30;
    s.apply(seat, build(Kind::Base, Target::Base(H2 * 2 + 1))).unwrap();
    let p = &s.players[seat];
    assert_eq!((p.engineers, p.constructions, p.excavators), (11, 1, 26));
    assert_eq!(p.wheel[0], Segment { tile: basic(Kind::Base), excavators: 4, mixers: 0 });
    assert_eq!(p.open, 1);
    assert!(!p.tiles.contains(&Tile::Basic(Kind::Base)));
    assert_eq!(s.dams[H2 * 2 + 1], Some(Dam { owner: Some(seat), level: 1, water: 0 }));

    // The other seat's turn now; back to ours: an Elevation, 2 Engineers.
    turn_of(&mut s, seat);
    assert_eq!(s.apply(seat, build(Kind::Base, Target::Base(H2 * 2))), Err(Error::NoTile));
    s.apply(seat, build(Kind::Elevation, Target::Elevation(H2 * 2 + 1))).unwrap();
    assert_eq!((s.players[seat].engineers, s.players[seat].mixers), (9, 27));
    assert_eq!(s.dams[H2 * 2 + 1].unwrap().level, 2);
    turn_of(&mut s, seat);
    // Conduit of value 4: 8 Excavators, 3 Engineers.
    s.apply(seat, build(Kind::Conduit, Target::Conduit(10))).unwrap();
    assert_eq!(CONDUITS[10].value, 4);
    assert_eq!((s.players[seat].engineers, s.players[seat].excavators), (6, 18));
    turn_of(&mut s, seat);
    // The fourth: 3 Engineers and 3 Credits; a Powerhouse, 2 Mixers.
    let before = s.players[seat].credits;
    s.apply(seat, build(Kind::Powerhouse, Target::Powerhouse(10))).unwrap();
    assert_eq!(s.players[seat].credits, before - 3);
    assert_eq!(s.players[seat].mixers, 25);
    turn_of(&mut s, seat);
    // No fifth space on the board this round.
    s.players[seat].tiles.push(Tile::Basic(Kind::Base));
    assert_eq!(s.apply(seat, build(Kind::Base, Target::Base(P1 * 2 + 1))), Err(Error::NoConstruction));
    // Two more turns of the wheel bring the Base tile and its Excavators back.
    s.players[seat].tiles.retain(|&t| t != Tile::Basic(Kind::Base));
    let excavators = s.players[seat].excavators;
    s.apply(seat, Action::Workshop { space: Spot::new(Section::Workshop, 0, false) }).unwrap();
    assert!(!s.players[seat].tiles.contains(&Tile::Basic(Kind::Base)));
    turn_of(&mut s, seat);
    s.apply(seat, Action::Workshop { space: Spot::new(Section::Workshop, 0, true) }).unwrap();
    let p = &s.players[seat];
    assert!(p.tiles.contains(&Tile::Basic(Kind::Base)));
    assert_eq!(p.excavators, excavators + 4);
}

#[test]
fn one_base_and_one_powerhouse_a_basin_each() {
    let mut s = started(2, 3);
    empty_map(&mut s);
    let seat = s.current();
    s.dams[H1 * 2] = Some(Dam { owner: Some(seat), level: 1, water: 0 });
    s.powerhouses[0] = Some(seat);
    assert_eq!(s.can_place(seat, Target::Base(H1 * 2 + 1)), Err(Error::OneABasin));
    assert_eq!(s.can_place(seat, Target::Powerhouse(1)), Err(Error::OneABasin));
    assert_eq!(s.can_place(1 - seat, Target::Base(H1 * 2 + 1)), Ok(()));
    assert_eq!(s.can_place(seat, Target::Elevation(H1 * 2 + 1)), Err(Error::NotYourDam));
    s.dams[H1 * 2].as_mut().unwrap().level = 3;
    assert_eq!(s.can_place(seat, Target::Elevation(H1 * 2)), Err(Error::Top));
    s.players[seat].built[Kind::Conduit.index()] = 5;
    assert_eq!(s.can_place(seat, Target::Conduit(0)), Err(Error::NoPiece));
}

#[test]
fn officers_change_what_building_costs() {
    let mut s = started(2, 5);
    empty_map(&mut s);
    let seat = s.current();
    turn_of(&mut s, seat);
    let conduit = |mixers, credits| Build { tile: basic(Kind::Conduit), copy: None, target: Target::Conduit(0), mixers, credits };
    // Value 5: 10 Excavators, which nobody has at the start.
    make(&mut s, seat, Company::Usa, Officer::Adler);
    assert_eq!(s.construction(seat, &conduit(false, 0)), Err(Error::Machineries));
    assert_eq!(s.construction(seat, &conduit(true, 0)), Err(Error::NotYourOfficer));
    // Jill McDowell: 5 Mixers instead (she has 4).
    make(&mut s, seat, Company::Usa, Officer::McDowell);
    s.players[seat].mixers = 5;
    assert_eq!(s.construction(seat, &conduit(true, 0)).unwrap().mixers, 5);
    // Solomon P. Jordan: 3 Credits each for the 4 missing Excavators.
    make(&mut s, seat, Company::Usa, Officer::Jordan);
    s.players[seat].credits = 12;
    assert_eq!(s.jordan_credits(seat, &conduit(false, 0)), Some(4));
    let cost = s.construction(seat, &conduit(false, 4)).unwrap();
    assert_eq!((cost.excavators, cost.credits), (6, 12));
    s.players[seat].credits = 11;
    assert_eq!(s.construction(seat, &conduit(false, 4)), Err(Error::Credits));
}

#[test]
fn split_payments() {
    assert_eq!(split_payment(5, 0, 3, 0, 2, false), Some((3, 0)));
    assert_eq!(split_payment(5, 0, 3, 0, 1, false), None);
    assert_eq!(split_payment(5, 0, 6, 0, 2, false), Some((3, 0)));
    assert_eq!(split_payment(2, 2, 1, 5, 0, true), Some((1, 3)));
    assert_eq!(split_payment(2, 2, 1, 2, 0, true), None);
    assert_eq!(split_payment(0, 4, 4, 0, 1, true), Some((3, 0)));
}

#[test]
fn incomes_are_paid_on_building_and_every_round() {
    let mut s = started(2, 6);
    empty_map(&mut s);
    let seat = s.current();
    make(&mut s, seat, Company::Germany, Officer::Fiesler);
    s.players[seat].built[Kind::Base.index()] = 1;
    s.dams[M1 * 2 + 1] = Some(Dam { owner: Some(seat), level: 1, water: 0 });
    turn_of(&mut s, seat);
    let vp = s.players[seat].vp;
    s.apply(seat, build(Kind::Base, Target::Base(P2 * 2 + 1))).unwrap();
    assert_eq!(s.players[seat].vp, vp + 3, "Germany's second Base: 3 VP");
}

#[test]
fn advanced_tiles_do_their_special_effect() {
    let mut s = started(2, 7);
    empty_map(&mut s);
    let seat = s.current();
    make(&mut s, seat, Company::Italy, Officer::Fiesler);
    s.players[seat].excavators = 20;
    s.players[seat].mixers = 20;
    s.players[seat].credits = 20;
    // I Base: a drop in each empty Dam of yours.
    s.dams[M1 * 2 + 1] = Some(Dam { owner: Some(seat), level: 2, water: 0 });
    s.dams[M2 * 2 + 1] = Some(Dam { owner: Some(seat), level: 2, water: 2 });
    s.dams[M3 * 2 + 1] = Some(Dam { owner: None, level: 1, water: 0 });
    s.players[seat].built[Kind::Base.index()] = 2;
    s.players[seat].tiles.push(Tile::Advanced(0));
    turn_of(&mut s, seat);
    let b = Build { tile: Some(Tile::Advanced(0)), copy: None, target: Target::Base(P1 * 2 + 1), mixers: false, credits: 0 };
    s.apply(seat, Action::Build(b)).unwrap();
    assert_eq!(s.dams[M1 * 2 + 1].unwrap().water, 1);
    assert_eq!(s.dams[M2 * 2 + 1].unwrap().water, 2);
    assert_eq!(s.dams[M3 * 2 + 1].unwrap().water, 0, "not neutral dams");
    assert_eq!(s.dams[P1 * 2 + 1].unwrap().water, 1, "the new one too");

    // I Powerhouse: no Engineers needed, even with none left.
    s.players[seat].tiles.push(Tile::Advanced(3));
    s.players[seat].engineers = 0;
    turn_of(&mut s, seat);
    assert!(s.can_build_without_engineers(seat));
    let b = Build { tile: Some(Tile::Advanced(3)), copy: None, target: Target::Powerhouse(7), mixers: false, credits: 0 };
    s.apply(seat, Action::Build(b)).unwrap();
    assert_eq!(s.powerhouses[7], Some(seat));

    // III Conduit: 2 energy per value, which can fulfill a Contract.
    s.players[seat].tiles.push(Tile::Advanced(12));
    s.players[seat].engineers = 12;
    s.players[seat].constructions = 0;
    s.players[seat].contracts = vec![4]; // green: 2 energy for 5 Credits
    let energy = s.players[seat].energy;
    turn_of(&mut s, seat);
    let b = Build { tile: Some(Tile::Advanced(12)), copy: None, target: Target::Conduit(16), mixers: false, credits: 0 };
    s.apply(seat, Action::Build(b)).unwrap();
    assert_eq!(CONDUITS[16].value, 1);
    assert_eq!(s.players[seat].energy, energy + 2);
    assert_eq!(s.todo.first().map(|t| t.task), Some(Task::Contract { energy: 2 }));
    assert_eq!(s.moves(seat).fulfill, [4]);
    let credits = s.players[seat].credits;
    s.apply(seat, Action::Fulfill { contract: Some(4) }).unwrap();
    assert_eq!(s.players[seat].credits, credits + 5);
    assert_eq!(s.players[seat].fulfilled, [4]);
}

#[test]
fn anton_krylov_copies_a_tile_on_the_wheel() {
    let mut s = started(2, 8);
    empty_map(&mut s);
    let seat = s.current();
    make(&mut s, seat, Company::Italy, Officer::Krylov);
    s.players[seat].tiles.push(Tile::Krylov);
    turn_of(&mut s, seat);
    s.apply(seat, build(Kind::Base, Target::Base(P1 * 2 + 1))).unwrap();
    turn_of(&mut s, seat);
    let b = Build { tile: Some(Tile::Krylov), copy: None, target: Target::Base(P2 * 2 + 1), mixers: false, credits: 0 };
    assert_eq!(s.apply(seat, Action::Build(b)), Err(Error::NoCopy));
    let b = Build { copy: basic(Kind::Base), ..b };
    s.apply(seat, Action::Build(b)).unwrap();
    assert_eq!(s.players[seat].built(Kind::Base), 2);
    assert_eq!(s.players[seat].wheel[1].tile, Some(Tile::Krylov));
}

// ----- Water and production -----

#[test]
fn water_fills_the_upper_dam_first_then_runs_on() {
    let mut s = started(2, 9);
    empty_map(&mut s);
    s.dams[M2 * 2] = Some(Dam { owner: Some(0), level: 1, water: 0 });
    s.dams[M2 * 2 + 1] = Some(Dam { owner: Some(1), level: 2, water: 1 });
    s.dams[P2 * 2 + 1] = Some(Dam { owner: None, level: 3, water: 1 });
    let seat = s.current();
    turn_of(&mut s, seat);
    // Five drops on M2's headstream.
    s.headstreams = [0, 5, 0, 0];
    s.flow(M2, 5, false);
    assert_eq!(s.dams[M2 * 2].unwrap().water, 1);
    assert_eq!(s.dams[M2 * 2 + 1].unwrap().water, 2);
    assert_eq!(s.dams[P2 * 2 + 1].unwrap().water, 3, "down M2→H1→P2");
}

#[test]
fn a_production_sends_drops_through_a_conduit_and_on_downstream() {
    let mut s = started(2, 10);
    empty_map(&mut s);
    let seat = s.current();
    let other = 1 - seat;
    make(&mut s, seat, Company::Italy, Officer::Adler);
    make(&mut s, other, Company::France, Officer::Jordan);
    // A neutral dam in M1 with 2 drops, the other's conduit M1→H1 (4), our
    // powerhouse in H1, our dam below it in H1.
    s.dams[M1 * 2 + 1] = Some(Dam { owner: None, level: 2, water: 2 });
    s.conduits[1] = Some(other);
    s.powerhouses[0] = Some(seat);
    s.players[seat].built[Kind::Powerhouse.index()] = 1;
    s.dams[H1 * 2 + 1] = Some(Dam { owner: Some(seat), level: 1, water: 0 });
    turn_of(&mut s, seat);
    let opts = s.moves(seat).productions;
    assert_eq!(opts.len(), 1);
    assert_eq!((opts[0].max, opts[0].value, opts[0].paid), (2, 4, true));
    let (credits, other_credits, other_vp) = (s.players[seat].credits, s.players[other].credits, s.players[other].vp);
    let production = Production { powerhouse: 0, conduit: 1, dam: M1 * 2 + 1, drops: 2 };
    s.apply(seat, Action::Produce { space: Some(Spot::new(Section::Turbine, 0, false)), production }).unwrap();
    // 2 × 4 + 2 for the space.
    assert_eq!(s.players[seat].energy, 10);
    assert_eq!(s.players[seat].credits, credits - 2);
    assert_eq!((s.players[other].credits, s.players[other].vp), (other_credits + 2, other_vp + 2));
    assert_eq!(s.dams[M1 * 2 + 1].unwrap().water, 0);
    // One drop stopped in our H1 dam, the other ran on.
    assert_eq!(s.dams[H1 * 2 + 1].unwrap().water, 1);
    // A Contract could be fulfilled: the starting one, if low enough.
    let todo = s.todo.first().map(|t| t.task);
    let fulfillable = s.fulfillable(seat, 10);
    assert_eq!(todo.is_some(), !fulfillable.is_empty());
    assert_eq!(s.placements.len(), 1);
    // The space is taken for the round.
    s.todo.clear();
    turn_of(&mut s, other);
    assert!(!s.spot_open(other, Spot::new(Section::Turbine, 0, false)));
}

#[test]
fn the_turbine_malus_can_leave_nothing_to_produce() {
    let mut s = started(2, 11);
    empty_map(&mut s);
    let seat = s.current();
    make(&mut s, seat, Company::Italy, Officer::Adler);
    s.dams[P3 * 2 + 1] = Some(Dam { owner: Some(seat), level: 1, water: 1 });
    s.conduits[19] = Some(seat); // P3→B2, value 1
    s.powerhouses[22] = Some(seat);
    turn_of(&mut s, seat);
    let production = Production { powerhouse: 22, conduit: 19, dam: P3 * 2 + 1, drops: 1 };
    let r = s.apply(seat, Action::Produce { space: Some(Spot::new(Section::Turbine, 4, false)), production });
    assert_eq!(r, Err(Error::TooLittle));
    // Viktor Fiesler makes it 4 - 2.
    make(&mut s, seat, Company::Italy, Officer::Fiesler);
    s.apply(seat, Action::Produce { space: Some(Spot::new(Section::Turbine, 4, false)), production }).unwrap();
    assert_eq!(s.players[seat].energy, 2);
}

#[test]
fn the_usa_gains_energy_from_water_through_its_powerhouses() {
    let mut s = started(2, 12);
    empty_map(&mut s);
    let usa = s.current();
    let other = 1 - usa;
    make(&mut s, usa, Company::Usa, Officer::Adler);
    make(&mut s, other, Company::Italy, Officer::Fiesler);
    for (i, ph) in [6, 9, 15].into_iter().enumerate() {
        s.powerhouses[ph] = Some(usa);
        s.players[usa].built[Kind::Powerhouse.index()] = i as u8 + 1;
    }
    // Three drops from M1 run through P1 and B1: 2 steps each.
    s.flow(M1, 3, false);
    assert_eq!(s.players[usa].energy, 6);
    // Produced into P1: not counted there, but in B1 below.
    s.dams[H1 * 2] = Some(Dam { owner: Some(other), level: 1, water: 1 });
    s.powerhouses[7] = Some(other);
    s.conduits[8] = Some(other); // H1→P1
    turn_of(&mut s, other);
    let production = Production { powerhouse: 7, conduit: 8, dam: H1 * 2, drops: 1 };
    s.apply(other, Action::Produce { space: Some(Spot::new(Section::Turbine, 2, false)), production }).unwrap();
    assert_eq!(s.players[usa].energy, 7);
}

#[test]
fn germany_produces_twice_and_italy_adds_three() {
    let mut s = started(2, 13);
    empty_map(&mut s);
    let seat = s.current();
    make(&mut s, seat, Company::Germany, Officer::Adler);
    s.players[seat].built[Kind::Powerhouse.index()] = 3;
    s.powerhouses[6] = Some(seat); // P1
    s.powerhouses[15] = Some(seat); // B1
    s.dams[M1 * 2] = Some(Dam { owner: Some(seat), level: 2, water: 2 });
    s.dams[P1 * 2] = Some(Dam { owner: Some(seat), level: 1, water: 0 });
    s.conduits[0] = Some(seat); // M1→P1, 5
    s.conduits[14] = Some(seat); // P1→B1, 3
    turn_of(&mut s, seat);
    let first = Production { powerhouse: 6, conduit: 0, dam: M1 * 2, drops: 1 };
    s.apply(seat, Action::Produce { space: Some(Spot::new(Section::Turbine, 2, false)), production: first }).unwrap();
    // 5 + 1 (3 powerhouses).
    assert_eq!(s.players[seat].energy, 6);
    // Skip any Contract, then the second production waits.
    while matches!(s.todo.first().map(|t| t.task), Some(Task::Contract { .. })) {
        s.apply(seat, Action::Fulfill { contract: None }).unwrap();
    }
    assert!(matches!(s.todo.first().map(|t| t.task), Some(Task::Produce { plain: true, .. })));
    let opts = s.moves(seat).productions;
    assert!(opts.iter().all(|o| o.powerhouse != 6));
    let second = Production { powerhouse: 15, conduit: 14, dam: P1 * 2, drops: 1 };
    s.apply(seat, Action::Produce { space: None, production: second }).unwrap();
    assert_eq!(s.players[seat].energy, 9, "3 more, no bonus");

    let mut s = started(2, 13);
    empty_map(&mut s);
    let seat = s.current();
    make(&mut s, seat, Company::Italy, Officer::Adler);
    s.players[seat].built[Kind::Powerhouse.index()] = 3;
    s.powerhouses[6] = Some(seat);
    s.dams[M1 * 2] = Some(Dam { owner: Some(seat), level: 2, water: 2 });
    s.conduits[0] = Some(seat);
    s.players[seat].contracts.clear();
    s.nationals.clear();
    turn_of(&mut s, seat);
    s.apply(seat, Action::Produce { space: Some(Spot::new(Section::Turbine, 2, false)), production: first }).unwrap();
    assert_eq!(s.players[seat].energy, 6 + 3);
}

#[test]
fn france_fulfills_contracts_with_three_less() {
    let mut s = started(2, 14);
    let seat = s.current();
    make(&mut s, seat, Company::France, Officer::Adler);
    s.players[seat].contracts = vec![34]; // red, 8
    s.nationals = vec![49]; // 13
    assert!(s.fulfillable(seat, 7).is_empty());
    s.players[seat].built[Kind::Powerhouse.index()] = 3;
    assert_eq!(s.fulfillable(seat, 5), [34]);
    assert_eq!(s.fulfillable(seat, 10), [34, 49]);
}

#[test]
fn water_management_pours_on_the_headstreams() {
    let mut s = started(2, 15);
    empty_map(&mut s);
    let seat = s.current();
    turn_of(&mut s, seat);
    let later = Spot::new(Section::Water, 0, false);
    assert_eq!(s.apply(seat, Action::Water { space: Some(later), drops: [1, 1, 1, 0] }), Err(Error::TooMuch));
    s.apply(seat, Action::Water { space: Some(later), drops: [0, 2, 0, 0] }).unwrap();
    assert_eq!(s.headstreams, [0, 2, 0, 0]);
    s.dams[M4 * 2 + 1] = Some(Dam { owner: Some(seat), level: 1, water: 0 });
    turn_of(&mut s, seat);
    let now = Spot::new(Section::Water, 1, false);
    s.apply(seat, Action::Water { space: Some(now), drops: [0, 0, 0, 1] }).unwrap();
    assert_eq!(s.dams[M4 * 2 + 1].unwrap().water, 1);
    assert_eq!(s.headstreams, [0, 2, 0, 0]);
}

// ----- The Management board -----

#[test]
fn spaces_are_closed_with_fewer_players_and_taken_once() {
    let mut s = started(2, 16);
    let seat = s.current();
    turn_of(&mut s, seat);
    assert_eq!(s.apply(seat, Action::Workshop { space: Spot::new(Section::Workshop, 1, false) }), Err(Error::Closed));
    let mut s3 = started(3, 16);
    let seat3 = s3.current();
    turn_of(&mut s3, seat3);
    s3.apply(seat3, Action::Workshop { space: Spot::new(Section::Workshop, 1, false) }).unwrap();
    assert_eq!(s3.players[seat3].credits, 4, "2 Credits for 2 turns");
    let next = s3.current();
    assert_eq!(s3.apply(next, Action::Workshop { space: Spot::new(Section::Workshop, 1, false) }), Err(Error::Taken));
    // The bank takes any number of Engineers, again and again.
    s3.apply(next, Action::Bank { engineers: 3 }).unwrap();
    let third = s3.current();
    s3.apply(third, Action::Bank { engineers: 1 }).unwrap();
    assert_eq!(s3.players[next].credits, 9);
    // A red space costs 3 Credits more.
    let mut s = started(2, 17);
    let seat = s.current();
    turn_of(&mut s, seat);
    s.apply(seat, Action::Buy { space: Spot::new(Section::Shop, 0, true), mixer: false }).unwrap();
    assert_eq!((s.players[seat].credits, s.players[seat].excavators), (1, 7));
}

#[test]
fn contracts_are_taken_refilled_at_the_end_of_the_turn_and_capped_at_three() {
    let mut s = started(4, 18);
    let seat = s.current();
    turn_of(&mut s, seat);
    let green = s.offers[0];
    let picks = vec![Pick { tier: 0, slot: 0 }, Pick { tier: 0, slot: 1 }];
    s.apply(seat, Action::Contracts { space: Spot::new(Section::Contracts, 2, false), picks }).unwrap();
    assert_eq!(s.players[seat].contracts.len(), 3);
    assert!(s.players[seat].contracts.contains(&green[0].unwrap()));
    assert!(s.offers[0].iter().all(Option::is_some), "refilled once the turn ended");
    assert_ne!(s.offers[0], green);
    turn_of(&mut s, seat);
    s.apply(seat, Action::Contracts { space: Spot::new(Section::Contracts, 0, false), picks: vec![Pick { tier: 2, slot: 0 }] })
        .unwrap();
    assert_eq!(s.actor(), Some(seat), "one has to go");
    assert!(s.moves(seat).discard);
    let first = s.players[seat].contracts[0];
    assert_eq!(s.apply(seat, Action::Skip), Err(Error::Optional));
    s.apply(seat, Action::Discard { contract: first }).unwrap();
    assert_eq!(s.players[seat].contracts.len(), 3);
    assert_ne!(s.actor(), Some(seat));
}

#[test]
fn the_patent_office_sells_advanced_tiles() {
    let mut s = started(2, 19);
    let seat = s.current();
    turn_of(&mut s, seat);
    let tile = s.patents[1].unwrap();
    s.apply(seat, Action::Patent { slot: 1 }).unwrap();
    assert!(s.players[seat].tiles.contains(&Tile::Advanced(tile)));
    assert_eq!((s.players[seat].credits, s.players[seat].engineers), (1, 10));
    assert_eq!(s.patents[1], None);
    let next = s.current();
    assert_eq!(s.apply(next, Action::Patent { slot: 1 }), Err(Error::NoTile2));
}

#[test]
fn mahiri_sekibo_copies_an_officer_and_then_acts() {
    let mut s = started(2, 20);
    empty_map(&mut s);
    let seat = s.current();
    make(&mut s, seat, Company::Italy, Officer::Sekibo);
    turn_of(&mut s, seat);
    s.apply(seat, Action::Copy { officer: Officer::Adler }).unwrap();
    assert_eq!(s.actor(), Some(seat), "still her turn");
    assert_eq!(s.apply(seat, Action::Copy { officer: Officer::Jordan }), Err(Error::CopyUsed));
    let cost = s.construction(seat, &Build { tile: basic(Kind::Base), copy: None, target: Target::Base(M1 * 2 + 1), mixers: false, credits: 0 });
    assert_eq!(cost.unwrap().excavators, 3);
    s.apply(seat, build(Kind::Base, Target::Base(M1 * 2 + 1))).unwrap();
    assert_eq!(s.players[seat].copied, None, "only for the turn");
    assert_eq!(s.players[seat].engineers, 10);
    // The second time costs 3 Credits.
    turn_of(&mut s, seat);
    let credits = s.players[seat].credits;
    s.apply(seat, Action::Copy { officer: Officer::Fiesler }).unwrap();
    assert_eq!(s.players[seat].credits, credits - 3);
    s.apply(seat, Action::Bank { engineers: 1 }).unwrap();
    turn_of(&mut s, seat);
    assert!(s.moves(seat).copy.is_empty(), "twice a round");
}

#[test]
fn a_player_without_engineers_passes() {
    let mut s = started(2, 21);
    let seat = s.current();
    turn_of(&mut s, seat);
    s.apply(seat, Action::Bank { engineers: 12 }).unwrap();
    let other = 1 - seat;
    assert_eq!(s.actor(), Some(other));
    s.apply(other, Action::Bank { engineers: 5 }).unwrap();
    // Out of Engineers: passed when the turn comes back. The other plays on
    // alone.
    assert!(s.players[seat].passed);
    assert!(s.log.contains(&Event::Passed { seat }));
    assert_eq!(s.actor(), Some(other));
    s.apply(other, Action::Bank { engineers: 5 }).unwrap();
    assert_eq!(s.actor(), Some(other));
}

// ----- Rounds and scoring -----

#[test]
fn the_round_ends_with_water_scoring_and_a_new_turn_order() {
    let mut s = started(3, 22);
    let order = s.order.clone();
    let bonus = s.bonus[0];
    // Give each a little energy: the first in turn order the most.
    s.players[order[0]].energy = 12;
    s.players[order[1]].energy = 5;
    s.players[order[2]].energy = 5;
    let vp: Vec<u32> = s.players.iter().map(|p| p.vp).collect();
    let credits: Vec<u32> = s.players.iter().map(|p| p.credits).collect();
    let drops: u8 = s.headstreams.iter().sum();
    let held_before: u8 = s.dams.iter().flatten().map(|d| d.water).sum();
    for i in 0..3 {
        let seat = s.current();
        let _ = i;
        s.apply(seat, Action::Bank { engineers: 12 }).unwrap();
    }
    assert_eq!(s.round, 2);
    assert!(s.log.contains(&Event::WaterFlow));
    let held: u8 = s.dams.iter().flatten().map(|d| d.water).sum();
    assert!(held <= held_before + drops);
    // 6 VP to the best, 1 each to the two tied second.
    let bonus_vp = s.bonus_vp(order[0], bonus);
    let _ = bonus_vp;
    assert!(s.log.contains(&Event::Scored { seat: order[0], vp: 6, why: Why::FirstEnergy }));
    assert!(s.log.contains(&Event::Scored { seat: order[1], vp: 1, why: Why::SecondEnergy }));
    assert!(s.log.contains(&Event::Scored { seat: order[2], vp: 1, why: Why::SecondEnergy }));
    assert_eq!(s.players[order[1]].vp, vp[order[1]] + 1);
    // Credits: 12 Bank + 5 for 12 energy, 3 for 5.
    assert_eq!(s.players[order[0]].credits, credits[order[0]] + 12 + 5);
    assert_eq!(s.players[order[1]].credits, credits[order[1]] + 12 + 3);
    // Least energy first, the tied two swapped.
    assert_eq!(s.order, [order[2], order[1], order[0]]);
    for p in &s.players {
        assert_eq!((p.energy, p.engineers, p.passed), (0, 12, false));
    }
    assert!(s.placements.is_empty());
}

#[test]
fn ties_for_first_share_both_bonuses_and_nothing_costs_three_vp() {
    let mut s = started(3, 23);
    let order = s.order.clone();
    s.players[order[0]].energy = 7;
    s.players[order[1]].energy = 7;
    let vp: Vec<u32> = s.players.iter().map(|p| p.vp).collect();
    for _ in 0..3 {
        let seat = s.current();
        s.apply(seat, Action::Bank { engineers: 12 }).unwrap();
    }
    assert!(s.log.contains(&Event::Scored { seat: order[0], vp: 4, why: Why::FirstEnergy }));
    assert!(s.log.contains(&Event::Scored { seat: order[1], vp: 4, why: Why::FirstEnergy }));
    assert!(s.log.contains(&Event::Scored { seat: order[2], vp: -3, why: Why::NoEnergy }));
    assert_eq!(s.players[order[2]].vp, vp[order[2]] - 3);
}

#[test]
fn the_bonus_tile_needs_six_energy_and_drops_four_a_section_behind() {
    let mut s = started(2, 24);
    let seat = 0;
    s.round = 3;
    s.players[seat].built[Kind::Powerhouse.index()] = 3;
    s.players[seat].energy = 5;
    assert_eq!(s.bonus_vp(seat, Bonus::Powerhouses), 0);
    s.players[seat].energy = 18;
    assert_eq!(s.bonus_vp(seat, Bonus::Powerhouses), 15);
    s.players[seat].energy = 12;
    assert_eq!(s.bonus_vp(seat, Bonus::Powerhouses), 11);
    s.players[seat].energy = 6;
    s.players[seat].built[Kind::Powerhouse.index()] = 1;
    assert_eq!(s.bonus_vp(seat, Bonus::Powerhouses), 0);
}

#[test]
fn the_objectives_count_what_they_say() {
    let mut s = started(3, 25);
    empty_map(&mut s);
    // Seat 0: a base on a red space in M1, a conduit M1→H1, a powerhouse in H1.
    s.dams[M1 * 2] = Some(Dam { owner: Some(0), level: 3, water: 0 });
    s.conduits[1] = Some(0);
    s.powerhouses[0] = Some(0);
    // Seat 1: a base in P1 with a powerhouse in B1 but somebody else's conduit.
    s.dams[P1 * 2 + 1] = Some(Dam { owner: Some(1), level: 1, water: 0 });
    s.conduits[14] = Some(2);
    s.powerhouses[15] = Some(1);
    let count = |s: &mut State, o, seat| {
        s.objective = o;
        s.objective_count(seat)
    };
    assert_eq!(count(&mut s, Objective::RedSpaces, 0), 2);
    assert_eq!(count(&mut s, Objective::RedSpaces, 1), 1);
    assert_eq!(count(&mut s, Objective::ConnectedBases, 0), 1);
    assert_eq!(count(&mut s, Objective::ConnectedBases, 1), 0);
    // Seat 0: 4 pieces in M1 (base + 2 elevations + conduit), 1 in the Hills.
    assert_eq!(count(&mut s, Objective::MostArea, 0), 4);
    assert_eq!(count(&mut s, Objective::LeastArea, 0), 0);
    assert_eq!(count(&mut s, Objective::Basins1, 0), 2);
    assert_eq!(count(&mut s, Objective::Basins3, 0), 1);
    // 15 / 10 / 5, ties share, nothing for nothing.
    s.objective = Objective::Basins1;
    assert_eq!(s.objective_vp(), [13, 13, 5]);
    s.objective = Objective::RedSpaces;
    assert_eq!(s.objective_vp(), [15, 10, 0]);
    s.powerhouses[19] = Some(1);
    assert_eq!(s.objective_vp(), [13, 13, 0]);
}

// ----- Whole games -----

/// A random but legal move for whoever has to act, from what the view
/// offers them.
fn random_move(s: &State, seat: usize, dice: &mut Dice) -> Action {
    let m = s.moves(seat);
    if m.choose {
        let setup = s.setup.as_ref().unwrap();
        let pairs: Vec<usize> = (0..setup.pairs.len()).filter(|&i| setup.pairs[i].2.is_none()).collect();
        let contracts: Vec<usize> = (0..setup.contracts.len()).filter(|&i| setup.contracts[i].1.is_none()).collect();
        return Action::Choose { pair: dice.pick(&pairs), contract: dice.pick(&contracts) };
    }
    if let Some(n) = m.machinery {
        return Action::Machinery { excavators: dice.below(n as usize + 1) as u8 };
    }
    if let Some((n, _)) = m.water {
        let mut drops = [0; HEADSTREAMS];
        for _ in 0..n {
            drops[dice.below(HEADSTREAMS)] += 1;
        }
        return Action::Water { space: None, drops };
    }
    if !m.places.is_empty() {
        return Action::Place { target: dice.pick(&m.places) };
    }
    if !m.fulfill.is_empty() {
        let best = m.fulfill.iter().copied().max_by_key(|&c| CONTRACTS[c as usize].energy);
        return Action::Fulfill { contract: best };
    }
    if m.discard {
        return Action::Discard { contract: s.players[seat].contracts[0] };
    }
    if !s.todo.is_empty() {
        let good: Vec<&ProductionOption> = m
            .productions
            .iter()
            .filter(|o| production_energy(o.max, o.value, m.fiesler, m.bonus) >= 1)
            .collect();
        if let Some(o) = good.first() {
            let production = Production { powerhouse: o.powerhouse, conduit: o.conduit, dam: o.dam, drops: o.max };
            return Action::Produce { space: None, production };
        }
        return Action::Skip;
    }
    if m.pass {
        if !m.builds.is_empty() && dice.below(2) == 0 {
            return Action::Build(dice.pick(&m.builds).build);
        }
        return Action::Pass;
    }
    // Produce when it pays, build often, otherwise the rest.
    let turbine: Vec<Spot> = m.spots.iter().copied().filter(|s| s.section == Section::Turbine).collect();
    if !turbine.is_empty() && dice.below(3) > 0 {
        let spot = turbine[0];
        let bonus = m.bonus + TURBINE_BONUS[spot.row as usize];
        let fee = if spot.def().unwrap().0.fee { FEE } else { 0 };
        let credits = s.players[seat].credits - fee;
        let opts: Vec<&ProductionOption> = m
            .productions
            .iter()
            .filter(|o| o.most(credits) > 0 && production_energy(o.most(credits), o.value, m.fiesler, bonus) >= 1)
            .collect();
        if let Some(o) = opts.iter().max_by_key(|o| o.most(credits) as u32 * o.value as u32) {
            let drops = o.most(credits);
            let production = Production { powerhouse: o.powerhouse, conduit: o.conduit, dam: o.dam, drops };
            return Action::Produce { space: Some(spot), production };
        }
    }
    if !m.builds.is_empty() && dice.below(4) > 0 {
        // Prefer powerhouses and conduits that connect, else anything.
        return Action::Build(dice.pick(&m.builds).build);
    }
    if !m.copy.is_empty() && dice.below(5) == 0 {
        return Action::Copy { officer: dice.pick(&m.copy) };
    }
    let others: Vec<Spot> = m.spots.iter().copied().filter(|s| s.section != Section::Turbine).collect();
    if !others.is_empty() && dice.below(3) > 0 {
        let spot = dice.pick(&others);
        return match spot.section {
            Section::Water => {
                let mut drops = [0; HEADSTREAMS];
                let n = if spot.row == 0 { 2 } else { 1 };
                for _ in 0..n {
                    drops[dice.below(HEADSTREAMS)] += 1;
                }
                Action::Water { space: Some(spot), drops }
            }
            Section::Workshop => Action::Workshop { space: spot },
            Section::Shop => Action::Buy { space: spot, mixer: dice.below(2) == 0 },
            Section::Contracts => {
                let mut picks = Vec::new();
                for t in 0..3u8 {
                    for slot in 0..2u8 {
                        if s.offers[t as usize][slot as usize].is_some() {
                            picks.push(Pick { tier: t, slot });
                        }
                    }
                }
                picks.truncate(CONTRACTS_TAKEN[spot.row as usize]);
                Action::Contracts { space: spot, picks }
            }
            Section::Patent => Action::Patent { slot: spot.row },
            _ => unreachable!(),
        };
    }
    Action::Bank { engineers: (dice.below(m.bank as usize) + 1) as u8 }
}

/// What must hold after every move.
fn check(s: &State) {
    for (d, dam) in s.dams.iter().enumerate() {
        if let Some(dam) = dam {
            assert!((1..=3).contains(&dam.level), "{d}: {dam:?}");
            assert!(dam.water <= 4, "{d}: {dam:?}");
            if dam.level < 3 {
                assert!(dam.water <= dam.level, "{d}: {dam:?}");
            }
        }
    }
    for (seat, p) in s.players.iter().enumerate() {
        for kind in Kind::ALL {
            assert!(p.built(kind) <= kind.pieces(), "{kind:?}");
        }
        let bases = s.dams.iter().flatten().filter(|d| d.owner == Some(seat)).count();
        let elevations: u8 = s.dams.iter().flatten().filter(|d| d.owner == Some(seat)).map(|d| d.level - 1).sum();
        assert_eq!(bases as u8, p.built(Kind::Base));
        assert_eq!(elevations, p.built(Kind::Elevation));
        assert_eq!(s.conduits.iter().filter(|&&c| c == Some(seat)).count() as u8, p.built(Kind::Conduit));
        assert_eq!(s.powerhouses.iter().filter(|&&c| c == Some(seat)).count() as u8, p.built(Kind::Powerhouse));
        assert!(p.contracts.len() <= MAX_CONTRACTS || s.todo.iter().any(|t| t.seat == seat && t.task == Task::Discard));
        // Every tile is either in the supply or on the wheel, once.
        let tiles = p.tiles.len() + p.wheel.iter().filter(|w| w.tile.is_some()).count();
        let owned = 4 + p.advanced() + usize::from(p.officer == Some(Officer::Krylov));
        assert_eq!(tiles, owned);
        assert!(p.wheel[p.open].is_empty(), "the open segment is always empty");
        assert!(p.engineers <= ENGINEERS);
    }
    assert!(s.actor().is_some() || s.phase == Phase::Over);
}

fn play_out(players: usize, seed: u64) -> State {
    let mut s = State::new(players, seed);
    let mut dice = Dice(seed ^ 0xBA22);
    let mut moves = 0;
    while s.phase != Phase::Over {
        let seat = s.actor().expect("somebody has to act");
        let action = random_move(&s, seat, &mut dice);
        if let Err(e) = s.apply(seat, action.clone()) {
            panic!("seed {seed}, move {moves}: {action:?} by {seat} refused: {e:?} ({})", e.message());
        }
        check(&s);
        moves += 1;
        assert!(moves < 5000, "the game doesn't end");
    }
    s
}

#[test]
fn whole_games_end_after_five_rounds() {
    for players in 2..=4 {
        for seed in 0..12 {
            let s = play_out(players, seed * 31 + players as u64);
            assert_eq!(s.round, 5);
            assert!(!s.winners().is_empty());
            assert!(s.log.contains(&Event::GameOver));
            let view = s.view_for(0);
            assert_eq!(view.winners, s.winners());
            assert_eq!(view.moves, Moves::default());
        }
    }
}

#[test]
fn bots_actually_build_and_produce() {
    let mut built = 0;
    let mut produced = 0;
    let mut fulfilled = 0;
    for seed in 0..10 {
        let s = play_out(3, seed);
        for p in &s.players {
            built += p.built.iter().map(|&b| b as u32).sum::<u32>();
            fulfilled += p.fulfilled.len();
        }
        produced += s.log.len();
    }
    assert!(built > 60, "{built}");
    assert!(fulfilled > 5, "{fulfilled}");
    assert!(produced > 0);
}

#[test]
fn the_view_survives_json() {
    let mut s = State::new(3, 40);
    let mut dice = Dice(1);
    for _ in 0..80 {
        let seat = s.actor().unwrap();
        let a = random_move(&s, seat, &mut dice);
        s.apply(seat, a).unwrap();
    }
    for seat in 0..3 {
        let v = s.view_for(seat);
        let json = serde_json::to_string(&v).unwrap();
        let back: View = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }
    let actions = [
        Action::Choose { pair: 1, contract: 0 },
        Action::Build(Build { tile: Some(Tile::Advanced(7)), copy: None, target: Target::Conduit(3), mixers: false, credits: 2 }),
        Action::Produce { space: Some(Spot::new(Section::Turbine, 1, true)), production: Production { powerhouse: 1, conduit: 2, dam: 3, drops: 2 } },
        Action::Water { space: None, drops: [1, 0, 0, 1] },
        Action::Contracts { space: Spot::new(Section::Contracts, 2, false), picks: vec![Pick { tier: 1, slot: 0 }] },
        Action::Copy { officer: Officer::Krylov },
        Action::Fulfill { contract: None },
    ];
    for a in actions {
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Action>(&json).unwrap(), a);
    }
}

/// `cargo test -p barrage-core game_report -- --ignored --nocapture`: what
/// random bots do in whole games, to see the rules at work.
#[test]
#[ignore]
fn game_report() {
    for players in 2..=4 {
        let (mut vp, mut built, mut fulfilled, mut produced, mut energy) = (0, 0, 0, 0, 0);
        let games = 20;
        for seed in 0..games {
            let s = play_out(players, seed);
            for p in &s.players {
                vp += p.vp;
                built += p.built.iter().map(|&b| b as u32).sum::<u32>();
                fulfilled += p.fulfilled.len();
            }
            produced += s.log.iter().filter(|e| matches!(e, Event::Produced { .. })).count();
            energy += s.players.iter().map(|p| p.energy).sum::<u32>();
        }
        let n = (games * players as u64) as f64;
        println!(
            "{players} players: {:.1} VP, {:.1} pieces, {:.1} contracts a player; {:.1} productions logged in round 5, {:.1} energy in round 5",
            vp as f64 / n,
            built as f64 / n,
            fulfilled as f64 / n,
            produced as f64 / games as f64,
            energy as f64 / n
        );
    }
}
