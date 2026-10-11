//! The pieces of the game and the card catalogue.
//!
//! The catalogue is the published base game (78 designs, 150 cards) and its
//! Corruption & Ascension expansion (38 designs, 56 cards, in a deck of
//! their own), with all seven Empires on side A. The numbers were taken from
//! Game Park's online implementation (github.com/gamepark/its-a-wonderful-world,
//! `Developments.ts`, `AscensionDevelopments.ts` and `Empires.ts`). The whole
//! catalogue is the two tables in [`designs`], with each design's number of
//! copies, and [`EMPIRES`] above them.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// The five resources, in production order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Res {
    Materials,
    Energy,
    Science,
    Gold,
    Exploration,
}

impl Res {
    pub const ALL: [Res; 5] = [Res::Materials, Res::Energy, Res::Science, Res::Gold, Res::Exploration];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Res::Materials => "Materials",
            Res::Energy => "Energy",
            Res::Science => "Science",
            Res::Gold => "Gold",
            Res::Exploration => "Exploration",
        }
    }

    /// The character the player with the most of this resource receives.
    /// `None` for Science, where the winner chooses.
    pub fn supremacy_token(self) -> Option<Token> {
        match self {
            Res::Materials | Res::Gold => Some(Token::Financier),
            Res::Energy | Res::Exploration => Some(Token::General),
            Res::Science => None,
        }
    }
}

/// The five card types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    Structure,
    Vehicle,
    Research,
    Project,
    Discovery,
}

impl Kind {
    pub const ALL: [Kind; 5] = [Kind::Structure, Kind::Vehicle, Kind::Research, Kind::Project, Kind::Discovery];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Structure => "Structure",
            Kind::Vehicle => "Vehicle",
            Kind::Research => "Research",
            Kind::Project => "Project",
            Kind::Discovery => "Discovery",
        }
    }
}

/// The two kinds of character token. Each is worth a point at the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Token {
    General,
    Financier,
}

impl Token {
    pub fn name(self) -> &'static str {
        match self {
            Token::General => "General",
            Token::Financier => "Financier",
        }
    }
}

/// What a card needs to be built, or (in a building) what has been placed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost {
    /// Spaces for resource cubes, in [`Res::ALL`] order.
    pub res: [u8; 5],
    /// Spaces only Krystallium can fill.
    pub krystallium: u8,
    pub generals: u8,
    pub financiers: u8,
}

impl Cost {
    pub fn total(&self) -> u32 {
        self.res.iter().map(|&n| n as u32).sum::<u32>()
            + self.krystallium as u32
            + self.generals as u32
            + self.financiers as u32
    }

    /// What is still missing when `filled` has been placed.
    pub fn minus(&self, filled: &Cost) -> Cost {
        let mut res = [0u8; 5];
        for (i, r) in res.iter_mut().enumerate() {
            *r = self.res[i].saturating_sub(filled.res[i]);
        }
        Cost {
            res,
            krystallium: self.krystallium.saturating_sub(filled.krystallium),
            generals: self.generals.saturating_sub(filled.generals),
            financiers: self.financiers.saturating_sub(filled.financiers),
        }
    }

    pub fn is_zero(&self) -> bool {
        self.total() == 0
    }
}

/// What a card's points can count: the cards of a type you own, or the
/// characters of a kind you hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Per {
    Kind(Kind),
    Token(Token),
}

impl Per {
    pub fn name(self) -> &'static str {
        match self {
            Per::Kind(kind) => kind.name(),
            Per::Token(token) => token.name(),
        }
    }
}

impl From<Kind> for Per {
    fn from(kind: Kind) -> Per {
        Per::Kind(kind)
    }
}

impl From<Token> for Per {
    fn from(token: Token) -> Per {
        Per::Token(token)
    }
}

/// One thing a card hands over the moment it is finished. It is kept, like
/// the characters and Krystallium won any other way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bonus {
    Krystallium,
    Token(Token),
}

/// Identifies one physical card, of either deck. The first ids are one card
/// of each design, in catalogue order; the extra copies follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CardId(pub u16);

