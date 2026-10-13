//! The printed components: Company boards, Executive Officers, Technology
//! tiles, Contracts, Bonus and Objective tiles, Headstream tiles, the Energy
//! Track and the Management board's action spaces.
//!
//! Numbers come from the 2019 English rulebook and from photographs of the
//! printed boards and tiles (see `docs/barrage.md`).

use crate::map::Area;
use serde::{Deserialize, Serialize};

/// The four structure pieces a company builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Kind {
    Base,
    Elevation,
    Conduit,
    Powerhouse,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Base, Kind::Elevation, Kind::Conduit, Kind::Powerhouse];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Base => "Base",
            Kind::Elevation => "Elevation",
            Kind::Conduit => "Conduit",
            Kind::Powerhouse => "Powerhouse",
        }
    }

    pub fn plural(self) -> &'static str {
        match self {
            Kind::Base => "Bases",
            Kind::Elevation => "Elevations",
            Kind::Conduit => "Conduits",
            Kind::Powerhouse => "Powerhouses",
        }
    }

    /// Pieces of this kind on a Company board.
    pub fn pieces(self) -> u8 {
        match self {
            Kind::Powerhouse => 4,
            _ => 5,
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

/// Where a Base may go, for a Contract that builds one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Areas {
    Any,
    Plains,
    HillsOrPlains,
}

impl Areas {
    pub fn allows(self, area: Area) -> bool {
        match self {
            Areas::Any => true,
            Areas::Plains => area == Area::Plains,
            Areas::HillsOrPlains => area != Area::Mountains,
        }
    }
}

/// One thing a Contract, an income or a tile gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reward {
    Credits(u8),
    Vp(u8),
    Excavators(u8),
    Mixers(u8),
    /// Machineries, Excavators or Concrete Mixers as you like.
    Machinery(u8),
    /// Turns of the Construction Wheel.
    Rotate(u8),
    /// Drops on Headstreams of your choice, flowing in the Water Flow phase.
    Water(u8),
    /// Drops on Headstreams of your choice that flow at once.
    WaterNow(u8),
    /// Steps on the Energy Track; not for Contracts.
    Energy(u8),
    /// A free Base (no Engineers, tile or Machineries).
    Base(Areas),
    Elevation,
    /// A Conduit of at most this production value.
    Conduit(u8),
    Powerhouse,
}

impl Reward {
    /// Says what it gives, e.g. "3 VP".
    pub fn text(self) -> String {
        let n = |n: u8, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        match self {
            Reward::Credits(c) => n(c, "Credit", "Credits"),
            Reward::Vp(v) => format!("{v} VP"),
            Reward::Excavators(e) => n(e, "Excavator", "Excavators"),
            Reward::Mixers(m) => n(m, "Concrete Mixer", "Concrete Mixers"),
            Reward::Machinery(m) => n(m, "Machinery of your choice", "Machineries of your choice"),
            Reward::Rotate(r) => format!("turn the wheel {}", n(r, "segment", "segments")),
            Reward::Water(w) => n(w, "drop on a Headstream", "drops on Headstreams"),
            Reward::WaterNow(w) => format!("{} that flow at once", n(w, "drop", "drops")),
            Reward::Energy(e) => format!("{e} energy steps"),
            Reward::Base(Areas::Any) => "a free Base".into(),
            Reward::Base(Areas::Plains) => "a free Base in the Plains".into(),
            Reward::Base(Areas::HillsOrPlains) => "a free Base in the Hills or Plains".into(),
            Reward::Elevation => "a free Elevation".into(),
            Reward::Conduit(v) => format!("a free Conduit of value {v} or less"),
            Reward::Powerhouse => "a free Powerhouse".into(),
        }
    }
}

/// The four companies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Company {
    Usa,
    Germany,
    Italy,
    France,
}

impl Company {
    pub const ALL: [Company; 4] = [Company::Usa, Company::Germany, Company::Italy, Company::France];

