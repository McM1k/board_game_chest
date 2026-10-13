//! The game state and its rules.
//!
//! * **Setup.** Each seat in turn picks one of the dealt Company and
//!   Executive Officer pairs and a Starting Contract; the turn order is the
//!   reverse of the picking order.
//! * **Five rounds**, each: water on the Headstreams (rounds 1 to 4) and the
//!   income of the Company boards; then the **actions**, one per turn in
//!   turn order, placing Engineers on the Company board (construction) or on
//!   the Management board, until everybody has passed; then the **water
//!   flow** from the Headstreams; then the **scoring** (6 and 2 VP to the
//!   two best producers, Credits from the Energy Track, the round's Bonus
//!   tile); then the end of the round (new turn order, Engineers back, the
//!   Patent Office refilled).
//! * **The end**, after round 5: the Objective tile, 1 VP per 5 Credits and
//!   Machineries in the supply, 1 VP per drop held by your Dams. The most VP
//!   win; a tie goes to whoever produced the most in the last round.
//!
//! What a turn or an income gives that needs a decision (Machineries of
//! your choice, drops on Headstreams of your choice, a free building, a
//! Contract to fulfill, a second production...) waits in [`State::todo`]
//! until its owner settles it.
//!
//! The rules are pure: the only randomness, the shuffles, comes from the
//! seed the game is created with.

use crate::data::*;
use crate::map::*;
use serde::{Deserialize, Serialize};

pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 4;
/// How many events [`State::log`] keeps.
const LOG_LEN: usize = 60;

/// A dam on the map: a Base with 0 to 2 Elevations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dam {
    /// `None`: a neutral Dam.
    pub owner: Option<usize>,
    pub level: u8,
    pub water: u8,
}

/// A segment of the Construction Wheel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub tile: Option<Tile>,
    pub excavators: u32,
    pub mixers: u32,
}

impl Segment {
    pub fn is_empty(&self) -> bool {
        self.tile.is_none() && self.excavators == 0 && self.mixers == 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub company: Option<Company>,
    pub officer: Option<Officer>,
    pub vp: u32,
    pub credits: u32,
    pub excavators: u32,
    pub mixers: u32,
    /// Engineers still in hand this round.
    pub engineers: u8,
    /// Technology tiles in the supply (not on the wheel).
    pub tiles: Vec<Tile>,
    pub wheel: Vec<Segment>,
    /// The open segment of the wheel.
    pub open: usize,
    /// Pieces built, per [`Kind`].
    pub built: [u8; 4],
    /// Contracts face up, waiting to be fulfilled.
    pub contracts: Vec<u8>,
    pub fulfilled: Vec<u8>,
    /// Energy produced this round.
    pub energy: u32,
    /// Construction spaces of the Company board used this round.
    pub constructions: u8,
    /// Mahiri Sekibo's action used this round.
    pub sekibo: u8,
    /// Mahiri Sekibo: the officer copied for this turn.
    pub copied: Option<Officer>,
    pub passed: bool,
}

impl Player {
    fn new() -> Player {
        Player {
            company: None,
            officer: None,
            vp: START_VP,
            credits: START_CREDITS,
            excavators: START_EXCAVATORS,
            mixers: START_MIXERS,
            engineers: ENGINEERS,
            tiles: Kind::ALL.iter().map(|&k| Tile::Basic(k)).collect(),
            wheel: vec![Segment::default(); WHEEL],
            open: 0,
            built: [0; 4],
            contracts: Vec::new(),
            fulfilled: Vec::new(),
            energy: 0,
            constructions: 0,
            sekibo: 0,
            copied: None,
            passed: false,
        }
    }

    /// Has this officer's ability: their own, or Mahiri's copy this turn.
    pub fn has(&self, officer: Officer) -> bool {
        self.officer == Some(officer) || self.copied == Some(officer)
    }

    pub fn built(&self, kind: Kind) -> u8 {
        self.built[kind.index()]
    }

    /// The company's own ability is on (third Powerhouse).
    pub fn ability(&self, company: Company) -> bool {
        self.company == Some(company) && self.built(Kind::Powerhouse) >= ABILITY_AT
    }

    /// Advanced tiles owned, in the supply and on the wheel.
    pub fn advanced(&self) -> usize {
        let wheel = self.wheel.iter().filter(|s| s.tile.is_some_and(Tile::is_advanced)).count();
        wheel + self.tiles.iter().filter(|t| t.is_advanced()).count()
    }

    /// Tiles on the wheel, for Anton Krylov to copy.
    pub fn on_wheel(&self) -> Vec<Tile> {
        let mut tiles: Vec<Tile> = self.wheel.iter().filter_map(|s| s.tile).filter(|&t| t != Tile::Krylov).collect();
        tiles.dedup();
        tiles
    }
}

/// A space of the Management board (or Mahiri Sekibo's tile).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Spot {
    pub section: Section,
    pub row: u8,
    pub right: bool,
}

impl Spot {
    pub fn new(section: Section, row: u8, right: bool) -> Spot {
        Spot { section, row, right }
    }

    /// Its Engineers and fee (`None`: no such space).
    pub fn def(self) -> Option<(SpaceDef, Row)> {
        let row = *rows(self.section).get(self.row as usize)?;
        let def = if self.right { row.right? } else { row.left };
        Some((def, row))
    }
}

/// Engineers on an action space this round.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub spot: Spot,
    pub seat: usize,
    pub engineers: u8,
}

/// What a piece is built on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Target {
    /// A dam space.
    Base(usize),
    /// A dam space holding one of your Dams.
    Elevation(usize),
    /// A conduit space.
    Conduit(usize),
    /// A powerhouse space.
    Powerhouse(usize),
}

impl Target {
    pub fn kind(self) -> Kind {
        match self {
            Target::Base(_) => Kind::Base,
            Target::Elevation(_) => Kind::Elevation,
            Target::Conduit(_) => Kind::Conduit,
            Target::Powerhouse(_) => Kind::Powerhouse,
        }
    }

    pub fn space(self) -> usize {
        match self {
            Target::Base(s) | Target::Elevation(s) | Target::Conduit(s) | Target::Powerhouse(s) => s,
        }
    }

    pub fn basin(self) -> usize {
        match self {
            Target::Base(s) | Target::Elevation(s) => dam_basin(s),
            Target::Conduit(c) => CONDUITS[c].basin,
            Target::Powerhouse(p) => POWERHOUSES[p].basin,
        }
    }

    /// On a red building space (Elevations never pay for it).
    pub fn paid(self) -> bool {
        match self {
            Target::Base(s) => dam_paid(s),
            Target::Powerhouse(p) => POWERHOUSES[p].paid,
            _ => false,
        }
    }
}

/// A construction with the Company board.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Build {
    /// The tile from the supply put on the wheel (`None`: Mahiri Sekibo
    /// copying Anton Krylov, who needs none).
    pub tile: Option<Tile>,
    /// The tile on the wheel copied, with Anton Krylov.
    pub copy: Option<Tile>,
    pub target: Target,
    /// Jill McDowell: a Conduit paid with Concrete Mixers.
    #[serde(default)]
    pub mixers: bool,
    /// Solomon P. Jordan: Machineries paid with 3 Credits each instead.
    #[serde(default)]
    pub credits: u8,
}

/// A production: drops from a Dam, through a Conduit, to a Powerhouse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Production {
    pub powerhouse: usize,
    pub conduit: usize,
    pub dam: usize,
    pub drops: u8,
}

/// A face-up private Contract on the Management board.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pick {
    /// 0 green, 1 yellow, 2 red.
    pub tier: u8,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Setup: a Company and Executive Officer pair, and a Starting Contract.
    Choose { pair: usize, contract: usize },
    /// Construction.
    Build(Build),
    /// Turbine Station (`space`), or a production waiting in the todo list.
    Produce { space: Option<Spot>, production: Production },
    /// Water Management (`space`), or drops a reward lets you place: how
    /// many on each Headstream.
    Water { space: Option<Spot>, drops: [u8; HEADSTREAMS] },
    Bank { engineers: u8 },
    Workshop { space: Spot },
    /// Machinery Shop; `mixer` picks the Concrete Mixer where there's a
    /// choice.
    Buy { space: Spot, mixer: bool },
    /// Contract Office.
    Contracts { space: Spot, picks: Vec<Pick> },
    /// Patent Office: one of the three tiles.
    Patent { slot: u8 },
    /// Mahiri Sekibo: copy an officer for this turn, then act.
    Copy { officer: Officer },
    /// Only with no Engineers left.
    Pass,
    /// Settling the todo list.
    Machinery { excavators: u8 },
    Place { target: Target },
    Fulfill { contract: Option<u8> },
    Discard { contract: u8 },
    /// Leave an optional todo (a free building, a production) undone.
    Skip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// Picking Companies and Starting Contracts.
    Setup,
    /// Income decisions at the start of a round.
    Income,
    Actions,
    Over,
}