impl CardId {
    pub fn def(self) -> &'static Card {
        &catalogue()[deck_designs()[self.0 as usize % CARDS]]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    pub name: &'static str,
    pub kind: Kind,
    pub cost: Cost,
    /// Resource icons produced every round once finished.
    pub produces: [u8; 5],
    /// Corrupted resource icons (Corruption & Ascension): each takes one off
    /// that resource's production.
    pub corruption: [u8; 5],
    /// One more icon of this resource for every card of that type you own.
    pub scaled: Option<(Res, Kind)>,
    /// The cube you take when you recycle the card.
    pub recycle: Res,
    /// What finishing the card hands over (several of a kind, at most).
    pub bonus: &'static [Bonus],
    /// Points that depend on nothing.
    pub vp: u8,
    /// Points for each card of this type you own (this one included if it
    /// is of that type).
    pub combo: Option<(Kind, u8)>,
    /// Extra points for each character token of this type you hold.
    pub per_token: Option<(Token, u8)>,
    /// Points for each duo, one of each of the two (Corruption & Ascension's
    /// duo scoring).
    pub duo: Option<(Per, Per, u8)>,
    /// How many cards of this design are in its deck.
    pub copies: u8,
    /// A card of the Corruption & Ascension deck.
    pub ascension: bool,
}

impl Card {
    fn new(name: &'static str, kind: Kind, cost: [u8; 5], produces: [u8; 5], recycle: Res, vp: u8) -> Card {
        Card {
            name,
            kind,
            cost: Cost { res: cost, ..Cost::default() },
            produces,
            corruption: [0; 5],
            scaled: None,
            recycle,
            bonus: &[],
            vp,
            combo: None,
            per_token: None,
            duo: None,
            copies: 1,
            ascension: false,
        }
    }

    fn corruption(mut self, corrupted: [u8; 5]) -> Card {
        self.corruption = corrupted;
        self
    }

    /// Spaces in the cost that only Krystallium fills.
    fn krystallium(mut self, n: u8) -> Card {
        self.cost.krystallium = n;
        self
    }

    /// Character spaces in the cost: `g` Generals and `f` Financiers.
    fn chars(mut self, g: u8, f: u8) -> Card {
        self.cost.generals = g;
        self.cost.financiers = f;
        self
    }

    fn scaled(mut self, res: Res, kind: Kind) -> Card {
        self.scaled = Some((res, kind));
        self
    }

    fn bonus(mut self, bonus: &'static [Bonus]) -> Card {
        self.bonus = bonus;
        self
    }

    fn combo(mut self, kind: Kind, points: u8) -> Card {
        self.combo = Some((kind, points));
        self
    }

    fn per_token(mut self, token: Token, points: u8) -> Card {
        self.per_token = Some((token, points));
        self
    }

    fn duo(mut self, a: impl Into<Per>, b: impl Into<Per>, points: u8) -> Card {
        self.duo = Some((a.into(), b.into(), points));
        self
    }

    fn copies(mut self, n: u8) -> Card {
        self.copies = n;
        self
    }
}

/// The base game's designs, and its deck once every design's copies are
/// counted.
pub const DESIGNS: usize = 78;
pub const DECK_SIZE: usize = 150;
/// The Corruption & Ascension deck's designs and cards.
pub const ASCENSION_DESIGNS: usize = 38;
pub const ASCENSION_DECK_SIZE: usize = 56;
/// Every card of both decks: their ids run from 0 to `CARDS - 1`.
pub const CARDS: usize = DECK_SIZE + ASCENSION_DECK_SIZE;

static CATALOGUE: OnceLock<Vec<Card>> = OnceLock::new();
static DECK_DESIGNS: OnceLock<Vec<usize>> = OnceLock::new();

/// Every design, in catalogue order: the base game's, then Corruption &
/// Ascension's.
pub fn catalogue() -> &'static [Card] {
    CATALOGUE.get_or_init(designs)
}

/// Every card of the base deck, or of the Corruption & Ascension deck.
pub fn deck(ascension: bool) -> Vec<CardId> {
    (0..CARDS as u16).map(CardId).filter(|c| c.def().ascension == ascension).collect()
}

/// The design of every card, indexed by `CardId.0`: one of each design
/// first, then each design's extra copies.
fn deck_designs() -> &'static [usize] {
    DECK_DESIGNS.get_or_init(|| {
        let all = catalogue();
        let mut deck: Vec<usize> = (0..all.len()).collect();
        for (design, card) in all.iter().enumerate() {
            deck.extend(std::iter::repeat_n(design, card.copies as usize - 1));
        }
        debug_assert_eq!(deck.len(), CARDS);
        deck
    })
}