    pub fn name(self) -> &'static str {
        match self {
            Company::Usa => "USA",
            Company::Germany => "Germany",
            Company::Italy => "Italy",
            Company::France => "France",
        }
    }

    /// The CEO on the board.
    pub fn ceo(self) -> &'static str {
        match self {
            Company::Usa => "Marguerite Grant",
            Company::Germany => "Oberst Dassler",
            Company::Italy => "Enrico Olivi",
            Company::France => "Joseph Fontaine",
        }
    }

    /// The colour of its pieces.
    pub fn color(self) -> &'static str {
        match self {
            Company::Usa => "red",
            Company::Germany => "black",
            Company::Italy => "green",
            Company::France => "white",
        }
    }

    /// What the third Powerhouse switches on.
    pub fn ability(self) -> &'static str {
        match self {
            Company::Usa => "Each drop that flows naturally through one of your Powerhouses moves your Energy marker 1 step.",
            Company::Germany => "After a production, you may produce again with another Powerhouse (no bonus).",
            Company::Italy => "After a production, move your Energy marker 3 more steps.",
            Company::France => "You fulfill Contracts with 3 Energy Units less than they ask.",
        }
    }

    /// The incomes of a row (Bases, Elevations, Conduits): uncovered by the
    /// 2nd, 4th and 5th piece built.
    pub fn income(self, kind: Kind) -> [&'static [Reward]; 3] {
        use Reward::*;
        const SEVEN: &[Reward] = &[Vp(7)];
        match (self, kind) {
            (Company::Usa, Kind::Base) => [&[Rotate(1)], &[Credits(6)], SEVEN],
            (Company::Usa, Kind::Elevation) => [&[Machinery(1)], &[WaterNow(2), Vp(2)], SEVEN],
            (Company::Usa, Kind::Conduit) => [&[Vp(3)], &[Rotate(1), Vp(2)], SEVEN],
            (Company::Germany, Kind::Base) => [&[Vp(3)], &[Machinery(2)], SEVEN],
            (Company::Germany, Kind::Elevation) => [&[Credits(3)], &[Vp(2), Energy(2)], SEVEN],
            (Company::Germany, Kind::Conduit) => [&[Rotate(1)], &[WaterNow(2), Vp(2)], SEVEN],
            (Company::Italy, Kind::Base) => [&[Credits(3)], &[Vp(5)], SEVEN],
            (Company::Italy, Kind::Elevation) => [&[Rotate(1)], &[Energy(4)], SEVEN],
            (Company::Italy, Kind::Conduit) => [&[Machinery(1)], &[Credits(3), Vp(2)], SEVEN],
            (Company::France, Kind::Base) => [&[Machinery(1)], &[Rotate(2)], SEVEN],
            (Company::France, Kind::Elevation) => [&[Vp(3)], &[Vp(2), Machinery(1)], SEVEN],
            (Company::France, Kind::Conduit) => [&[Credits(3)], &[Energy(4)], SEVEN],
            (_, Kind::Powerhouse) => [&[], &[], &[]],
        }
    }
}

/// Which pieces of a row uncover an income (the 2nd, 4th and 5th).
pub const INCOME_AT: [u8; 3] = [2, 4, 5];

/// The incomes uncovered once `built` pieces of `kind` are on the map.
pub fn incomes(company: Company, kind: Kind, built: u8) -> Vec<Reward> {
    let rows = company.income(kind);
    INCOME_AT.iter().zip(rows).filter(|(&at, _)| built >= at).flat_map(|(_, r)| r.iter().copied()).collect()
}

/// The production bonus of a company with this many Powerhouses: +1 from
/// the second, +3 from the fourth.
pub fn powerhouse_bonus(built: u8) -> i32 {
    match built {
        0 | 1 => 0,
        2 | 3 => 1,
        _ => 3,
    }
}

/// A company's own ability comes with its third Powerhouse.
pub const ABILITY_AT: u8 = 3;

/// The seven Executive Officers of the base game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Officer {
    Adler,
    DelMonte,
    Fiesler,
    McDowell,
    Jordan,
    Krylov,
    Sekibo,
}