/// Something waiting for a player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Task {
    /// A reward: applied at once unless it needs a decision.
    Reward(Reward),
    /// This much energy was produced: one Contract may be fulfilled.
    Contract { energy: u32 },
    /// More than three Contracts face up: discard one.
    Discard,
    /// A production to make, or skip: II Powerhouse's (+2), or Germany's
    /// second (`plain`: no bonus at all, and another Powerhouse than
    /// `not`).
    Produce { bonus: i32, plain: bool, not: Option<usize> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Todo {
    pub seat: usize,
    pub task: Task,
}

/// What the setup deals out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setup {
    /// Company and officer pairs, and who took each.
    pub pairs: Vec<(Company, Officer, Option<usize>)>,
    /// Starting Contracts, and who took each.
    pub contracts: Vec<(u8, Option<usize>)>,
    /// The seat picking now.
    pub next: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    GameOver,
    UnknownSeat,
    NotYourTurn,
    WrongPhase,
    Settle,
    NoSuchSpace,
    Closed,
    Taken,
    Engineers,
    Credits,
    Machineries,
    NoTile,
    WrongTile,
    NoCopy,
    NoPiece,
    Occupied,
    OneABasin,
    NotYourDam,
    Top,
    Area,
    TooBig,
    NoPowerhouse,
    NoConduit,
    NotConnected,
    NoWater,
    TooLittle,
    NoContract,
    TooMuch,
    NoTile2,
    CopyUsed,
    NotSekibo,
    NotYourOfficer,
    CantPass,
    NoSuchContract,
    Optional,
    SamePowerhouse,
    NoConstruction,
}

impl Error {
    /// What the error means, in words for the player.
    pub fn message(self) -> &'static str {
        match self {
            Error::GameOver => "The game is over.",
            Error::UnknownSeat => "You aren't seated in this game.",
            Error::NotYourTurn => "It isn't your turn.",
            Error::WrongPhase => "You can't do that right now.",
            Error::Settle => "Finish what your last action gave you first.",
            Error::NoSuchSpace => "There's no such action space.",
            Error::Closed => "That action space isn't used with this many players.",
            Error::Taken => "Somebody's Engineers are already there this round.",
            Error::Engineers => "You don't have enough Engineers for that.",
            Error::Credits => "You don't have enough Credits for that.",
            Error::Machineries => "You don't have the Machineries for that.",
            Error::NoTile => "You don't have that Technology tile in your supply.",
            Error::WrongTile => "That Technology tile can't build this.",
            Error::NoCopy => "Pick a tile on your Construction Wheel to copy.",
            Error::NoPiece => "You have no pieces of that kind left.",
            Error::Occupied => "That building space is taken.",
            Error::OneABasin => "You already have one of those in that basin.",
            Error::NotYourDam => "Elevations go on your own Dams.",
            Error::Top => "That Dam is already level 3.",
            Error::Area => "That reward can't build there.",
            Error::TooBig => "That Conduit's value is too high for this reward.",
            Error::NoPowerhouse => "That isn't one of your Powerhouses.",
            Error::NoConduit => "There's no Conduit there.",
            Error::NotConnected => "That Conduit doesn't connect that Dam to that Powerhouse.",
            Error::NoWater => "That Dam doesn't hold enough water (and must be yours or neutral).",
            Error::TooLittle => "That production makes no energy.",
            Error::NoContract => "No Contract can be fulfilled with that much energy.",
            Error::TooMuch => "That's more than you can take.",
            Error::NoTile2 => "There's no tile there.",
            Error::CopyUsed => "You've already copied an officer this turn.",
            Error::NotSekibo => "Only Mahiri Sekibo can do that.",
            Error::NotYourOfficer => "Your Executive Officer can't do that.",
            Error::CantPass => "You can only pass with no Engineers left.",
            Error::NoSuchContract => "There's no such Contract.",
            Error::Optional => "That has to be done.",
            Error::SamePowerhouse => "The second production uses another Powerhouse.",
            Error::NoConstruction => "Your Company board has no construction space left this round.",
        }
    }
}

/// Why VP were scored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Why {
    FirstEnergy,
    SecondEnergy,
    NoEnergy,
    Bonus,
    Objective,
    Resources,
    Water,
    /// An Advanced Technology tile's VP.
    Tile,
}

/// Something that happened, kept (briefly) for the log. Seats are indices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    Round(u8),
    Chose { seat: usize, company: Company, officer: Officer, contract: u8 },
    Built { seat: usize, target: Target, tile: Option<Tile> },
    Produced { seat: usize, production: Production, energy: u32, paid_to: Option<usize> },
    /// Energy without a production (II Base, III Conduit).
    Generated { seat: usize, energy: u32 },
    Fulfilled { seat: usize, contract: u8 },
    TookContracts { seat: usize, contracts: Vec<u8> },
    Discarded { seat: usize, contract: u8 },
    Watered { seat: usize, drops: [u8; HEADSTREAMS], now: bool },
    Bank { seat: usize, credits: u32 },
    Rotated { seat: usize, turns: u8 },
    Bought { seat: usize, excavators: u32, mixers: u32 },
    Patent { seat: usize, tile: Tile },
    Copied { seat: usize, officer: Officer },
    Passed { seat: usize },
    Gained { seat: usize, reward: Reward },
    /// USA: drops through their Powerhouses.
    Flowed { seat: usize, energy: u32 },
    WaterFlow,
    Scored { seat: usize, vp: i32, why: Why },
    Credits { seat: usize, credits: u32 },
    GameOver,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub players: Vec<Player>,
    pub phase: Phase,
    /// 1 to 5 (0 during the setup).
    pub round: u8,
    /// Seats in turn order.
    pub order: Vec<usize>,
    /// Whose turn it is: an index into `order`.
    pub turn: usize,
    /// The player whose turn it is has made their action.
    pub acted: bool,
    pub todo: Vec<Todo>,
    pub setup: Option<Setup>,
    pub dams: Vec<Option<Dam>>,
    pub conduits: Vec<Option<usize>>,
    pub powerhouses: Vec<Option<usize>>,
    /// Drops waiting on each Headstream.
    pub headstreams: [u8; HEADSTREAMS],
    /// The Headstream tile on each (an index into [`HEADSTREAM_TILES`]).
    pub headstream_tiles: [usize; HEADSTREAMS],
    /// Face-up private Contracts: green, yellow, red, two of each.
    pub offers: [[Option<u8>; 2]; 3],
    /// The three piles, the next tile last.
    pub piles: [Vec<u8>; 3],
    /// National Contracts still on the board.
    pub nationals: Vec<u8>,
    /// The Patent Office's three tiles.
    pub patents: [Option<u8>; 3],
    /// The Advanced tiles still in the I, II and III piles, the next last.
    pub tech: [Vec<u8>; 3],
    /// The Bonus tile of each round.
    pub bonus: Vec<Bonus>,
    pub objective: Objective,
    pub placements: Vec<Placement>,
    pub log: Vec<Event>,
    rng: u64,
}