/// An Empire card (side A): the production every seat starts with, and the
/// points it scores at the end. They are dealt at random to the seats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Empire {
    pub name: &'static str,
    pub base: [u8; 5],
    /// Points for each card of this type you own.
    pub combo: Option<(Kind, u8)>,
    /// Points for each character token of this type you hold.
    pub per_token: Option<(Token, u8)>,
    /// One of the two Empires that come with Corruption & Ascension.
    pub ascension: bool,
}

const fn empire(name: &'static str, base: [u8; 5], combo: Option<(Kind, u8)>, per_token: Option<(Token, u8)>) -> Empire {
    Empire { name, base, combo, per_token, ascension: false }
}

pub const EMPIRES: [Empire; 7] = [
    empire("Noram States", [3, 0, 0, 1, 0], None, Some((Token::Financier, 1))),
    empire("Republic of Europe", [2, 1, 1, 0, 0], None, Some((Token::General, 1))),
    empire("Federation of Asia", [1, 0, 0, 2, 0], Some((Kind::Project, 2)), None),
    empire("Panafrican Union", [2, 0, 2, 0, 0], Some((Kind::Research, 2)), None),
    empire("Aztec Empire", [0, 2, 0, 0, 1], Some((Kind::Discovery, 3)), None),
    Empire { ascension: true, ..empire("Nations of Oceania", [3, 1, 0, 0, 0], Some((Kind::Structure, 1)), None) },
    Empire { ascension: true, ..empire("North Hegemony", [0, 3, 0, 0, 0], Some((Kind::Vehicle, 2)), None) },
];