impl Officer {
    pub const ALL: [Officer; 7] = [
        Officer::Adler,
        Officer::DelMonte,
        Officer::Fiesler,
        Officer::McDowell,
        Officer::Jordan,
        Officer::Krylov,
        Officer::Sekibo,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Officer::Adler => "Wilhelm Adler",
            Officer::DelMonte => "Graziano Del Monte",
            Officer::Fiesler => "Viktor Fiesler",
            Officer::McDowell => "Jill McDowell",
            Officer::Jordan => "Solomon P. Jordan",
            Officer::Krylov => "Anton Krylov",
            Officer::Sekibo => "Mahiri Sekibo",
        }
    }

    pub fn ability(self) -> &'static str {
        match self {
            Officer::Adler => "Your Bases always cost 3 Excavators, wherever you build them.",
            Officer::DelMonte => "Your level 3 Dams hold up to 4 drops.",
            Officer::Fiesler => "A production that makes less than 4 Energy Units makes 4 (before bonuses).",
            Officer::McDowell => "You may build Conduits with 1 Concrete Mixer per production value instead of 2 Excavators.",
            Officer::Jordan => "When you build, you may pay 3 Credits instead of any Machinery.",
            Officer::Krylov => "Your special Technology tile copies another tile on your Construction Wheel, special effect included.",
            Officer::Sekibo => "Place an Engineer on your tile (3 Credits more the second time in a round) to copy another Executive Officer's ability for the turn.",
        }
    }
}

/// A Technology tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tile {
    /// One of the four basic tiles every company starts with.
    Basic(Kind),
    /// One of the 15 Advanced Technology tiles (see [`Advanced`]).
    Advanced(u8),
    /// Anton Krylov's special tile.
    Krylov,
}

/// What an Advanced Technology tile builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Builds {
    One(Kind),
    Any,
}

impl Builds {
    pub fn allows(self, kind: Kind) -> bool {
        match self {
            Builds::One(k) => k == kind,
            Builds::Any => true,
        }
    }
}

pub const ADVANCED: usize = 15;

/// Advanced tile `id`: level `id / 5 + 1` (I, II, III), building a Base,
/// an Elevation, a Conduit, a Powerhouse or anything, in that order.
pub fn advanced_level(id: u8) -> u8 {
    id / 5 + 1
}

pub fn advanced_builds(id: u8) -> Builds {
    match id % 5 {
        4 => Builds::Any,
        k => Builds::One(Kind::ALL[k as usize]),
    }
}

/// What an Advanced tile does besides building.
pub fn advanced_effect(id: u8) -> &'static str {
    match id {
        0 => "Put a drop in each of your empty Dams.",
        1 => "Turn your wheel 1 segment for each Elevation you have built.",
        2 => "Gain 2 Credits per production value of the Conduit.",
        3 => "No Engineers needed.",
        4 => "No 3 Credits for a red building space.",
        5 => "Produce 1 Energy Unit per Base you have built (a Contract can be fulfilled).",
        6 => "Fill the Dam you just raised with drops.",
        7 => "A Conduit costing 6 Excavators or more costs 5.",
        8 => "Produce at once with any of your Powerhouses, +2.",
        9 => "Excavators and Concrete Mixers stand in for each other.",
        10 => "No Engineers and no Excavators needed.",
        11 => "3 VP for each of your Dams with an Elevation.",
        12 => "Produce 2 Energy Units per production value of the Conduit (a Contract can be fulfilled).",
        13 => "3 VP for each Powerhouse you have built.",
        _ => "3 VP for each Advanced Technology tile you have.",
    }
}

impl Tile {
    pub fn builds(self) -> Builds {
        match self {
            Tile::Basic(k) => Builds::One(k),
            Tile::Advanced(id) => advanced_builds(id),
            Tile::Krylov => Builds::Any,
        }
    }

    pub fn name(self) -> String {
        match self {
            Tile::Basic(k) => k.name().to_string(),
            Tile::Advanced(id) => {
                let level = ["I", "II", "III"][(advanced_level(id) - 1) as usize];
                match advanced_builds(id) {
                    Builds::One(k) => format!("{} {level}", k.name()),
                    Builds::Any => format!("Wild {level}"),
                }
            }
            Tile::Krylov => "Krylov's tile".into(),
        }
    }

    pub fn is_advanced(self) -> bool {
        matches!(self, Tile::Advanced(_))
    }
}

/// The level of a Contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tier {
    Starting,
    Green,
    Yellow,
    Red,
    National,
}

impl Tier {
    /// The three piles of private Contracts.
    pub const PRIVATE: [Tier; 3] = [Tier::Green, Tier::Yellow, Tier::Red];