/// SplitMix64: a tiny seeded generator, so the rules need no `rand`.
fn next_random(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn shuffle<T>(items: &mut [T], rng: &mut u64) {
    for i in (1..items.len()).rev() {
        let j = (next_random(rng) % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
}

/// The energy of a production: drops times the Conduit's value (at least 4
/// with Viktor Fiesler), plus the bonuses.
pub fn production_energy(drops: u8, value: u8, fiesler: bool, bonus: i32) -> i32 {
    let mut base = drops as u32 * value as u32;
    if fiesler && base < 4 {
        base = 4;
    }
    base as i32 + bonus
}

/// Splits a payment of `e` Excavators and `m` Concrete Mixers between
/// what the player has, `k` of them paid in Credits instead (Solomon P.
/// Jordan), and either kind standing in for the other with `swap` (II
/// wild tile). `None`: it can't be paid.
pub fn split_payment(e: u32, m: u32, have_e: u32, have_m: u32, k: u32, swap: bool) -> Option<(u32, u32)> {
    let total = (e + m).checked_sub(k)?;
    if swap {
        let mut pe = e.min(have_e).min(total);
        let mut pm = m.min(have_m).min(total - pe);
        let mut rest = total - pe - pm;
        let more_e = rest.min(have_e - pe);
        pe += more_e;
        rest -= more_e;
        let more_m = rest.min(have_m - pm);
        pm += more_m;
        rest -= more_m;
        (rest == 0).then_some((pe, pm))
    } else {
        let (mut pe, mut pm) = (e.min(have_e), m.min(have_m));
        let paid = pe + pm;
        if paid < total {
            return None;
        }
        // Credits beyond the shortfalls replace Excavators first.
        let mut over = paid - total;
        let r = over.min(pe);
        pe -= r;
        over -= r;
        pm -= over;
        Some((pe, pm))
    }
}

/// What a construction takes, once checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost {
    /// Engineers placed on the Company board (0: none needed).
    pub engineers: u8,
    pub excavators: u32,
    pub mixers: u32,
    pub credits: u32,
}

impl State {
    /// A new game for 2 to 4 players. The same seed gives the same game.
    pub fn new(players: usize, seed: u64) -> State {
        assert!((MIN_PLAYERS..=MAX_PLAYERS).contains(&players), "the game is for 2 to 4 players");
        let mut rng = seed;
        let mut companies = Company::ALL.to_vec();
        shuffle(&mut companies, &mut rng);
        let mut officers = Officer::ALL.to_vec();
        shuffle(&mut officers, &mut rng);
        let mut starting = contracts_of(Tier::Starting);
        shuffle(&mut starting, &mut rng);
        let pairs = (0..players).map(|i| (companies[i], officers[i], None)).collect();
        let contracts = starting[..players].iter().map(|&c| (c, None)).collect();

        let mut tiles: Vec<usize> = (0..HEADSTREAM_TILES.len()).collect();
        shuffle(&mut tiles, &mut rng);
        let headstream_tiles = [tiles[0], tiles[1], tiles[2], tiles[3]];

        let mut bonus = Bonus::ALL.to_vec();
        shuffle(&mut bonus, &mut rng);
        bonus.truncate(ROUNDS as usize);
        let mut objectives = Objective::ALL.to_vec();
        shuffle(&mut objectives, &mut rng);

        let mut nationals = contracts_of(Tier::National);
        shuffle(&mut nationals, &mut rng);
        nationals.truncate(players - 1);
        let mut piles: [Vec<u8>; 3] = Tier::PRIVATE.map(contracts_of);
        for p in &mut piles {
            shuffle(p, &mut rng);
        }
        let mut offers = [[None; 2]; 3];
        for (t, pile) in piles.iter_mut().enumerate() {
            offers[t] = [pile.pop(), pile.pop()];
        }
        let mut tech: [Vec<u8>; 3] = [(0..5).collect(), (5..10).collect(), (10..15).collect()];
        for t in &mut tech {
            shuffle(t, &mut rng);
        }

        // One neutral Dam in each area, on the lower space of a basin the
        // setup tiles pick: level 1 in the Mountains, 2 in the Hills, 3 in
        // the Plains, each holding a drop.
        let mut dams = vec![None; DAM_SPACES];
        let pick = |basins: &[usize], rng: &mut u64| basins[(next_random(rng) % basins.len() as u64) as usize];
        for (basins, level) in [(&[M1, M2, M3, M4][..], 1), (&[H1, H2, H3][..], 2), (&[P1, P2, P3][..], 3)] {
            let b = pick(basins, &mut rng);
            dams[b * 2 + 1] = Some(Dam { owner: None, level, water: 1 });
        }

        let mut state = State {
            players: (0..players).map(|_| Player::new()).collect(),
            phase: Phase::Setup,
            round: 0,
            order: (0..players).rev().collect(),
            turn: 0,
            acted: false,
            todo: Vec::new(),
            setup: Some(Setup { pairs, contracts, next: 0 }),
            dams,
            conduits: vec![None; CONDUITS.len()],
            powerhouses: vec![None; POWERHOUSES.len()],
            headstreams: [0; HEADSTREAMS],
            headstream_tiles,
            offers,
            piles,
            nationals,
            patents: [None; 3],
            tech,
            bonus,
            objective: objectives[0],
            placements: Vec::new(),
            log: Vec::new(),
            rng,
        };
        state.refill_patents();
        state
    }

    pub fn seats(&self) -> usize {
        self.players.len()
    }

    /// Whose turn it is in the actions.
    pub fn current(&self) -> usize {
        self.order[self.turn]
    }

    /// Who has to act now, if anybody.
    pub fn actor(&self) -> Option<usize> {
        if let Some(t) = self.todo.first() {
            return Some(t.seat);
        }
        match self.phase {
            Phase::Setup => self.setup.as_ref().map(|s| s.next),
            Phase::Actions => Some(self.current()),
            _ => None,
        }
    }

    fn event(&mut self, event: Event) {
        self.log.push(event);
        if self.log.len() > LOG_LEN {
            self.log.remove(0);
        }
    }

    fn company(&self, seat: usize) -> Company {
        self.players[seat].company.unwrap_or(Company::Usa)
    }

    fn refill_patents(&mut self) {
        for slot in 0..3 {
            self.patents[slot] = self.tech.iter_mut().find(|p| !p.is_empty()).and_then(Vec::pop);
        }
    }

    fn refill_offers(&mut self) {
        for t in 0..3 {
            for slot in 0..2 {
                if self.offers[t][slot].is_none() {
                    self.offers[t][slot] = self.piles[t].pop();
                }
            }
        }
    }

    // ----- The map -----

    /// How many drops a Dam holds at most.
    pub fn capacity(&self, dam: &Dam) -> u8 {
        let delmonte = dam.owner.is_some_and(|s| self.players[s].has(Officer::DelMonte));
        if dam.level == 3 && delmonte {
            4
        } else {
            dam.level
        }
    }

    /// Drops enter `basin` one by one and run downstream: through its
    /// Powerhouses (the USA's ability, unless `from_production`, for the
    /// basin they were produced in), into its upper Dam then its lower one
    /// while they have room, then on along the river or off the map.
    pub(crate) fn flow(&mut self, basin: usize, drops: u32, from_production: bool) {
        let mut usa = vec![0u32; self.seats()];
        for _ in 0..drops {
            let mut b = basin;
            let mut first = true;
            loop {
                if !(first && from_production) {
                    for p in powerhouses_of(b) {
                        if let Some(s) = self.powerhouses[p] {
                            if self.players[s].ability(Company::Usa) {
                                usa[s] += 1;
                            }
                        }
                    }
                }
                let held = dams_of(b).find(|&d| self.dams[d].is_some_and(|dam| dam.water < self.capacity(&dam)));
                if let Some(d) = held {
                    if let Some(dam) = self.dams[d].as_mut() {
                        dam.water += 1;
                    }
                    break;
                }
                match BASINS[b].flows_to {
                    Some(next) => {
                        b = next;
                        first = false;
                    }
                    None => break,
                }
            }
        }
        for (seat, energy) in usa.into_iter().enumerate() {
            if energy > 0 {
                self.players[seat].energy += energy;
                self.event(Event::Flowed { seat, energy });
            }
        }
    }

    /// Drops on the Headstreams, flowing now or waiting for the Water Flow.
    fn pour(&mut self, drops: [u8; HEADSTREAMS], now: bool) {
        for (h, &n) in drops.iter().enumerate() {
            if now {
                self.flow(headstream_basin(h), n as u32, false);
            } else {
                self.headstreams[h] += n;
            }
        }
    }

    /// Pieces of `seat` in each basin.
    pub fn pieces_in(&self, seat: usize) -> [u32; 12] {
        let mut n = [0u32; 12];
        for (d, dam) in self.dams.iter().enumerate() {
            if let Some(dam) = dam {
                if dam.owner == Some(seat) {
                    n[dam_basin(d)] += dam.level as u32;
                }
            }
        }
        for (c, owner) in self.conduits.iter().enumerate() {
            if *owner == Some(seat) {
                n[CONDUITS[c].basin] += 1;
            }
        }
        for (p, owner) in self.powerhouses.iter().enumerate() {
            if *owner == Some(seat) {
                n[POWERHOUSES[p].basin] += 1;
            }
        }
        n
    }

    // ----- Building -----

    /// Whether `seat` may put a piece on `target`, leaving costs aside.
    pub fn can_place(&self, seat: usize, target: Target) -> Result<(), Error> {
        let p = &self.players[seat];
        let kind = target.kind();
        if p.built(kind) >= kind.pieces() {
            return Err(Error::NoPiece);
        }
        match target {
            Target::Base(d) => {
                if d >= DAM_SPACES {
                    return Err(Error::NoSuchSpace);
                }
                if self.dams[d].is_some() {
                    return Err(Error::Occupied);
                }
                if dams_of(dam_basin(d)).any(|o| self.dams[o].is_some_and(|dam| dam.owner == Some(seat))) {
                    return Err(Error::OneABasin);
                }
            }
            Target::Elevation(d) => match self.dams.get(d).copied().flatten() {
                Some(dam) if dam.owner == Some(seat) => {
                    if dam.level >= 3 {
                        return Err(Error::Top);
                    }
                }
                _ => return Err(Error::NotYourDam),
            },
            Target::Conduit(c) => {
                if c >= CONDUITS.len() {
                    return Err(Error::NoSuchSpace);
                }
                if self.conduits[c].is_some() {
                    return Err(Error::Occupied);
                }
            }
            Target::Powerhouse(ph) => {
                if ph >= POWERHOUSES.len() {
                    return Err(Error::NoSuchSpace);
                }
                if self.powerhouses[ph].is_some() {
                    return Err(Error::Occupied);
                }
                if powerhouses_of(POWERHOUSES[ph].basin).any(|o| self.powerhouses[o] == Some(seat)) {
                    return Err(Error::OneABasin);
                }
            }
        }
        Ok(())
    }

    /// The tile a construction builds with: its own, or the one copied.
    fn effective_tile(&self, seat: usize, b: &Build) -> Result<Tile, Error> {
        let p = &self.players[seat];
        let copied = |t: Option<Tile>| -> Result<Tile, Error> {
            let t = t.ok_or(Error::NoCopy)?;
            if p.on_wheel().contains(&t) {
                Ok(t)
            } else {
                Err(Error::NoCopy)
            }
        };
        match b.tile {
            Some(Tile::Krylov) => {
                if !p.tiles.contains(&Tile::Krylov) {
                    return Err(Error::NoTile);
                }
                copied(b.copy)
            }
            Some(t) => {
                if !p.tiles.contains(&t) {
                    return Err(Error::NoTile);
                }
                if b.copy.is_some() {
                    return Err(Error::NoCopy);
                }
                Ok(t)
            }
            None if p.copied == Some(Officer::Krylov) => copied(b.copy),
            None => Err(Error::NoTile),
        }
    }

    /// What a construction costs `seat`, or why they can't make it.
    pub fn construction(&self, seat: usize, b: &Build) -> Result<Cost, Error> {
        let p = &self.players[seat];
        let tile = self.effective_tile(seat, b)?;
        let kind = b.target.kind();
        if !tile.builds().allows(kind) {
            return Err(Error::WrongTile);
        }
        self.can_place(seat, b.target)?;
        let mut credits = 0;
        let no_engineers = matches!(tile, Tile::Advanced(3) | Tile::Advanced(10));
        let engineers = if no_engineers {
            0
        } else {
            let c = p.constructions as usize;
            if c >= CONSTRUCTION.len() {
                return Err(Error::NoConstruction);
            }
            if c == CONSTRUCTION.len() - 1 {
                credits += FEE;
            }
            CONSTRUCTION[c]
        };
        if p.engineers < engineers {
            return Err(Error::Engineers);
        }
        let area = BASINS[b.target.basin()].area;
        let (mut e, mut m) = (0u32, 0u32);
        match b.target {
            Target::Base(_) => {
                e = if p.has(Officer::Adler) { 3 } else { area.base_cost() as u32 };
                if tile == Tile::Advanced(10) {
                    e = 0;
                }
            }
            Target::Elevation(_) => m = area.elevation_cost() as u32,
            Target::Conduit(c) => {
                let value = CONDUITS[c].value as u32;
                if b.mixers {
                    if !p.has(Officer::McDowell) {
                        return Err(Error::NotYourOfficer);
                    }
                    m = value;
                } else {
                    e = 2 * value;
                    if tile == Tile::Advanced(7) && e >= 6 {
                        e = 5;
                    }
                }
            }
            Target::Powerhouse(_) => m = 2 + p.built(Kind::Powerhouse) as u32,
        }
        if b.target.paid() && tile != Tile::Advanced(4) {
            credits += FEE;
        }
        let k = b.credits as u32;
        if k > 0 && !p.has(Officer::Jordan) {
            return Err(Error::NotYourOfficer);
        }
        let swap = tile == Tile::Advanced(9);
        let (pe, pm) = split_payment(e, m, p.excavators, p.mixers, k, swap).ok_or(Error::Machineries)?;
        credits += 3 * k;
        if p.credits < credits {
            return Err(Error::Credits);
        }
        Ok(Cost { engineers, excavators: pe, mixers: pm, credits })
    }

    /// Fewest Machineries Solomon P. Jordan pays in Credits for a build
    /// (0 for anybody else, or `None` if even that can't be paid).
    pub fn jordan_credits(&self, seat: usize, b: &Build) -> Option<u8> {
        let p = &self.players[seat];
        let max = if p.has(Officer::Jordan) { 12 } else { 0 };
        (0..=max).find(|&k| self.construction(seat, &Build { credits: k, ..*b }).is_ok())
    }

    /// Puts the piece on the map, takes it off the Company board, and
    /// queues the income it uncovers.
    fn place(&mut self, seat: usize, target: Target, tile: Option<Tile>) {
        match target {
            Target::Base(d) => self.dams[d] = Some(Dam { owner: Some(seat), level: 1, water: 0 }),
            Target::Elevation(d) => {
                if let Some(dam) = self.dams[d].as_mut() {
                    dam.level += 1;
                }
            }
            Target::Conduit(c) => self.conduits[c] = Some(seat),
            Target::Powerhouse(ph) => self.powerhouses[ph] = Some(seat),
        }
        let kind = target.kind();
        let p = &mut self.players[seat];
        p.built[kind.index()] += 1;
        let built = p.built(kind);
        self.event(Event::Built { seat, target, tile });
        if let Some(i) = INCOME_AT.iter().position(|&at| at == built) {
            let income = self.company(seat).income(kind)[i];
            self.queue(seat, income.iter().map(|&r| Task::Reward(r)).collect());
        }
    }

    /// Turns the wheel: whatever comes back to the open segment returns to
    /// the supply.
    fn rotate(&mut self, seat: usize, turns: u8) {
        let p = &mut self.players[seat];
        for _ in 0..turns {
            p.open = (p.open + 1) % WHEEL;
            let back = std::mem::take(&mut p.wheel[p.open]);
            if let Some(t) = back.tile {
                p.tiles.push(t);
            }
            p.excavators += back.excavators;
            p.mixers += back.mixers;
        }
    }

    /// Queues tasks for `seat`, before whatever is already waiting.
    fn queue(&mut self, seat: usize, tasks: Vec<Task>) {
        let todos: Vec<Todo> = tasks.into_iter().map(|task| Todo { seat, task }).collect();
        self.todo.splice(0..0, todos);
    }

    fn build(&mut self, seat: usize, b: Build) -> Result<(), Error> {
        let cost = self.construction(seat, &b)?;
        let tile = self.effective_tile(seat, &b)?;
        let p = &mut self.players[seat];
        if cost.engineers > 0 {
            p.engineers -= cost.engineers;
            p.constructions += 1;
        }
        p.credits -= cost.credits;
        p.excavators -= cost.excavators;
        p.mixers -= cost.mixers;
        if let Some(t) = b.tile {
            let i = p.tiles.iter().position(|&x| x == t).expect("checked");
            p.tiles.remove(i);
        }
        let open = p.open;
        p.wheel[open] = Segment { tile: b.tile, excavators: cost.excavators, mixers: cost.mixers };
        self.rotate(seat, 1);
        self.place(seat, b.target, b.tile.or(b.copy));
        self.tile_effect(seat, tile, b.target);
        self.acted = true;
        Ok(())
    }

    /// An Advanced tile's special effect, once its piece is built.
    fn tile_effect(&mut self, seat: usize, tile: Tile, target: Target) {
        let Tile::Advanced(id) = tile else { return };
        let mut later = Vec::new();
        match id {
            0 => {
                for dam in self.dams.iter_mut().flatten() {
                    if dam.owner == Some(seat) && dam.water == 0 {
                        dam.water = 1;
                    }
                }
            }
            1 => {
                let n = self.players[seat].built(Kind::Elevation);
                self.rotate(seat, n);
            }
            2 => {
                if let Target::Conduit(c) = target {
                    self.players[seat].credits += 2 * CONDUITS[c].value as u32;
                }
            }
            5 => {
                let energy = self.players[seat].built(Kind::Base) as u32;
                self.generate(seat, energy, &mut later);
            }
            6 => {
                let d = target.space();
                if let Some(dam) = self.dams[d] {
                    let cap = self.capacity(&dam);
                    if let Some(dam) = self.dams[d].as_mut() {
                        dam.water = dam.water.max(cap);
                    }
                }
            }
            8 => later.push(Task::Produce { bonus: 2, plain: false, not: None }),
            11 => {
                let dams = self.dams.iter().flatten().filter(|d| d.owner == Some(seat) && d.level >= 2).count();
                self.score(seat, 3 * dams as u32);
            }
            12 => {
                if let Target::Conduit(c) = target {
                    let energy = 2 * CONDUITS[c].value as u32;
                    self.generate(seat, energy, &mut later);
                }
            }
            13 => {
                let n = self.players[seat].built(Kind::Powerhouse) as u32;
                self.score(seat, 3 * n);
            }
            14 => {
                let n = self.players[seat].advanced() as u32;
                self.score(seat, 3 * n);
            }
            _ => {}
        }
        // After the income the piece uncovered.
        let at = self.todo.iter().position(|t| t.seat != seat).unwrap_or(self.todo.len());
        let todos: Vec<Todo> = later.into_iter().map(|task| Todo { seat, task }).collect();
        self.todo.splice(at..at, todos);
    }

    fn score(&mut self, seat: usize, vp: u32) {
        if vp > 0 {
            self.award(seat, vp as i32, Why::Tile);
        }
    }

    /// Energy that isn't a production, but can fulfill a Contract.
    fn generate(&mut self, seat: usize, energy: u32, later: &mut Vec<Task>) {
        self.players[seat].energy += energy;
        self.event(Event::Generated { seat, energy });
        later.push(Task::Contract { energy });
    }

    // ----- Producing -----

    /// The productions `seat` can make: (powerhouse, conduit, dam) with the
    /// most drops they can send.
    pub fn productions(&self, seat: usize, not: Option<usize>) -> Vec<(usize, usize, usize, u8)> {
        let mut out = Vec::new();
        for (ph, owner) in self.powerhouses.iter().enumerate() {
            if *owner != Some(seat) || Some(ph) == not {
                continue;
            }
            let to = POWERHOUSES[ph].basin;
            for (c, conduit) in CONDUITS.iter().enumerate() {
                let Some(owner) = self.conduits[c] else { continue };
                if conduit.to != to {
                    continue;
                }
                for d in dams_of(conduit.basin) {
                    let Some(dam) = self.dams[d] else { continue };
                    if dam.water == 0 || !(dam.owner.is_none() || dam.owner == Some(seat)) {
                        continue;
                    }
                    let mut max = dam.water;
                    if owner != seat {
                        max = max.min(self.players[seat].credits.min(255) as u8);
                    }
                    if max > 0 {
                        out.push((ph, c, d, max));
                    }
                }
            }
        }
        out
    }

    /// The production bonus of `seat`'s Company board.
    pub fn company_bonus(&self, seat: usize) -> i32 {
        powerhouse_bonus(self.players[seat].built(Kind::Powerhouse))
    }

    fn produce(&mut self, seat: usize, pr: Production, bonus: i32, plain: bool) -> Result<(), Error> {
        let ph_owner = self.powerhouses.get(pr.powerhouse).copied().flatten();
        if ph_owner != Some(seat) {
            return Err(Error::NoPowerhouse);
        }
        let Some(conduit_owner) = self.conduits.get(pr.conduit).copied().flatten() else {
            return Err(Error::NoConduit);
        };
        let conduit = CONDUITS[pr.conduit];
        if conduit.to != POWERHOUSES[pr.powerhouse].basin || pr.dam >= DAM_SPACES || dam_basin(pr.dam) != conduit.basin {
            return Err(Error::NotConnected);
        }
        let dam = self.dams[pr.dam].ok_or(Error::NoWater)?;
        if !(dam.owner.is_none() || dam.owner == Some(seat)) || pr.drops == 0 || dam.water < pr.drops {
            return Err(Error::NoWater);
        }
        let paying = conduit_owner != seat;
        if paying && self.players[seat].credits < pr.drops as u32 {
            return Err(Error::Credits);
        }
        let p = &self.players[seat];
        let extra = if plain { 0 } else { bonus + self.company_bonus(seat) };
        let energy = production_energy(pr.drops, conduit.value, p.has(Officer::Fiesler), extra);
        if energy < 1 {
            return Err(Error::TooLittle);
        }
        let energy = energy as u32;
        if paying {
            self.players[seat].credits -= pr.drops as u32;
            self.players[conduit_owner].credits += pr.drops as u32;
            self.players[conduit_owner].vp += pr.drops as u32;
        }
        if let Some(d) = self.dams[pr.dam].as_mut() {
            d.water -= pr.drops;
        }
        self.players[seat].energy += energy;
        self.event(Event::Produced { seat, production: pr, energy, paid_to: paying.then_some(conduit_owner) });
        self.flow(POWERHOUSES[pr.powerhouse].basin, pr.drops as u32, true);
        let mut tasks = vec![Task::Contract { energy }];
        if self.players[seat].ability(Company::Italy) {
            tasks.push(Task::Reward(Reward::Energy(3)));
        }
        if !plain && self.players[seat].ability(Company::Germany) {
            tasks.push(Task::Produce { bonus: 0, plain: true, not: Some(pr.powerhouse) });
        }
        self.queue(seat, tasks);
        Ok(())
    }

    /// The Contracts `seat` could fulfill with `energy`.
    pub fn fulfillable(&self, seat: usize, energy: u32) -> Vec<u8> {
        let p = &self.players[seat];
        let discount = if p.ability(Company::France) { 3 } else { 0 };
        p.contracts
            .iter()
            .chain(&self.nationals)
            .copied()
            .filter(|&c| (CONTRACTS[c as usize].energy as u32).saturating_sub(discount) <= energy)
            .collect()
    }

    // ----- Rewards and the todo list -----

    /// Whether a task needs its owner to decide something, and can't just be
    /// applied.
    fn needs_decision(&self, todo: &Todo) -> bool {
        match todo.task {
            Task::Reward(r) => matches!(
                r,
                Reward::Machinery(_)
                    | Reward::Water(_)
                    | Reward::WaterNow(_)
                    | Reward::Base(_)
                    | Reward::Elevation
                    | Reward::Conduit(_)
                    | Reward::Powerhouse
            ),
            _ => true,
        }
    }

    /// Whether a decision has any choice to make (if not, it's dropped).
    fn has_choice(&self, todo: &Todo) -> bool {
        let seat = todo.seat;
        match todo.task {
            Task::Reward(r) => match r {
                Reward::Base(_) | Reward::Elevation | Reward::Conduit(_) | Reward::Powerhouse => {
                    !self.free_targets(seat, r).is_empty()
                }
                _ => true,
            },
            Task::Contract { energy } => !self.fulfillable(seat, energy).is_empty(),
            Task::Discard => self.players[seat].contracts.len() > MAX_CONTRACTS,
            Task::Produce { not, .. } => !self.productions(seat, not).is_empty(),
        }
    }

    /// Where a free building (a reward) can go, with the 3 Credits of a red
    /// space paid if it is one.
    pub fn free_targets(&self, seat: usize, reward: Reward) -> Vec<Target> {
        let credits = self.players[seat].credits;
        let targets: Vec<Target> = match reward {
            Reward::Base(areas) => {
                (0..DAM_SPACES).filter(|&d| areas.allows(BASINS[dam_basin(d)].area)).map(Target::Base).collect()
            }
            Reward::Elevation => (0..DAM_SPACES).map(Target::Elevation).collect(),
            Reward::Conduit(max) => {
                (0..CONDUITS.len()).filter(|&c| CONDUITS[c].value <= max).map(Target::Conduit).collect()
            }
            Reward::Powerhouse => (0..POWERHOUSES.len()).map(Target::Powerhouse).collect(),
            _ => Vec::new(),
        };
        targets
            .into_iter()
            .filter(|&t| self.can_place(seat, t).is_ok() && (!t.paid() || credits >= FEE))
            .collect()
    }

    fn apply_reward(&mut self, seat: usize, reward: Reward) {
        let p = &mut self.players[seat];
        match reward {
            Reward::Credits(c) => p.credits += c as u32,
            Reward::Vp(v) => p.vp += v as u32,
            Reward::Excavators(e) => p.excavators += e as u32,
            Reward::Mixers(m) => p.mixers += m as u32,
            Reward::Energy(e) => p.energy += e as u32,
            Reward::Rotate(r) => self.rotate(seat, r),
            _ => return,
        }
        self.event(Event::Gained { seat, reward });
    }

    /// Applies what needs no decision, drops decisions with no choice, and
    /// moves the game on (turns, rounds) until somebody has to act.
    fn proceed(&mut self) {
        loop {
            while let Some(&todo) = self.todo.first() {
                if self.needs_decision(&todo) {
                    if self.has_choice(&todo) {
                        return;
                    }
                    self.todo.remove(0);
                    continue;
                }
                self.todo.remove(0);
                if let Task::Reward(r) = todo.task {
                    self.apply_reward(todo.seat, r);
                }
            }
            match self.phase {
                Phase::Setup | Phase::Over => return,
                Phase::Income => {
                    self.phase = Phase::Actions;
                    self.turn = 0;
                    self.acted = false;
                }
                Phase::Actions => {
                    if self.acted {
                        self.end_turn();
                        continue;
                    }
                    if self.players.iter().all(|p| p.passed) {
                        self.end_round();
                        continue;
                    }
                    let seat = self.current();
                    if self.players[seat].passed {
                        self.next_turn();
                        continue;
                    }
                    if self.players[seat].engineers == 0 && !self.can_build_without_engineers(seat) {
                        self.players[seat].passed = true;
                        self.event(Event::Passed { seat });
                        self.end_turn();
                        continue;
                    }
                    return;
                }
            }
        }
    }

    /// With a tile that needs no Engineers, a player with none left can
    /// still build.
    pub fn can_build_without_engineers(&self, seat: usize) -> bool {
        self.build_options(seat).iter().any(|o| o.cost.engineers == 0)
    }

    fn end_turn(&mut self) {
        let seat = self.current();
        self.players[seat].copied = None;
        self.acted = false;
        self.refill_offers();
        self.next_turn();
    }

    fn next_turn(&mut self) {
        let n = self.order.len();
        for step in 1..=n {
            let t = (self.turn + step) % n;
            if !self.players[self.order[t]].passed {
                self.turn = t;
                return;
            }
        }
    }

    // ----- Rounds -----

    fn start_round(&mut self) {
        self.round += 1;
        self.event(Event::Round(self.round));
        if self.round <= 4 {
            for h in 0..HEADSTREAMS {
                self.headstreams[h] += HEADSTREAM_TILES[self.headstream_tiles[h]][self.round as usize - 1];
            }
        }
        let mut todo = Vec::new();
        for &seat in &self.order {
            let company = self.company(seat);
            for kind in [Kind::Base, Kind::Elevation, Kind::Conduit] {
                for reward in incomes(company, kind, self.players[seat].built(kind)) {
                    todo.push(Todo { seat, task: Task::Reward(reward) });
                }
            }
        }
        self.todo.extend(todo);
        self.phase = Phase::Income;
        self.turn = 0;
        self.acted = false;
    }

    fn end_round(&mut self) {
        // Water Flow.
        let drops = self.headstreams;
        self.headstreams = [0; HEADSTREAMS];
        for (h, &n) in drops.iter().enumerate() {
            self.flow(headstream_basin(h), n as u32, false);
        }
        self.event(Event::WaterFlow);
        self.score_round();
        if self.round >= ROUNDS {
            self.final_scoring();
            self.phase = Phase::Over;
            self.event(Event::GameOver);
            return;
        }
        // New turn order: least energy first; tied players swap.
        let mut order: Vec<usize> = self.order.iter().rev().copied().collect();
        order.sort_by_key(|&s| self.players[s].energy);
        self.order = order;
        for p in &mut self.players {
            p.energy = 0;
            p.engineers = ENGINEERS;
            p.constructions = 0;
            p.sekibo = 0;
            p.passed = false;
        }
        self.placements.clear();
        self.refill_patents();
        self.start_round();
    }

    fn score_round(&mut self) {
        // 6 and 2 VP for the two best producers.
        let mut energies: Vec<u32> = self.players.iter().map(|p| p.energy).filter(|&e| e > 0).collect();
        energies.sort_unstable_by(|a, b| b.cmp(a));
        energies.dedup();
        if let Some(&best) = energies.first() {
            let firsts: Vec<usize> = (0..self.seats()).filter(|&s| self.players[s].energy == best).collect();
            if firsts.len() == 1 {
                self.award(firsts[0], FIRST_VP as i32, Why::FirstEnergy);
                if let Some(&second) = energies.get(1) {
                    let seconds: Vec<usize> = (0..self.seats()).filter(|&s| self.players[s].energy == second).collect();
                    let vp = if seconds.len() == 1 { SECOND_VP } else { 1 };
                    for s in seconds {
                        self.award(s, vp as i32, Why::SecondEnergy);
                    }
                }
            } else {
                let vp = (FIRST_VP + SECOND_VP).div_ceil(firsts.len() as u32);
                for s in firsts {
                    self.award(s, vp as i32, Why::FirstEnergy);
                }
            }
        }
        let bonus = self.bonus[self.round as usize - 1];
        for seat in 0..self.seats() {
            let energy = self.players[seat].energy;
            let credits = energy_credits(energy);
            self.players[seat].credits += credits;
            self.event(Event::Credits { seat, credits });
            if energy == 0 {
                self.award(seat, -(NOTHING_PENALTY as i32), Why::NoEnergy);
            }
            let vp = self.bonus_vp(seat, bonus);
            if vp > 0 {
                self.award(seat, vp as i32, Why::Bonus);
            }
        }
    }

    /// What the round's Bonus tile gives `seat` now.
    pub fn bonus_vp(&self, seat: usize, bonus: Bonus) -> u32 {
        let p = &self.players[seat];
        let section = energy_section(p.energy);
        if section == 0 {
            return 0;
        }
        let count = match bonus {
            Bonus::Contracts => p.fulfilled.len() as u32,
            Bonus::Powerhouses => p.built(Kind::Powerhouse) as u32,
            Bonus::Bases => p.built(Kind::Base) as u32,
            Bonus::Conduits => p.built(Kind::Conduit) as u32,
            Bonus::Elevations => p.built(Kind::Elevation) as u32,
            Bonus::Advanced => p.advanced() as u32,
        };
        let behind = (self.round as u32).saturating_sub(section);
        (count * bonus.vp()).saturating_sub(SECTION_PENALTY * behind)
    }

    fn award(&mut self, seat: usize, vp: i32, why: Why) {
        let p = &mut self.players[seat];
        p.vp = (p.vp as i32 + vp).max(0) as u32;
        self.event(Event::Scored { seat, vp, why });
    }

    /// How far `seat` gets on the Objective tile.
    pub fn objective_count(&self, seat: usize) -> u32 {
        match self.objective {
            Objective::RedSpaces => {
                let bases = (0..DAM_SPACES)
                    .filter(|&d| dam_paid(d) && self.dams[d].is_some_and(|dam| dam.owner == Some(seat)))
                    .count();
                let ph = (0..POWERHOUSES.len()).filter(|&p| POWERHOUSES[p].paid && self.powerhouses[p] == Some(seat)).count();
                (bases + ph) as u32
            }
            Objective::ConnectedBases => (0..DAM_SPACES)
                .filter(|&d| self.dams[d].is_some_and(|dam| dam.owner == Some(seat)))
                .filter(|&d| {
                    conduits_of(dam_basin(d)).any(|c| {
                        self.conduits[c] == Some(seat) && powerhouses_of(CONDUITS[c].to).any(|p| self.powerhouses[p] == Some(seat))
                    })
                })
                .count() as u32,
            Objective::MostArea | Objective::LeastArea => {
                let pieces = self.pieces_in(seat);
                let per_area = Area::ALL.map(|a| (0..12).filter(|&b| BASINS[b].area == a).map(|b| pieces[b]).sum::<u32>());
                if self.objective == Objective::MostArea {
                    per_area.into_iter().max().unwrap_or(0)
                } else {
                    per_area.into_iter().min().unwrap_or(0)
                }
            }
            Objective::Basins1 => self.pieces_in(seat).iter().filter(|&&n| n >= 1).count() as u32,
            Objective::Basins3 => self.pieces_in(seat).iter().filter(|&&n| n >= 3).count() as u32,
        }
    }

    /// The Objective's VP for each seat: 15, 10 and 5 to the first three
    /// (ties share their places' VP, rounded up). Nothing for a count of 0.
    pub fn objective_vp(&self) -> Vec<u32> {
        let counts: Vec<u32> = (0..self.seats()).map(|s| self.objective_count(s)).collect();
        let mut vp = vec![0; self.seats()];
        let mut place = 0;
        let mut levels: Vec<u32> = counts.iter().copied().filter(|&c| c > 0).collect();
        levels.sort_unstable_by(|a, b| b.cmp(a));
        levels.dedup();
        for level in levels {
            let tied: Vec<usize> = (0..self.seats()).filter(|&s| counts[s] == level).collect();
            let pot: u32 = (place..place + tied.len()).filter_map(|i| OBJECTIVE_VP.get(i)).sum();
            for &s in &tied {
                vp[s] = pot.div_ceil(tied.len() as u32);
            }
            place += tied.len();
        }
        vp
    }

    fn final_scoring(&mut self) {
        for (seat, vp) in self.objective_vp().into_iter().enumerate() {
            if vp > 0 {
                self.award(seat, vp as i32, Why::Objective);
            }
            let p = &self.players[seat];
            let resources = (p.credits + p.excavators + p.mixers) / 5;
            if resources > 0 {
                self.award(seat, resources as i32, Why::Resources);
            }
            let water: u32 =
                self.dams.iter().flatten().filter(|d| d.owner == Some(seat)).map(|d| d.water as u32).sum();
            if water > 0 {
                self.award(seat, water as i32, Why::Water);
            }
        }
    }

    /// The winners once the game is over: the most VP, then the most
    /// energy in the last round. Ties beyond that share the win.
    pub fn winners(&self) -> Vec<usize> {
        if self.phase != Phase::Over {
            return Vec::new();
        }
        let key = |s: usize| (self.players[s].vp, self.players[s].energy);
        let best = (0..self.seats()).map(key).max();
        (0..self.seats()).filter(|&s| Some(key(s)) == best).collect()
    }

    // ----- Actions -----

    /// Checks a Management space for `seat` and places their Engineers
    /// there, paying its cost and fee.
    fn occupy(&mut self, seat: usize, spot: Spot) -> Result<(), Error> {
        let (def, row) = spot.def().ok_or(Error::NoSuchSpace)?;
        if row.players > self.seats() {
            return Err(Error::Closed);
        }
        if self.placements.iter().any(|p| p.spot == spot) {
            return Err(Error::Taken);
        }
        let credits = row.cost + if def.fee { FEE } else { 0 };
        let p = &mut self.players[seat];
        if p.engineers < def.engineers {
            return Err(Error::Engineers);
        }
        if p.credits < credits {
            return Err(Error::Credits);
        }
        p.engineers -= def.engineers;
        p.credits -= credits;
        self.placements.push(Placement { spot, seat, engineers: def.engineers });
        Ok(())
    }

    /// Whether `seat` could take `spot` now (Engineers, Credits, free).
    pub fn spot_open(&self, seat: usize, spot: Spot) -> bool {
        let Some((def, row)) = spot.def() else { return false };
        let p = &self.players[seat];
        row.players <= self.seats()
            && !self.placements.iter().any(|pl| pl.spot == spot)
            && p.engineers >= def.engineers
            && p.credits >= row.cost + if def.fee { FEE } else { 0 }
    }

    /// Applies `seat`'s action. Nothing changes if it is refused.
    pub fn apply(&mut self, seat: usize, action: Action) -> Result<(), Error> {
        if seat >= self.seats() {
            return Err(Error::UnknownSeat);
        }
        if self.phase == Phase::Over {
            return Err(Error::GameOver);
        }
        if self.actor() != Some(seat) {
            return Err(Error::NotYourTurn);
        }
        let mut next = self.clone();
        next.act(seat, action)?;
        next.proceed();
        *self = next;
        Ok(())
    }

    fn act(&mut self, seat: usize, action: Action) -> Result<(), Error> {
        if let Some(&todo) = self.todo.first() {
            return self.settle(seat, todo, action);
        }
        if self.phase == Phase::Setup {
            let Action::Choose { pair, contract } = action else { return Err(Error::WrongPhase) };
            return self.choose(seat, pair, contract);
        }
        if self.phase != Phase::Actions {
            return Err(Error::WrongPhase);
        }
        let has_engineers = self.players[seat].engineers > 0;
        match action {
            Action::Build(b) => self.build(seat, b)?,
            Action::Pass => {
                if has_engineers {
                    return Err(Error::CantPass);
                }
                self.players[seat].passed = true;
                self.event(Event::Passed { seat });
                self.acted = true;
            }
            _ if !has_engineers => return Err(Error::Engineers),
            Action::Produce { space: Some(spot), production } => {
                if spot.section != Section::Turbine {
                    return Err(Error::NoSuchSpace);
                }
                self.occupy(seat, spot)?;
                self.produce(seat, production, TURBINE_BONUS[spot.row as usize], false)?;
                self.acted = true;
            }
            Action::Water { space: Some(spot), drops } => {
                if spot.section != Section::Water {
                    return Err(Error::NoSuchSpace);
                }
                let total: u8 = drops.iter().sum();
                let now = spot.row == 1;
                let ok = if now { total == 1 } else { (1..=2).contains(&total) };
                if !ok {
                    return Err(Error::TooMuch);
                }
                self.occupy(seat, spot)?;
                self.pour(drops, now);
                self.event(Event::Watered { seat, drops, now });
                self.acted = true;
            }
            Action::Bank { engineers } => {
                let p = &mut self.players[seat];
                if engineers == 0 || engineers > p.engineers {
                    return Err(Error::Engineers);
                }
                p.engineers -= engineers;
                p.credits += engineers as u32;
                self.placements.push(Placement { spot: Spot::new(Section::Bank, 0, false), seat, engineers });
                self.event(Event::Bank { seat, credits: engineers as u32 });
                self.acted = true;
            }
            Action::Workshop { space } => {
                if space.section != Section::Workshop {
                    return Err(Error::NoSuchSpace);
                }
                self.occupy(seat, space)?;
                let turns = WORKSHOP_TURNS[space.row as usize];
                self.rotate(seat, turns);
                self.event(Event::Rotated { seat, turns });
                self.acted = true;
            }
            Action::Buy { space, mixer } => {
                if space.section != Section::Shop {
                    return Err(Error::NoSuchSpace);
                }
                self.occupy(seat, space)?;
                let (e, m) = match space.row {
                    0 => (1, 0),
                    1 if mixer => (0, 1),
                    1 => (1, 0),
                    _ => (1, 1),
                };
                let p = &mut self.players[seat];
                p.excavators += e;
                p.mixers += m;
                self.event(Event::Bought { seat, excavators: e, mixers: m });
                self.acted = true;
            }
            Action::Contracts { space, picks } => {
                if space.section != Section::Contracts {
                    return Err(Error::NoSuchSpace);
                }
                let most = CONTRACTS_TAKEN.get(space.row as usize).copied().ok_or(Error::NoSuchSpace)?;
                if picks.is_empty() || picks.len() > most {
                    return Err(Error::TooMuch);
                }
                let mut taken = Vec::new();
                for (i, pick) in picks.iter().enumerate() {
                    if picks[..i].contains(pick) {
                        return Err(Error::NoSuchContract);
                    }
                    let c = self
                        .offers
                        .get(pick.tier as usize)
                        .and_then(|t| t.get(pick.slot as usize))
                        .copied()
                        .flatten()
                        .ok_or(Error::NoSuchContract)?;
                    taken.push(c);
                }
                self.occupy(seat, space)?;
                for pick in &picks {
                    self.offers[pick.tier as usize][pick.slot as usize] = None;
                }
                self.players[seat].contracts.extend(&taken);
                self.event(Event::TookContracts { seat, contracts: taken });
                let excess = self.players[seat].contracts.len().saturating_sub(MAX_CONTRACTS);
                self.queue(seat, vec![Task::Discard; excess]);
                self.acted = true;
            }
            Action::Patent { slot } => {
                let tile = self.patents.get(slot as usize).copied().flatten().ok_or(Error::NoTile2)?;
                self.occupy(seat, Spot::new(Section::Patent, slot, false))?;
                self.patents[slot as usize] = None;
                self.players[seat].tiles.push(Tile::Advanced(tile));
                self.event(Event::Patent { seat, tile: Tile::Advanced(tile) });
                self.acted = true;
            }
            Action::Copy { officer } => {
                let p = &self.players[seat];
                if p.officer != Some(Officer::Sekibo) || officer == Officer::Sekibo {
                    return Err(Error::NotSekibo);
                }
                if p.copied.is_some() || p.sekibo >= 2 {
                    return Err(Error::CopyUsed);
                }
                let fee = if p.sekibo == 1 { FEE } else { 0 };
                if p.credits < fee {
                    return Err(Error::Credits);
                }
                let p = &mut self.players[seat];
                p.credits -= fee;
                p.engineers -= 1;
                p.sekibo += 1;
                p.copied = Some(officer);
                let right = p.sekibo == 2;
                self.placements.push(Placement { spot: Spot::new(Section::Sekibo, 0, right), seat, engineers: 1 });
                self.event(Event::Copied { seat, officer });
                // Out of Engineers, with nothing to build without: the turn is over.
                if self.players[seat].engineers == 0 && !self.can_build_without_engineers(seat) {
                    self.acted = true;
                }
            }
            _ => return Err(Error::WrongPhase),
        }
        Ok(())
    }

    fn choose(&mut self, seat: usize, pair: usize, contract: usize) -> Result<(), Error> {
        let setup = self.setup.as_mut().ok_or(Error::WrongPhase)?;
        if setup.next != seat {
            return Err(Error::NotYourTurn);
        }
        let Some(&(company, officer, None)) = setup.pairs.get(pair) else { return Err(Error::NoSuchSpace) };
        let Some(&(c, None)) = setup.contracts.get(contract) else { return Err(Error::NoSuchContract) };
        setup.pairs[pair].2 = Some(seat);
        setup.contracts[contract].1 = Some(seat);
        setup.next += 1;
        let done = setup.next >= self.players.len();
        let p = &mut self.players[seat];
        p.company = Some(company);
        p.officer = Some(officer);
        p.contracts.push(c);
        if officer == Officer::Krylov {
            p.tiles.push(Tile::Krylov);
        }
        self.event(Event::Chose { seat, company, officer, contract: c });
        if done {
            // The first to pick plays last.
            self.order = (0..self.seats()).rev().collect();
            self.start_round();
        }
        Ok(())
    }

    /// Settles the first task of the todo list.
    fn settle(&mut self, seat: usize, todo: Todo, action: Action) -> Result<(), Error> {
        debug_assert_eq!(todo.seat, seat);
        match (todo.task, action) {
            (Task::Reward(Reward::Machinery(n)), Action::Machinery { excavators }) => {
                if excavators > n {
                    return Err(Error::TooMuch);
                }
                let p = &mut self.players[seat];
                p.excavators += excavators as u32;
                p.mixers += (n - excavators) as u32;
                self.todo.remove(0);
                self.event(Event::Bought { seat, excavators: excavators as u32, mixers: (n - excavators) as u32 });
            }
            (Task::Reward(r @ (Reward::Water(n) | Reward::WaterNow(n))), Action::Water { space: None, drops }) => {
                if drops.iter().sum::<u8>() > n {
                    return Err(Error::TooMuch);
                }
                let now = matches!(r, Reward::WaterNow(_));
                self.todo.remove(0);
                self.pour(drops, now);
                self.event(Event::Watered { seat, drops, now });
            }
            (
                Task::Reward(r @ (Reward::Base(_) | Reward::Elevation | Reward::Conduit(_) | Reward::Powerhouse)),
                Action::Place { target },
            ) => {
                if !self.free_targets(seat, r).contains(&target) {
                    self.can_place(seat, target)?;
                    return Err(if target.kind() == Kind::Conduit { Error::TooBig } else { Error::Area });
                }
                self.todo.remove(0);
                if target.paid() {
                    self.players[seat].credits -= FEE;
                }
                self.place(seat, target, None);
            }
            (Task::Contract { energy }, Action::Fulfill { contract }) => {
                self.todo.remove(0);
                let Some(c) = contract else { return Ok(()) };
                if !self.fulfillable(seat, energy).contains(&c) {
                    return Err(Error::NoContract);
                }
                let p = &mut self.players[seat];
                if let Some(i) = p.contracts.iter().position(|&x| x == c) {
                    p.contracts.remove(i);
                } else if let Some(i) = self.nationals.iter().position(|&x| x == c) {
                    self.nationals.remove(i);
                }
                self.players[seat].fulfilled.push(c);
                self.event(Event::Fulfilled { seat, contract: c });
                let rewards = CONTRACTS[c as usize].rewards.iter().map(|&r| Task::Reward(r)).collect();
                self.queue(seat, rewards);
            }
            (Task::Discard, Action::Discard { contract }) => {
                let p = &mut self.players[seat];
                let i = p.contracts.iter().position(|&x| x == contract).ok_or(Error::NoSuchContract)?;
                p.contracts.remove(i);
                self.todo.remove(0);
                self.event(Event::Discarded { seat, contract });
            }
            (Task::Produce { bonus, plain, not }, Action::Produce { space: None, production }) => {
                if Some(production.powerhouse) == not {
                    return Err(Error::SamePowerhouse);
                }
                self.todo.remove(0);
                self.produce(seat, production, bonus, plain)?;
            }
            (
                Task::Reward(
                    Reward::Base(_)
                    | Reward::Elevation
                    | Reward::Conduit(_)
                    | Reward::Powerhouse
                    | Reward::Water(_)
                    | Reward::WaterNow(_),
                ),
                Action::Skip,
            ) => {
                self.todo.remove(0);
            }
            (Task::Produce { .. } | Task::Contract { .. }, Action::Skip) => {
                self.todo.remove(0);
            }
            (_, Action::Skip) => return Err(Error::Optional),
            _ => return Err(Error::Settle),
        }
        Ok(())
    }

    /// Every construction `seat` can make now, with what it costs (Solomon
    /// P. Jordan paying as few Machineries in Credits as needed).
    pub fn build_options(&self, seat: usize) -> Vec<BuildOption> {
        let p = &self.players[seat];
        let mut tiles: Vec<(Option<Tile>, Option<Tile>)> = Vec::new();
        let mut supply = p.tiles.clone();
        supply.sort_by_key(|t| format!("{t:?}"));
        supply.dedup();
        for &t in &supply {
            if t == Tile::Krylov {
                tiles.extend(p.on_wheel().into_iter().map(|c| (Some(t), Some(c))));
            } else {
                tiles.push((Some(t), None));
            }
        }
        if p.copied == Some(Officer::Krylov) {
            tiles.extend(p.on_wheel().into_iter().map(|c| (None, Some(c))));
        }
        let mut targets: Vec<Target> = Vec::new();
        targets.extend((0..DAM_SPACES).map(Target::Base));
        targets.extend((0..DAM_SPACES).map(Target::Elevation));
        targets.extend((0..CONDUITS.len()).map(Target::Conduit));
        targets.extend((0..POWERHOUSES.len()).map(Target::Powerhouse));
        let targets: Vec<Target> = targets.into_iter().filter(|&t| self.can_place(seat, t).is_ok()).collect();
        let mut out = Vec::new();
        for &(tile, copy) in &tiles {
            for &target in &targets {
                for mixers in [false, true] {
                    if mixers && !(p.has(Officer::McDowell) && target.kind() == Kind::Conduit) {
                        continue;
                    }
                    let b = Build { tile, copy, target, mixers, credits: 0 };
                    let Some(k) = self.jordan_credits(seat, &b) else { continue };
                    let b = Build { credits: k, ..b };
                    if let Ok(cost) = self.construction(seat, &b) {
                        out.push(BuildOption { build: b, cost });
                    }
                }
            }
        }
        out
    }
}

/// A construction that can be made now, and its cost.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildOption {
    pub build: Build,
    pub cost: Cost,
}