fn designs() -> Vec<Card> {
    use Kind::{Discovery, Project, Research, Structure, Vehicle};
    use Res::{Energy as E, Exploration as X, Gold as G, Materials as M, Science as S};
    const KRYSTALLIUM: Bonus = Bonus::Krystallium;
    const GENERAL: Bonus = Bonus::Token(Token::General);
    const FINANCIER: Bonus = Bonus::Token(Token::Financier);
    let c = Card::new;

    let v = vec![
        // Structures (9 designs, 50 cards).
        c("Financial Center", Structure, [4, 1, 0, 0, 0], [0, 0, 0, 2, 0], G, 0).bonus(&[FINANCIER]).copies(5),
        c("Industrial Complex", Structure, [3, 1, 0, 0, 0], [1, 0, 0, 1, 0], G, 0).bonus(&[FINANCIER]).copies(6),
        c("Military Base", Structure, [3, 1, 0, 0, 0], [1, 0, 1, 0, 0], M, 0).bonus(&[GENERAL]).copies(6),
        c("Nuclear Plant", Structure, [4, 0, 1, 0, 0], [0, 3, 0, 0, 0], E, 0).copies(5),
        c("Offshore Oil Rig", Structure, [3, 0, 0, 0, 1], [0, 1, 0, 1, 0], E, 0).bonus(&[FINANCIER]).copies(5),
        c("Recycling Plant", Structure, [2, 0, 0, 0, 0], [2, 0, 0, 0, 0], M, 0).copies(7),
        c("Research Center", Structure, [3, 1, 0, 0, 0], [0, 0, 2, 0, 0], S, 0).copies(7),
        c("Transportation Network", Structure, [3, 0, 0, 0, 0], [0; 5], M, 0).combo(Vehicle, 1).copies(2),
        c("Wind Turbines", Structure, [2, 0, 0, 0, 0], [0, 1, 0, 0, 0], E, 0).copies(7),

        // Vehicles (9 designs, 31 cards).
        c("Airborne Laboratory", Vehicle, [0, 3, 0, 0, 0], [0, 0, 1, 0, 1], S, 0).copies(3),
        c("Aircraft Carrier", Vehicle, [3, 4, 0, 0, 0], [0; 5], M, 0).scaled(X, Vehicle).bonus(&[GENERAL, GENERAL]),
        c("Icebreaker", Vehicle, [0, 3, 1, 0, 0], [0, 0, 0, 0, 2], X, 0).copies(4),
        c("Juggernaut", Vehicle, [3, 3, 0, 0, 0], [0, 0, 0, 0, 2], M, 0).krystallium(1).bonus(&[GENERAL, GENERAL]).combo(Vehicle, 1),
        c("Mega-Drill", Vehicle, [1, 2, 0, 0, 0], [1, 0, 0, 0, 1], M, 0).copies(4),
        c("Saucer Squadron", Vehicle, [0, 3, 2, 0, 0], [0, 0, 0, 0, 3], S, 0).copies(2),
        c("Submarine", Vehicle, [2, 3, 0, 0, 0], [0, 0, 0, 0, 2], M, 0).bonus(&[GENERAL]).copies(3),
        c("Tank Division", Vehicle, [1, 2, 0, 0, 0], [0, 0, 0, 0, 1], M, 0).bonus(&[GENERAL]).copies(7),
        c("Zeppelin", Vehicle, [0, 2, 0, 0, 0], [0, 0, 0, 0, 1], X, 0).copies(6),

        // Research (23 designs, 23 cards).
        c("Aquaculture", Research, [0, 0, 4, 2, 0], [0; 5], S, 0).bonus(&[FINANCIER]).per_token(Token::Financier, 1),
        c("Bionic Grafts", Research, [0, 0, 5, 0, 0], [2, 0, 0, 0, 0], M, 4).bonus(&[GENERAL]),
        c("Climate Control", Research, [0, 0, 5, 0, 0], [0, 2, 0, 1, 0], E, 2),
        c("Cryopreservation", Research, [0, 0, 7, 0, 0], [0; 5], G, 0).bonus(&[FINANCIER]).per_token(Token::Financier, 1),
        c("Genetic Upgrades", Research, [0, 0, 4, 0, 0], [0; 5], S, 3).bonus(&[FINANCIER, FINANCIER]),
        c("Gravity Inverter", Research, [0, 1, 4, 0, 0], [0; 5], S, 0).krystallium(1).bonus(&[FINANCIER]).combo(Project, 2),
        c("Human Cloning", Research, [0, 0, 2, 1, 0], [0, 0, 0, 1, 0], G, 1).bonus(&[FINANCIER]),
        c("Mega-Bomb", Research, [0, 2, 2, 0, 0], [0; 5], E, 3).bonus(&[GENERAL, GENERAL]),
        c("Neuroscience", Research, [0, 0, 3, 0, 0], [0; 5], S, 1).scaled(S, Research),
        c("Quantum Generator", Research, [0, 0, 5, 0, 0], [0, 3, 0, 0, 0], E, 0).combo(Vehicle, 1),
        c("Robot Assistants", Research, [0, 0, 3, 0, 0], [0; 5], M, 1).scaled(M, Structure),
        c("Robotic Animals", Research, [0, 1, 2, 0, 0], [1, 0, 0, 0, 0], E, 2).bonus(&[GENERAL]),
        c("Satellites", Research, [0, 2, 4, 0, 0], [0, 0, 0, 0, 2], X, 3).bonus(&[GENERAL]),
        c("Security Automatons", Research, [0, 0, 4, 1, 0], [0; 5], G, 0).per_token(Token::General, 1),
        c("Super-Soldiers", Research, [0, 0, 7, 0, 0], [0; 5], X, 0).bonus(&[GENERAL]).per_token(Token::General, 1),
        c("Super-Sonar", Research, [0, 0, 4, 0, 0], [0; 5], X, 1).scaled(X, Vehicle),
        c("Supercomputer", Research, [0, 0, 4, 0, 0], [0, 0, 1, 0, 0], S, 0).combo(Vehicle, 1),
        c("Teleportation", Research, [0, 0, 8, 0, 0], [0; 5], X, 8).bonus(&[KRYSTALLIUM, KRYSTALLIUM]),
        c("Time Travel", Research, [0, 0, 5, 0, 0], [0; 5], X, 15).krystallium(3),
        c("Transmutation", Research, [0, 0, 3, 2, 0], [0, 0, 0, 3, 0], G, 1).bonus(&[KRYSTALLIUM]),
        c("Universal Vaccine", Research, [0, 0, 3, 0, 0], [0; 5], G, 0).combo(Project, 1),
        c("Unknown Technology", Research, [0, 0, 7, 0, 0], [0; 5], S, 0).krystallium(1).combo(Research, 3),
        c("Virtual Reality", Research, [0, 0, 5, 0, 0], [0; 5], G, 2).scaled(G, Research),

        // Projects (20 designs, 29 cards).
        c("Casino City", Project, [0, 3, 0, 4, 0], [0, 0, 0, 2, 0], G, 0).bonus(&[FINANCIER]).per_token(Token::Financier, 1).copies(2),
        c("Espionage Agency", Project, [0, 2, 0, 2, 0], [0, 0, 0, 0, 2], X, 1).copies(2),
        c("Giant Dam", Project, [3, 0, 0, 2, 0], [0, 4, 0, 0, 0], E, 1),
        c("Giant Tower", Project, [2, 0, 0, 3, 0], [0; 5], G, 10).chars(0, 1),
        c("Harbor Zone", Project, [0, 0, 0, 5, 0], [2, 0, 0, 2, 0], G, 2).bonus(&[FINANCIER, FINANCIER]).copies(2),
        c("Lunar Base", Project, [0, 2, 2, 2, 0], [0; 5], X, 10).krystallium(1).bonus(&[GENERAL, GENERAL]),
        c("Magnetic Train", Project, [0, 1, 1, 3, 0], [0; 5], G, 2).scaled(G, Structure).bonus(&[FINANCIER, FINANCIER]),
        c("Museum", Project, [0, 0, 0, 3, 0], [0; 5], X, 0).combo(Discovery, 2).copies(2),
        c("National Monument", Project, [5, 0, 0, 3, 0], [0; 5], G, 0).combo(Project, 2),
        c("Polar Base", Project, [0, 3, 0, 4, 0], [0, 0, 0, 0, 3], X, 0).bonus(&[GENERAL]).combo(Discovery, 2),
        c("Propaganda Center", Project, [0, 0, 0, 3, 0], [0; 5], G, 1).scaled(G, Project).bonus(&[GENERAL]).copies(2),
        c("Secret Laboratory", Project, [2, 0, 0, 3, 0], [0, 0, 2, 0, 0], S, 0).bonus(&[KRYSTALLIUM]).combo(Research, 1).copies(2),
        c("Secret Society", Project, [0, 0, 0, 3, 0], [0; 5], G, 0).krystallium(1).per_token(Token::Financier, 1).copies(2),
        c("Solar Cannon", Project, [0, 2, 1, 3, 0], [0; 5], E, 0).bonus(&[GENERAL]).per_token(Token::General, 1),
        c("Space Elevator", Project, [0, 3, 1, 2, 0], [0; 5], E, 0).bonus(&[FINANCIER]).per_token(Token::Financier, 1),
        c("Underground City", Project, [3, 0, 0, 3, 0], [2, 2, 0, 0, 0], E, 3).bonus(&[KRYSTALLIUM]).copies(2),
        c("Underwater City", Project, [0, 2, 1, 2, 0], [0, 0, 1, 0, 2], X, 3).copies(2),
        c("Universal Exposition", Project, [0, 0, 0, 3, 0], [0; 5], G, 0).chars(0, 2).combo(Research, 3),
        c("University", Project, [0, 0, 1, 2, 0], [0; 5], S, 2).scaled(S, Project),
        c("World Congress", Project, [0, 0, 0, 6, 0], [0; 5], G, 0).chars(0, 2).combo(Project, 3),

        // Discoveries (17 designs, 17 cards).
        c("Alexander's Tomb", Discovery, [0, 0, 0, 0, 7], [0; 5], G, 10).bonus(&[GENERAL, GENERAL]),
        c("Ancient Astronauts", Discovery, [0, 0, 0, 0, 6], [0; 5], S, 10).chars(1, 0).scaled(S, Discovery).bonus(&[KRYSTALLIUM, KRYSTALLIUM]),
        c("Ark of the Covenant", Discovery, [0, 0, 0, 0, 4], [0; 5], X, 5).bonus(&[KRYSTALLIUM]),
        c("Atlantis", Discovery, [0, 0, 0, 0, 7], [0; 5], G, 0).krystallium(1).per_token(Token::General, 2),
        c("Bermuda Triangle", Discovery, [0, 0, 0, 0, 4], [0, 0, 1, 0, 0], S, 4).bonus(&[KRYSTALLIUM]),
        c("Blackbeard's Treasure", Discovery, [0, 0, 0, 0, 3], [0, 0, 0, 1, 1], G, 2),
        c("Center of the Earth", Discovery, [0, 0, 0, 0, 5], [0; 5], X, 15).chars(2, 0),
        c("Cities of Gold", Discovery, [0, 0, 0, 0, 4], [0, 0, 0, 3, 0], G, 3),
        c("City of Agartha", Discovery, [0, 0, 0, 0, 4], [0, 0, 0, 0, 2], X, 0).krystallium(1).per_token(Token::General, 1),
        c("Fountain of Youth", Discovery, [0, 0, 0, 0, 7], [0; 5], E, 0).bonus(&[KRYSTALLIUM, KRYSTALLIUM, KRYSTALLIUM]).per_token(Token::General, 1),
        c("Gardens of the Hesperides", Discovery, [0, 0, 0, 0, 5], [0; 5], X, 0).combo(Project, 2),
        c("Island of Avalon", Discovery, [0, 0, 0, 0, 5], [0, 0, 1, 0, 0], S, 7),
        c("King Solomon's Mines", Discovery, [0, 0, 0, 0, 4], [0; 5], G, 2).scaled(G, Structure),
        c("Lost Continent of Mu", Discovery, [0, 0, 0, 0, 6], [0, 0, 0, 1, 0], G, 0).bonus(&[KRYSTALLIUM, KRYSTALLIUM]).combo(Discovery, 2),
        c("Parallel Dimension", Discovery, [0, 0, 3, 0, 4], [0; 5], X, 0).chars(1, 0).bonus(&[KRYSTALLIUM, KRYSTALLIUM, KRYSTALLIUM]).combo(Research, 3),
        c("Roswell", Discovery, [0, 0, 0, 0, 6], [0, 0, 1, 0, 0], S, 0).bonus(&[GENERAL]).per_token(Token::General, 1),
        c("Treasure of the Templars", Discovery, [0, 0, 0, 0, 5], [0, 0, 0, 2, 0], G, 3).bonus(&[KRYSTALLIUM, KRYSTALLIUM]),
    ];
    debug_assert_eq!(v.len(), DESIGNS);
    debug_assert_eq!(v.iter().map(|c| c.copies as usize).sum::<usize>(), DECK_SIZE);

    // Corruption & Ascension: a deck of its own, dealt beside the base one.
    let ascension = vec![
        // Structures (8 designs, 19 cards).
        c("Border Patrol", Structure, [2, 0, 0, 0, 0], [0, 0, 0, 1, 0], X, 0).corruption([0, 0, 0, 0, 1]).bonus(&[GENERAL]).copies(3),
        c("Krystallium Power Plant", Structure, [1, 0, 2, 0, 0], [3, 3, 0, 0, 0], S, 0).krystallium(1),
        c("Robot Factory", Structure, [2, 0, 2, 0, 0], [0, 2, 2, 0, 0], M, 0).corruption([1, 0, 0, 0, 0]).copies(2),
        c("Gold Mine", Structure, [3, 1, 0, 0, 1], [0, 0, 0, 3, 0], G, 0).copies(2),
        c("Secret Base", Structure, [4, 1, 1, 0, 0], [0, 0, 0, 0, 3], X, 0).corruption([0, 1, 0, 0, 0]).bonus(&[GENERAL]).copies(2),
        c("Lawless Zone", Structure, [2, 0, 0, 0, 0], [1, 1, 0, 0, 0], M, 0).corruption([0, 0, 0, 1, 0]).copies(3),
        c("Occult District", Structure, [1, 0, 0, 0, 0], [1, 0, 0, 0, 0], M, 0).corruption([0, 0, 1, 0, 0]).bonus(&[KRYSTALLIUM]).copies(3),
        c("Offshore Laboratory", Structure, [2, 0, 0, 1, 0], [0, 0, 1, 1, 0], E, 0).corruption([0, 1, 0, 0, 0]).bonus(&[FINANCIER]).copies(3),

        // Vehicles (6 designs, 12 cards).
        c("Floating Palace", Vehicle, [0, 2, 1, 2, 0], [0, 2, 0, 0, 0], E, 0).chars(1, 0).combo(Vehicle, 2),
        c("Giant Robot", Vehicle, [3, 5, 2, 0, 0], [0; 5], M, 0).krystallium(1).bonus(&[GENERAL, GENERAL]).duo(Structure, Vehicle, 6),
        c("Armored Convoy", Vehicle, [1, 2, 0, 0, 0], [0, 0, 0, 2, 0], G, 0).corruption([1, 0, 0, 0, 0]).copies(2),
        c("Mobile Base", Vehicle, [2, 3, 1, 0, 0], [0, 0, 2, 0, 2], S, 0).corruption([1, 0, 0, 0, 0]).bonus(&[GENERAL]).copies(2),
        c("Inquisitors", Vehicle, [1, 2, 0, 0, 0], [0, 0, 0, 1, 1], S, 0).corruption([0, 0, 1, 0, 0]).copies(3),
        c("Raiders", Vehicle, [0, 1, 0, 1, 0], [0, 0, 0, 0, 2], G, 0).corruption([0, 0, 0, 1, 0]).copies(3),

        // Research (8 designs, 8 cards).
        c("Planetary Archives", Research, [0, 0, 4, 0, 2], [0, 0, 2, 0, 0], X, 0).combo(Discovery, 3),
        c("Telekinesis", Research, [0, 0, 5, 0, 0], [0; 5], S, 0).krystallium(1).duo(Vehicle, Token::Financier, 6),
        c("Artificial Intelligence", Research, [0, 0, 3, 0, 0], [0; 5], S, 1).scaled(S, Vehicle),
        c("Dark Matter", Research, [5, 0, 5, 0, 0], [0; 5], M, 0).krystallium(2).duo(Structure, Research, 6),
        c("Artificial Sun", Research, [0, 4, 7, 2, 0], [0; 5], E, 25).krystallium(1),
        c("Immortality", Research, [0, 3, 8, 0, 0], [0; 5], S, 0).chars(1, 1).duo(Research, Project, 9),
        c("Utopia", Research, [2, 2, 2, 2, 2], [0; 5], G, 0).chars(0, 2).duo(Project, Discovery, 12),
        c("Tax Haven", Research, [0, 0, 2, 2, 0], [0, 0, 0, 2, 0], G, 1).corruption([0, 1, 0, 0, 0]).bonus(&[FINANCIER, FINANCIER]),

        // Projects (9 designs, 10 cards).
        c("Celestial Cathedral", Project, [4, 0, 0, 5, 2], [0; 5], G, 0).duo(Vehicle, Project, 8),
        c("Intercontinental Network", Project, [2, 2, 0, 3, 0], [0; 5], M, 0).duo(Discovery, Token::Financier, 6),
        c("High-Security Prison", Project, [2, 0, 0, 4, 0], [0; 5], M, 0).chars(0, 1).duo(Structure, Token::General, 5),
        c("Orbital Station", Project, [0, 3, 3, 5, 0], [0; 5], S, 0).duo(Token::Financier, Token::General, 6),
        c("World Bank", Project, [0, 0, 0, 7, 5], [0; 5], G, 0).chars(1, 0).duo(Structure, Project, 7),
        c("Luxury Clinic", Project, [0, 0, 1, 2, 0], [0, 0, 2, 0, 0], E, 3).corruption([0, 1, 0, 0, 0]).copies(2),
        c("The Wall", Project, [9, 0, 0, 4, 0], [0; 5], M, 25).chars(2, 0),
        c("Consortium", Project, [0, 3, 0, 3, 0], [0, 0, 0, 3, 2], G, 1).bonus(&[FINANCIER]),
        c("Mining Asteroid", Project, [0, 2, 2, 2, 0], [4, 0, 0, 0, 0], M, 3).bonus(&[KRYSTALLIUM, KRYSTALLIUM]),

        // Discoveries (7 designs, 7 cards).
        c("Hyperborea", Discovery, [0, 0, 4, 0, 5], [0; 5], S, 0).chars(2, 0).duo(Research, Discovery, 10),
        c("Valhalla", Discovery, [0, 0, 0, 2, 5], [0; 5], G, 0).krystallium(1).duo(Project, Token::General, 6),
        c("Mysterious Vessel", Discovery, [0, 0, 2, 0, 6], [0; 5], X, 0).duo(Vehicle, Research, 6),
        c("Pandemonium", Discovery, [0, 0, 0, 0, 4], [0; 5], E, 0).chars(0, 2).combo(Structure, 2),
        c("Pandora's Box", Discovery, [0, 0, 0, 0, 4], [1, 1, 1, 1, 0], X, 1).bonus(&[KRYSTALLIUM]),
        c("Alpha Centauri", Discovery, [0, 3, 3, 0, 8], [0; 5], X, 25),
        c("Shambhala", Discovery, [0, 0, 0, 0, 4], [0; 5], X, 4).scaled(X, Project),
    ];
    debug_assert_eq!(ascension.len(), ASCENSION_DESIGNS);
    debug_assert_eq!(ascension.iter().map(|c| c.copies as usize).sum::<usize>(), ASCENSION_DECK_SIZE);
    let mut v = v;
    v.extend(ascension.into_iter().map(|c| Card { ascension: true, ..c }));
    v
}