    pub fn name(self) -> &'static str {
        match self {
            Tier::Starting => "Starting",
            Tier::Green => "Green",
            Tier::Yellow => "Yellow",
            Tier::Red => "Red",
            Tier::National => "National",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contract {
    pub tier: Tier,
    /// Energy Units needed, in one production.
    pub energy: u8,
    pub rewards: &'static [Reward],
}

const fn c(tier: Tier, energy: u8, rewards: &'static [Reward]) -> Contract {
    Contract { tier, energy, rewards }
}

/// Every Contract tile: 4 starting, 15 of each private colour, 6 national.
/// A Contract's id is its index here.
pub const CONTRACTS: [Contract; 55] = {
    use Reward::*;
    use Tier::*;
    [
        c(Starting, 3, &[Vp(2), Energy(2)]),
        c(Starting, 3, &[Vp(2), Rotate(1)]),
        c(Starting, 2, &[Vp(3)]),
        c(Starting, 4, &[Vp(2), Machinery(1)]),
        c(Green, 2, &[Credits(5)]),
        c(Green, 2, &[Vp(2), Energy(3)]),
        c(Green, 2, &[Credits(3), Water(2)]),
        c(Green, 2, &[Machinery(2)]),
        c(Green, 2, &[Credits(1), Rotate(2)]),
        c(Green, 3, &[Credits(4), WaterNow(1)]),
        c(Green, 3, &[Credits(4), Excavators(1)]),
        c(Green, 3, &[Vp(3), Mixers(1)]),
        c(Green, 3, &[Vp(4), Credits(2)]),
        c(Green, 3, &[Energy(2), Rotate(2)]),
        c(Green, 4, &[Base(Areas::Plains)]),
        c(Green, 4, &[Vp(3), WaterNow(2)]),
        c(Green, 4, &[Conduit(2)]),
        c(Green, 4, &[Vp(5), Excavators(1)]),
        c(Green, 4, &[Vp(1), Rotate(3)]),
        c(Yellow, 5, &[Powerhouse]),
        c(Yellow, 5, &[Vp(2), WaterNow(1), Rotate(2)]),
        c(Yellow, 5, &[Credits(3), Mixers(2)]),
        c(Yellow, 5, &[Vp(4), Excavators(2)]),
        c(Yellow, 5, &[Vp(3), Excavators(1), Energy(2)]),
        c(Yellow, 6, &[Base(Areas::HillsOrPlains)]),
        c(Yellow, 6, &[Vp(2), Elevation]),
        c(Yellow, 6, &[Conduit(3)]),
        c(Yellow, 6, &[Rotate(3), WaterNow(1)]),
        c(Yellow, 6, &[Vp(3), Mixers(2)]),
        c(Yellow, 7, &[Credits(4), Water(2), Machinery(1)]),
        c(Yellow, 7, &[Vp(3), WaterNow(2), Mixers(1)]),
        c(Yellow, 7, &[Vp(6), Water(3)]),
        c(Yellow, 7, &[Vp(7), Rotate(1)]),
        c(Yellow, 7, &[Vp(6), Excavators(1)]),
        c(Red, 8, &[Base(Areas::Any)]),
        c(Red, 8, &[Vp(4), Energy(5)]),
        c(Red, 8, &[Conduit(4)]),
        c(Red, 8, &[Vp(4), Rotate(3)]),
        c(Red, 8, &[Credits(4), Mixers(1), Rotate(2)]),
        c(Red, 9, &[Powerhouse, Energy(3)]),
        c(Red, 9, &[Elevation, WaterNow(2)]),
        c(Red, 9, &[Vp(5), Water(3), Excavators(1)]),
        c(Red, 9, &[Vp(3), Credits(2), Machinery(2)]),
        c(Red, 9, &[Vp(8), Rotate(1)]),
        c(Red, 10, &[Vp(2), Machinery(3)]),
        c(Red, 10, &[Vp(6), Rotate(2), Water(2)]),
        c(Red, 10, &[Conduit(5)]),
        c(Red, 10, &[Vp(7), Credits(3)]),
        c(Red, 10, &[Vp(8), WaterNow(1)]),
        c(National, 13, &[Vp(7), Rotate(4)]),
        c(National, 13, &[Vp(6), Machinery(4)]),
        c(National, 14, &[Vp(8), Credits(8)]),
        c(National, 14, &[Vp(8), Energy(6)]),
        c(National, 15, &[Vp(12)]),
        c(National, 15, &[Vp(9), WaterNow(3)]),
    ]
};

/// The ids of a tier's Contracts.
pub fn contracts_of(tier: Tier) -> Vec<u8> {
    (0..CONTRACTS.len() as u8).filter(|&i| CONTRACTS[i as usize].tier == tier).collect()
}

/// The six Bonus tiles; five of them score the five rounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bonus {
    Contracts,
    Powerhouses,
    Bases,
    Conduits,
    Elevations,
    Advanced,
}

impl Bonus {
    pub const ALL: [Bonus; 6] =
        [Bonus::Contracts, Bonus::Powerhouses, Bonus::Bases, Bonus::Conduits, Bonus::Elevations, Bonus::Advanced];

    /// VP for each one.
    pub fn vp(self) -> u32 {
        match self {
            Bonus::Contracts => 2,
            Bonus::Powerhouses => 5,
            _ => 4,
        }
    }

    pub fn text(self) -> String {
        let what = match self {
            Bonus::Contracts => "Contract fulfilled",
            Bonus::Powerhouses => "Powerhouse",
            Bonus::Bases => "Base",
            Bonus::Conduits => "Conduit",
            Bonus::Elevations => "Elevation",
            Bonus::Advanced => "Advanced Technology tile",
        };
        format!("{} VP per {what}", self.vp())
    }
}

/// The six Objective tiles; one scores at the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Objective {
    /// Bases and Powerhouses on red building spaces.
    RedSpaces,
    /// Bases connected by a Conduit of yours to a Powerhouse of yours.
    ConnectedBases,
    /// Pieces in the area where you have built the most.
    MostArea,
    /// Pieces in the area where you have built the least.
    LeastArea,
    /// Basins with at least one of your pieces.
    Basins1,
    /// Basins with at least three of your pieces.
    Basins3,
}

impl Objective {
    pub const ALL: [Objective; 6] = [
        Objective::RedSpaces,
        Objective::ConnectedBases,
        Objective::MostArea,
        Objective::LeastArea,
        Objective::Basins1,
        Objective::Basins3,
    ];

    pub fn text(self) -> &'static str {
        match self {
            Objective::RedSpaces => "Bases and Powerhouses on red building spaces",
            Objective::ConnectedBases => "Bases linked by your Conduit to your Powerhouse",
            Objective::MostArea => "Pieces in the area where you built the most",
            Objective::LeastArea => "Pieces in the area where you built the least",
            Objective::Basins1 => "Basins with at least 1 of your pieces",
            Objective::Basins3 => "Basins with at least 3 of your pieces",
        }
    }
}

/// VP for the first, second and third on the Objective.
pub const OBJECTIVE_VP: [u32; 3] = [15, 10, 5];

/// The eight Headstream tiles: drops in rounds 1 to 4 (none in round 5).
pub const HEADSTREAM_TILES: [[u8; 4]; 8] =
    [[1, 1, 1, 1], [0, 0, 1, 3], [2, 0, 2, 0], [0, 1, 2, 1], [0, 1, 0, 2], [1, 0, 2, 2], [0, 2, 2, 0], [0, 0, 1, 2]];

pub const ROUNDS: u8 = 5;

/// Credits at the end of a round for the energy produced in it: 3 for
/// nothing (but 3 VP lost), then 1 from 1, 2 from 2, 3 from 4, 4 from 7,
/// 5 from 11, 6 from 16, 7 from 22, 8 from 29.
pub fn energy_credits(energy: u32) -> u32 {
    const STEPS: [u32; 8] = [1, 2, 4, 7, 11, 16, 22, 29];
    if energy == 0 {
        return 3;
    }
    STEPS.iter().filter(|&&s| energy >= s).count() as u32
}

/// VP lost for producing nothing in a round.
pub const NOTHING_PENALTY: u32 = 3;

/// The section of the Energy Track the marker is in: 0 below 6, then 1 to
/// 5, one per round (6, 12, 18, 24, 30).
pub fn energy_section(energy: u32) -> u32 {
    (energy / 6).min(5)
}

/// VP a Bonus tile is cut by for each section behind the round's.
pub const SECTION_PENALTY: u32 = 4;

/// VP for the first and second on the Energy Track each round.
pub const FIRST_VP: u32 = 6;
pub const SECOND_VP: u32 = 2;

pub const START_VP: u32 = 10;
pub const START_CREDITS: u32 = 6;
pub const START_EXCAVATORS: u32 = 6;
pub const START_MIXERS: u32 = 4;
pub const ENGINEERS: u8 = 12;
/// Contracts you may keep face up.
pub const MAX_CONTRACTS: usize = 3;
/// What a red space or a red action space costs.
pub const FEE: u32 = 3;
/// The Construction Wheel's segments.
pub const WHEEL: usize = 6;
/// What the Patent Office charges.
pub const PATENT_COST: u32 = 5;

/// Engineers for the 1st to 4th construction of a round on the Company
/// board; the 4th also costs [`FEE`].
pub const CONSTRUCTION: [u8; 4] = [1, 2, 3, 3];

/// A Management board section.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Section {
    Turbine,
    Water,
    Workshop,
    Shop,
    Contracts,
    Patent,
    Bank,
    /// Mahiri Sekibo's own action space.
    Sekibo,
}

impl Section {
    pub fn name(self) -> &'static str {
        match self {
            Section::Turbine => "Turbine Station",
            Section::Water => "Water Management",
            Section::Workshop => "Workshop",
            Section::Shop => "Machinery Shop",
            Section::Contracts => "Contract Office",
            Section::Patent => "Patent Office",
            Section::Bank => "Bank",
            Section::Sekibo => "Mahiri Sekibo",
        }
    }
}

/// An action space: Engineers to place, whether its red outline costs
/// [`FEE`], and from how many players it is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceDef {
    pub engineers: u8,
    pub fee: bool,
}

const fn s(engineers: u8, fee: bool) -> SpaceDef {
    SpaceDef { engineers, fee }
}

/// A row of a section: the action, its left and right spaces, and the
/// fewest players it is open with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub left: SpaceDef,
    pub right: Option<SpaceDef>,
    pub players: usize,
    /// Credits the action itself costs.
    pub cost: u32,
}

const fn row(left: SpaceDef, right: Option<SpaceDef>, players: usize, cost: u32) -> Row {
    Row { left, right, players, cost }
}

/// The production bonus of each Turbine Station row.
pub const TURBINE_BONUS: [i32; 5] = [2, 1, 0, -1, -2];

pub fn rows(section: Section) -> &'static [Row] {
    const TURBINE: [Row; 5] = [
        row(s(2, false), Some(s(3, true)), 2, 0),
        row(s(2, false), Some(s(3, true)), 4, 0),
        row(s(2, false), Some(s(3, true)), 2, 0),
        row(s(2, false), Some(s(3, true)), 3, 0),
        row(s(1, false), Some(s(2, false)), 2, 0),
    ];
    // Two drops for later, or one that flows now.
    const WATER: [Row; 2] = [row(s(1, false), Some(s(2, true)), 2, 0), row(s(1, false), Some(s(2, true)), 2, 0)];
    // One, two or three turns of the wheel.
    const WORKSHOP: [Row; 3] = [
        row(s(1, false), Some(s(2, false)), 2, 0),
        row(s(2, false), Some(s(3, true)), 3, 2),
        row(s(2, false), Some(s(3, true)), 2, 5),
    ];
    // An Excavator, a Machinery of your choice, or one of each.
    const SHOP: [Row; 3] = [
        row(s(1, false), Some(s(1, true)), 2, 2),
        row(s(1, false), Some(s(2, false)), 4, 4),
        row(s(2, false), Some(s(3, true)), 2, 5),
    ];
    // One Contract, one Contract, or two for a Credit.
    const CONTRACTS: [Row; 3] = [
        row(s(1, false), Some(s(1, true)), 2, 0),
        row(s(1, false), Some(s(1, true)), 4, 0),
        row(s(2, false), Some(s(3, false)), 2, 1),
    ];
    const PATENT: [Row; 3] = [
        row(s(2, false), None, 2, PATENT_COST),
        row(s(2, false), None, 2, PATENT_COST),
        row(s(2, false), None, 2, PATENT_COST),
    ];
    const SEKIBO: [Row; 1] = [row(s(1, false), Some(s(1, true)), 2, 0)];
    match section {
        Section::Turbine => &TURBINE,
        Section::Water => &WATER,
        Section::Workshop => &WORKSHOP,
        Section::Shop => &SHOP,
        Section::Contracts => &CONTRACTS,
        Section::Patent => &PATENT,
        Section::Sekibo => &SEKIBO,
        Section::Bank => &[],
    }
}

/// Turns of the wheel each Workshop row gives.
pub const WORKSHOP_TURNS: [u8; 3] = [1, 2, 3];

/// Contracts each Contract Office row takes.
pub const CONTRACTS_TAKEN: [usize; 3] = [1, 1, 2];
