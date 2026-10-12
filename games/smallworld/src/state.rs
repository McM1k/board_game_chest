//! The game state and its rules.
//!
//! Players take turns, clockwise from seat 0, for as many game turns as the
//! board says (10 with two or three players, 9 with four, 8 with five or
//! six).
//!
//! * **First turn**, and whenever a player has no active race: pick a race
//!   and power from the column of six. The top one is free; each one lower
//!   costs a coin more, dropped on the combos above it (3 on a Cursed one),
//!   and whoever takes a combo pockets the coins on it. The race comes with
//!   as many tokens as the banner and the badge add up to.
//! * **Conquest.** A region costs 2 tokens, plus 1 for each Mountain,
//!   Troll's Lair, Fortress, Encampment or Winter marker in it, plus 1 for
//!   each token defending it (a Lost Tribe, another player's race), less
//!   what the race and power take off (never below 1). A race enters the
//!   map through a border region, then conquers next to the regions it
//!   holds. Each defender loses one token for good (Elves none) and puts the
//!   others back on their own regions at the end of the turn. The last
//!   conquest of a turn can be tried with up to 3 tokens missing: the
//!   reinforcement die (0, 0, 0, 1, 2, 3) makes up the difference, or the
//!   conquests end there.
//! * **Following turns**: every region keeps one token, the rest come back
//!   in hand to conquer with. Regions can be abandoned for their tokens.
//!   Instead of conquering, a player can put their race **in decline**: one
//!   token stays in each region (all of them for Ghouls), its power is
//!   lost, any older race in decline leaves the map, and the turn ends. A
//!   new race is picked at the next turn.
//! * **Redeployment**: tokens move freely between the race's regions, at
//!   least one in each, and the power places its pieces.
//! * **Scoring**: a coin per region held by any of the player's races,
//!   active or in decline, plus the bonuses of the active race and power.
//!
//! The expansions the host switched on (see [`Options`]) add their races
//! and powers to the stacks; the Sky Islands add a second board (see
//! [`Board`]). Coins are hidden from the other players until the end. The
//! most coins win; a tie goes to whoever has the most tokens on the map.
//!
//! The rules are pure: the only randomness, the shuffles, the die and the
//! Skags' loot, comes from the seed the game is created with.

use crate::maps::{turns, Board, Map, Sky, Terrain};
use crate::races::{Pack, Power, Race};
use serde::{Deserialize, Serialize};

pub const MIN_PLAYERS: usize = 2;
/// The most players a game seats, with the Sky Islands (see
/// [`Options::max_players`]).
pub const MAX_PLAYERS: usize = 6;
/// Race and power combos face up in the column.
pub const OFFERS: usize = 6;
pub const START_COINS: u32 = 5;
/// The reinforcement die: three blank faces, then 1, 2 and 3.
pub const DIE: [u8; 6] = [0, 0, 0, 1, 2, 3];
/// The most tokens the die can make up for.
pub const DIE_MAX: u8 = 3;
/// Bivouacking's encampments.
pub const ENCAMPMENTS: u8 = 5;
/// Fortresses in the box: never more on the map.
pub const FORTRESSES: usize = 6;
/// Heroic's heroes.
pub const HEROES: usize = 2;
/// Halflings' Holes-in-the-Ground.
pub const HOLES: u8 = 2;
/// Amazons' tokens that only conquer.
pub const AMAZON_EXTRA: u8 = 4;
/// Wealthy's one-off bonus.
pub const WEALTHY_COINS: u32 = 7;
/// What it costs to skip a Cursed combo.
pub const CURSED_SKIP: u32 = 3;
/// Hordes of's tokens.
pub const HORDES: u8 = 2;
/// Leprechauns' Pots of Gold in the box.
pub const POTS: u8 = 16;
/// Fireball markers in the box.
pub const FIREBALLS: u8 = 4;
/// Lava tokens in the box.
pub const LAVA: u8 = 4;
/// Behemoths and Cannons.
pub const BEHEMOTHS: u8 = 2;
pub const CANNONS: usize = 2;
/// Drakons' Dragons, Storm Giants' Lightning Bolts, Zeppelins: each a turn.
pub const DRAKON_DRAGONS: u8 = 3;
pub const BOLTS: u8 = 2;
pub const ZEPPELINS: u8 = 5;
/// Haggling's Trade Pacts.
pub const PACTS: u8 = 5;
/// Barricade's bonus, and the most regions it allows.
pub const BARRICADE_COINS: u32 = 3;
pub const BARRICADE_REGIONS: usize = 4;
/// Peace-Loving's bonus.
pub const PEACE_COINS: u32 = 3;
/// How many events [`State::log`] keeps.
const LOG_LEN: usize = 40;

/// What the host picked in the lobby: the expansions in play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Options {
    #[serde(default)]
    pub cursed: bool,
    #[serde(default)]
    pub grand_dames: bool,
    #[serde(default)]
    pub be_not_afraid: bool,
    #[serde(default)]
    pub royal_bonus: bool,
    #[serde(default)]
    pub spiders_web: bool,
    /// The Sky Islands, on this side of their board.
    #[serde(default)]
    pub sky: Option<Sky>,
}

impl Options {
    /// Every expansion, the Sky Islands with three islands.
    pub fn all() -> Options {
        Options {
            cursed: true,
            grand_dames: true,
            be_not_afraid: true,
            royal_bonus: true,
            spiders_web: true,
            sky: Some(Sky::Three),
        }
    }

    pub fn has(&self, pack: Pack) -> bool {
        match pack {
            Pack::Base => true,
            Pack::Cursed => self.cursed,
            Pack::GrandDames => self.grand_dames,
            Pack::BeNotAfraid => self.be_not_afraid,
            Pack::RoyalBonus => self.royal_bonus,
            Pack::SpidersWeb => self.spiders_web,
            Pack::SkyIslands => self.sky.is_some(),
        }
    }

    /// Switches an expansion on or off (the Sky Islands on their three-island
    /// side, unless a side was already picked).
    pub fn set(&mut self, pack: Pack, on: bool) {
        match pack {
            Pack::Base => {}
            Pack::Cursed => self.cursed = on,
            Pack::GrandDames => self.grand_dames = on,
            Pack::BeNotAfraid => self.be_not_afraid = on,
            Pack::RoyalBonus => self.royal_bonus = on,
            Pack::SpidersWeb => self.spiders_web = on,
            Pack::SkyIslands => {
                let side = self.sky.unwrap_or(Sky::Three);
                self.sky = on.then_some(side);
            }
        }
    }

    /// The fewest players: the Sky Islands are played on the board for one
    /// player fewer, so with three at least.
    pub fn min_players(&self) -> usize {
        if self.sky.is_some() {
            3
        } else {
            MIN_PLAYERS
        }
    }

    /// The most players: six with the Sky Islands (on the five-player board).
    pub fn max_players(&self) -> usize {
        if self.sky.is_some() {
            6
        } else {
            5
        }
    }

    pub fn races(&self) -> Vec<Race> {
        Race::ALL.into_iter().filter(|r| self.has(r.pack())).collect()
    }

    pub fn powers(&self) -> Vec<Power> {
        Power::ALL.into_iter().filter(|p| self.has(p.pack())).collect()
    }
}

/// A race and a power face up in the column, with what was dropped on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub race: Race,
    pub power: Power,
    pub coins: u32,
    /// Homunculi: the tokens added each time the combo was skipped.
    #[serde(default)]
    pub tokens: u8,
}

/// A race a player controls, active or in decline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Army {
    pub race: Race,
    pub power: Power,
    /// Tokens in hand: to conquer with during the turn, or to put back on
    /// the map after losing a region.
    pub hand: u8,
    /// Tokens sitting out until the next turn (Amazons, Barbarians, Pixies).
    pub resting: u8,
    /// Turns finished with this race (Wealthy pays at the end of the first).
    pub turns: u8,
    /// Halflings: Holes-in-the-Ground placed so far (two, ever).
    pub holes: u8,
    /// Diplomat: the seat at peace with this race, who can't attack it
    /// until its owner's next turn.
    pub ally: Option<usize>,
    /// Copycat: the power copied from the column until the next turn.
    #[serde(default)]
    pub copied: Option<Power>,
    /// Hordes of: tokens on top of the race's own.
    #[serde(default)]
    pub extra: u8,
    /// Fireball markers held.
    #[serde(default)]
    pub fireballs: u8,
    /// Ice Witches: Winter markers waiting to be placed.
    #[serde(default)]
    pub winter: u8,
    /// Behemoths in hand (the others stand on the map).
    #[serde(default)]
    pub behemoths: u8,
    /// Igors: tokens collected from conquests, not traded in yet.
    #[serde(default)]
    pub collected: u8,
    /// Leprechauns: Pots of Gold used up.
    #[serde(default)]
    pub pots: u8,
}

impl Army {
    pub fn new(race: Race, power: Power, tokens: u8) -> Army {
        Army {
            race,
            power,
            hand: tokens,
            resting: 0,
            turns: 0,
            holes: 0,
            ally: None,
            copied: None,
            extra: 0,
            fireballs: 0,
            winter: 0,
            behemoths: 0,
            collected: 0,
            pots: 0,
        }
    }

    /// Has this power: its own, or the one it copies.
    pub fn has(&self, power: Power) -> bool {
        self.power == power || self.copied == Some(power)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    /// Victory coins (only their owner sees them before the end).
    pub coins: u32,
    pub active: Option<Army>,
    /// Races in decline: one, plus one with the Spirit power.
    pub declined: Vec<Army>,
    /// Racketeering: the next combo is free.
    #[serde(default)]
    pub free_pick: bool,
}

/// Race tokens standing in a region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tokens {
    pub race: Race,
    pub count: u8,
    pub declined: bool,
}

/// What is in a region right now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionState {
    pub tokens: Option<Tokens>,
    pub lost_tribe: bool,
    /// Trolls: +1 defense, stays in decline.
    pub lair: bool,
    /// Fortified: +1 defense, +1 coin a turn while active.
    pub fortress: bool,
    /// Bivouacking: +1 defense each.
    pub encampments: u8,
    /// Halflings: immune.
    pub hole: bool,
    /// Heroic: immune.
    pub hero: bool,
    /// Dragon Master or Drakons: immune.
    pub dragon: bool,
    /// Leprechauns: a Pot of Gold.
    #[serde(default)]
    pub pot: bool,
    /// The Catapult: immune.
    #[serde(default)]
    pub catapult: bool,
    /// Ice Witches: +1 defense, 1 coin less for anyone else holding it.
    #[serde(default)]
    pub winter: bool,
    /// Skags: a Loot marker lies here face down.
    #[serde(default)]
    pub loot: bool,
    /// Behemoths standing here.
    #[serde(default)]
    pub behemoths: u8,
    /// Gunner: a Cannon, firing or not. Immune.
    #[serde(default)]
    pub cannon: Option<bool>,
    /// Lava poured by this seat: nobody else may enter until its next turn.
    #[serde(default)]
    pub lava: Option<usize>,
    /// A crashed Zeppelin's fire, lit by this seat: immune until its next turn.
    #[serde(default)]
    pub burning: Option<usize>,
    /// Scavengers: the token of a race in decline kept in the region.
    #[serde(default)]
    pub captive: Option<Race>,
}

impl RegionState {
    /// Holds a Lost Tribe or race tokens (a Mountain alone is empty).
    pub fn occupied(&self) -> bool {
        self.lost_tribe || self.tokens.is_some()
    }

    /// The race whose tokens stand here.
    pub fn race(&self) -> Option<Race> {
        self.tokens.map(|t| t.race)
    }
}

/// What a Skags' Loot marker hides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Loot {
    Coins(u8),
    /// The conquest fails, and the attacker loses a token.
    SkagAttack,
}

/// The Skags' ten Loot markers.
const LOOT: [Loot; 10] = [
    Loot::Coins(0),
    Loot::Coins(0),
    Loot::Coins(1),
    Loot::Coins(1),
    Loot::Coins(2),
    Loot::Coins(2),
    Loot::Coins(3),
    Loot::SkagAttack,
    Loot::SkagAttack,
    Loot::SkagAttack,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// The current player picks a race and power from the column.
    Pick,
    /// The current player conquers: first with their Ghouls in decline
    /// (`ghouls`), then with their active race.
    Conquest { ghouls: bool },
    /// The current player moves their tokens around, places their power's
    /// pieces and ends the turn.
    Redeploy,
    /// Players who lost regions put the tokens they kept back on the map,
    /// one after the other (see [`State::reinforce`]).
    Reinforce,
    Over,
}

/// What the current turn has done so far.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Turn {
    /// Regions conquered this turn: their tokens stay put until redeploying.
    pub conquered: Vec<usize>,
    /// Non-empty regions the active race conquered (Orcs, Pillaging, Skeletons).
    pub plundered: u8,
    /// Seats whose active race was attacked (Diplomat can't ally with them;
    /// Peace-Loving pays only without any).
    pub attacked: Vec<usize>,
    /// Seats the Sorcerers already took a token from.
    pub enchanted: Vec<usize>,
    /// Dragon Master's Dragon has flown this turn.
    pub dragon: bool,
    /// This turn's fortress is built.
    pub fortress: bool,
    /// Berserk: the die rolled for the next conquest.
    pub berserk: Option<u8>,
    /// The turn began with an active race, which may go into decline.
    pub may_decline: bool,
    /// Something was conquered or abandoned: too late to go into decline.
    pub moved: bool,
    /// The Ghouls in decline conquered or abandoned something.
    pub ghouls_moved: bool,
    /// Tokens lifted off each region when the active race was readied, to
    /// put back if Ghouls go into decline (they keep them all).
    pub lifted: Vec<(usize, u8)>,
    /// The same for the Ghouls in decline, put back if they don't conquer.
    pub ghouls_lifted: Vec<(usize, u8)>,
    /// Marauding: the second round of conquests has begun.
    #[serde(default)]
    pub marauded: bool,
    /// Drakons' Dragons, Lightning Bolts and Zeppelins used this turn.
    #[serde(default)]
    pub drakons: u8,
    #[serde(default)]
    pub bolts: u8,
    #[serde(default)]
    pub zeppelins: u8,
    /// The Wendigos' clearing and the Catapult were used.
    #[serde(default)]
    pub wendigo: bool,
    #[serde(default)]
    pub catapult: bool,
    /// Lava poured this turn.
    #[serde(default)]
    pub lava: u8,
    /// Copycat chose its power.
    #[serde(default)]
    pub copied: bool,
    /// Regions a Skag Attack drove the player off: no second try.
    #[serde(default)]
    pub repelled: Vec<usize>,
    /// Gypsies: regions abandoned this turn (a coin each, no taking back).
    #[serde(default)]
    pub abandoned: Vec<usize>,
    /// Races whose regions the player took (Haggling).
    #[serde(default)]
    pub victims: Vec<Race>,
    /// Fauns: the seat whose active region they took, once per region.
    #[serde(default)]
    pub fauns: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Take the combo at this position of the column (0 is free).
    Pick { offer: usize },
    /// Put the active race in decline: at the start of the turn instead of
    /// conquering, or (Stout) at the end of a turn of conquests.
    Decline,
    /// Priestesses: go into decline, stacking the Ivory Tower here.
    Tower { region: usize },
    /// Conquer a region with the tokens it costs.
    Conquer { region: usize },
    /// The last conquest of the turn, with tokens missing: roll the die.
    Gamble { region: usize },
    /// Berserk: roll the die for the next conquest.
    Roll,
    /// Dragon Master or Drakons: conquer with one token, whatever defends
    /// the region.
    Dragon { region: usize },
    /// Sorcerers: replace a lone enemy token with one of yours.
    Enchant { region: usize },
    /// Mercenary: pay a coin to conquer with 2 tokens less.
    Hire { region: usize },
    /// Catapult: conquer a region two away with 1 token less.
    Catapult { region: usize },
    /// Fireball: throw Fireballs (2 tokens each) at a region.
    Fireball { region: usize },
    /// Storm Giants: a Lightning Bolt takes a Mountain with one token.
    Lightning { region: usize },
    /// Zeppelined: roll the die and conquer anywhere.
    Zeppelin { region: usize },
    /// Wendigos: clear a Forest region of its tokens.
    Wendigo { region: usize },
    /// Behemoth: attack with a Behemoth from hand.
    Behemoth { region: usize },
    /// Gunner: this Cannon fires this turn.
    Fire { region: usize },
    /// Copycat: copy this combo's power until the next turn (`None`: none).
    Copy { offer: Option<usize> },
    /// Empty one of your regions to take its tokens back in hand.
    Abandon { region: usize },
    /// Done conquering (with the Ghouls in decline, or the active race).
    EndConquests,
    /// Put a token from your hand in one of your regions.
    Put { region: usize },
    /// Take a token from one of your regions back in hand (one stays).
    Take { region: usize },
    /// Bivouacking: pitch an encampment in one of your regions.
    Camp { region: usize },
    /// Bivouacking: strike an encampment.
    Uncamp { region: usize },
    /// Fortified: build this turn's fortress.
    Fortify { region: usize },
    /// Heroic: put a Hero in one of your regions, or take it off again.
    Hero { region: usize },
    /// Leprechauns: put a Pot of Gold here, or take it off again.
    Pot { region: usize },
    /// Ice Witches: put a Winter marker in or next to one of your regions.
    Winter { region: usize },
    /// Lava: pour Lava next to one of your Mountains.
    Lava { region: usize },
    /// Behemoth: stand a Behemoth here, or take it back in hand.
    Herd { region: usize },
    /// Gunner: put a Cannon here, or take it off to move it.
    Cannon { region: usize },
    /// Diplomat: make peace with this seat until your next turn (`None`:
    /// nobody).
    Ally { seat: Option<usize> },
    /// Haggling: give this seat a Trade Pact, or take one back.
    Pact { seat: usize, give: bool },
    /// Score and end the turn.
    EndTurn,
}

/// The moves that act on a region, as the interface offers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegionMove {
    Conquer,
    Gamble,
    Hire,
    Fireball,
    Behemoth,
    Catapult,
    Dragon,
    Lightning,
    Zeppelin,
    Enchant,
    Wendigo,
    Fire,
    Abandon,
    Tower,
    Put,
    Take,
    Camp,
    Uncamp,
    Fortify,
    Hero,
    Pot,
    Winter,
    Lava,
    Herd,
    Cannon,
}

impl RegionMove {
    /// Every kind, in the order the interface lists them.
    pub const ALL: [RegionMove; 25] = [
        RegionMove::Conquer,
        RegionMove::Gamble,
        RegionMove::Hire,
        RegionMove::Fireball,
        RegionMove::Behemoth,
        RegionMove::Catapult,
        RegionMove::Dragon,
        RegionMove::Lightning,
        RegionMove::Zeppelin,
        RegionMove::Enchant,
        RegionMove::Wendigo,
        RegionMove::Fire,
        RegionMove::Abandon,
        RegionMove::Tower,
        RegionMove::Put,
        RegionMove::Take,
        RegionMove::Camp,
        RegionMove::Uncamp,
        RegionMove::Fortify,
        RegionMove::Hero,
        RegionMove::Pot,
        RegionMove::Winter,
        RegionMove::Lava,
        RegionMove::Herd,
        RegionMove::Cannon,
    ];

    pub fn action(self, region: usize) -> Action {
        match self {
            RegionMove::Conquer => Action::Conquer { region },
            RegionMove::Gamble => Action::Gamble { region },
            RegionMove::Hire => Action::Hire { region },
            RegionMove::Fireball => Action::Fireball { region },
            RegionMove::Behemoth => Action::Behemoth { region },
            RegionMove::Catapult => Action::Catapult { region },
            RegionMove::Dragon => Action::Dragon { region },
            RegionMove::Lightning => Action::Lightning { region },
            RegionMove::Zeppelin => Action::Zeppelin { region },
            RegionMove::Enchant => Action::Enchant { region },
            RegionMove::Wendigo => Action::Wendigo { region },
            RegionMove::Fire => Action::Fire { region },
            RegionMove::Abandon => Action::Abandon { region },
            RegionMove::Tower => Action::Tower { region },
            RegionMove::Put => Action::Put { region },
            RegionMove::Take => Action::Take { region },
            RegionMove::Camp => Action::Camp { region },
            RegionMove::Uncamp => Action::Uncamp { region },
            RegionMove::Fortify => Action::Fortify { region },
            RegionMove::Hero => Action::Hero { region },
            RegionMove::Pot => Action::Pot { region },
            RegionMove::Winter => Action::Winter { region },
            RegionMove::Lava => Action::Lava { region },
            RegionMove::Herd => Action::Herd { region },
            RegionMove::Cannon => Action::Cannon { region },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    GameOver,
    UnknownSeat,
    NotYourTurn,
    WrongPhase,
    NoSuchOffer,
    NotEnoughCoins,
    NoSuchRegion,
    NoRace,
    TooLateToDecline,
    AlreadyYours,
    Water,
    Immune,
    AtPeace,
    OutOfReach,
    NotABorder,
    NoTokens,
    NotEnoughTokens,
    EnoughTokens,
    TooFar,
    NotYourPower,
    PowerUsed,
    NotYours,
    StaysPut,
    LastToken,
    HandEmpty,
    TokensInHand,
    NotAlone,
    NoneLeft,
    RestAmazons,
    PlaceHeroes,
    PlaceCannons,
    NoEncampment,
    Attacked,
    NotHere,
    Abandoned,
    Repelled,
    NotYet,
    NoUse,
}

impl Error {
    /// What the error means, in words for the player.
    pub fn message(self) -> &'static str {
        match self {
            Error::GameOver => "The game is over.",
            Error::UnknownSeat => "You aren't seated in this game.",
            Error::NotYourTurn => "It isn't your turn.",
            Error::WrongPhase => "You can't do that right now.",
            Error::NoSuchOffer => "There's no such race in the column.",
            Error::NotEnoughCoins => "You don't have enough coins for that.",
            Error::NoSuchRegion => "There's no such region.",
            Error::NoRace => "You have no race to do that with.",
            Error::TooLateToDecline => "You can only go into decline before conquering anything this turn.",
            Error::AlreadyYours => "That region is already yours.",
            Error::Water => "Only a Seafaring race can conquer the seas and the lakes.",
            Error::Immune => "That region is immune.",
            Error::AtPeace => "You are at peace with that player until their next turn.",
            Error::OutOfReach => "That region isn't next to one of yours.",
            Error::NotABorder => "A race enters the map through a region on its edge.",
            Error::NoTokens => "You have no tokens left in hand.",
            Error::NotEnoughTokens => "You don't have enough tokens in hand to conquer that region.",
            Error::EnoughTokens => "You have enough tokens: no need for the die.",
            Error::TooFar => "Even the die can't make up that many tokens.",
            Error::NotYourPower => "Your race and power can't do that.",
            Error::PowerUsed => "You've already done that this turn.",
            Error::NotYours => "That isn't one of your regions.",
            Error::StaysPut => "Tokens that conquered this turn stay there until you redeploy.",
            Error::LastToken => "At least one token has to stay in each of your regions.",
            Error::HandEmpty => "You have no tokens in hand to put there.",
            Error::TokensInHand => "Put all the tokens in your hand on the map first.",
            Error::NotAlone => "Only a lone active token can be replaced.",
            Error::NoneLeft => "There are none of those left.",
            Error::RestAmazons => "Keep 4 Amazons in hand: they rest until your next turn.",
            Error::PlaceHeroes => "Place both your Heroes first.",
            Error::PlaceCannons => "Place both your Cannons first.",
            Error::NoEncampment => "There's no encampment there.",
            Error::Attacked => "You attacked that player this turn: no peace with them.",
            Error::NotHere => "That can't go there.",
            Error::Abandoned => "You abandoned that region this turn: you can't take it back yet.",
            Error::Repelled => "The Skags drove you off that region this turn.",
            Error::NotYet => "Finish your first round of conquests before the die.",
            Error::NoUse => "That wouldn't make the conquest any cheaper.",
        }
    }
}

/// Who defended a conquered region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Foe {
    LostTribe,
    Race { seat: usize, race: Race },
}

/// How a region was conquered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum How {
    Tokens,
    Dragon,
    Hire,
    Fireball,
    Behemoth,
    Catapult,
    Lightning,
    Zeppelin,
    Sling,
}

/// Why coins changed hands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Why {
    Ransacking,
    Corrupt,
    Scarecrows,
    Slingmen,
    Loot,
    Pot,
    Pact,
    Historian,
    Racketeering,
    Escargots,
}

/// Something that happened, kept (briefly) for the log. Seats are indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    TurnStarted(u8),
    Picked { seat: usize, race: Race, power: Power, paid: u32, gained: u32 },
    Conquered { seat: usize, race: Race, region: usize, tokens: u8, foe: Option<Foe>, how: How },
    /// The reinforcement die, for a last conquest.
    Gambled { seat: usize, region: usize, die: u8, won: bool },
    /// Berserk's die, before a conquest.
    Rolled { seat: usize, die: u8 },
    /// A Zeppelin's die; a blank burns the region.
    Zeppelin { seat: usize, region: usize, die: u8 },
    Enchanted { seat: usize, region: usize, victim: usize, race: Race },
    Abandoned { seat: usize, region: usize },
    /// A race got new tokens (Skeletons, Pygmies, Fauns, Igors, Wendigos).
    Raised { seat: usize, race: Race, tokens: u8 },
    Declined { seat: usize, race: Race },
    /// A race in decline left the map.
    Vanished { seat: usize, race: Race },
    /// Soul-Touch brought a race in decline back.
    Revived { seat: usize, race: Race },
    Scored { seat: usize, coins: u32 },
    /// Coins changed hands (`from: None`: from the bank).
    Paid { from: Option<usize>, to: usize, coins: u32, why: Why },
    Peace { seat: usize, ally: usize },
    /// A Skag Attack drove the seat off a region.
    SkagAttack { seat: usize, region: usize },
    Lava { seat: usize, region: usize },
    Wendigo { seat: usize, region: usize },
    Copied { seat: usize, power: Option<Power> },
    GameOver,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub options: Options,
    pub board: Board,
    pub players: Vec<Player>,
    pub regions: Vec<RegionState>,
    /// The column, top (free) first.
    pub offers: Vec<Offer>,
    /// Race banners still in the stack, the next one first.
    pub races: Vec<Race>,
    /// Special Power badges still in the stack, the next one first.
    pub powers: Vec<Power>,
    pub discarded_powers: Vec<Power>,
    /// 1 to the board's number of turns.
    pub turn: u8,
    /// Whose turn it is.
    pub current: usize,
    pub phase: Phase,
    pub this_turn: Turn,
    /// In [`Phase::Reinforce`]: the seats still to put tokens back, the one
    /// acting first.
    pub reinforce: Vec<usize>,
    /// The latest events, oldest first.
    pub log: Vec<Event>,
    /// What each region's Loot marker hides (only the Skags' owner sees).
    pub loot: Vec<Option<Loot>>,
    /// The Loot markers not drawn yet.
    pub loot_bag: Vec<Loot>,
    /// Haggling: the Trade Pacts each seat holds.
    pub pacts: Vec<u8>,
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

/// The fewest tokens a race may hold a region with.
fn min_tokens(race: Race, declined: bool) -> u8 {
    if race == Race::Kobolds && !declined {
        2
    } else {
        1
    }
}

impl State {
    /// A new base game for 2 to 5 players, on the board for that many. The
    /// same seed gives the same game.
    pub fn new(players: usize, seed: u64) -> State {
        State::with_options(players, seed, Options::default())
    }

    /// A new game with the lobby's expansions. The same seed gives the same
    /// game.
    pub fn with_options(players: usize, seed: u64, options: Options) -> State {
        let (min, max) = (options.min_players(), options.max_players());
        assert!((min..=max).contains(&players), "the game is for {min} to {max} players");
        let mut rng = seed;
        let mut races = options.races();
        shuffle(&mut races, &mut rng);
        let mut powers = options.powers();
        shuffle(&mut powers, &mut rng);
        let ground = if options.sky.is_some() { players - 1 } else { players };
        let access = match options.sky {
            Some(_) => access_points(Map::for_players(ground), &mut rng),
            None => Vec::new(),
        };
        let board = Board::new(ground, options.sky, &access);
        let regions = board
            .regions
            .iter()
            .map(|r| RegionState { lost_tribe: r.lost_tribe(), ..RegionState::default() })
            .collect();
        let mut loot_bag = LOOT.to_vec();
        shuffle(&mut loot_bag, &mut rng);
        let mut state = State {
            options,
            loot: vec![None; board.len()],
            board,
            players: vec![Player { coins: START_COINS, active: None, declined: Vec::new(), free_pick: false }; players],
            regions,
            offers: Vec::new(),
            races,
            powers,
            discarded_powers: Vec::new(),
            turn: 1,
            current: 0,
            phase: Phase::Pick,
            this_turn: Turn::default(),
            reinforce: Vec::new(),
            log: Vec::new(),
            loot_bag,
            pacts: vec![0; players],
            rng,
        };
        state.refill();
        state.push(Event::TurnStarted(1));
        state.begin_turn();
        state
    }

    /// For trying a combo out: puts `race` and `power` (either, or both) on
    /// the top combo of the column, the free one, swapping them with what
    /// was there. Meant for a game nobody has picked in yet. Returns false,
    /// and changes nothing, if one of them isn't in this game.
    pub fn put_on_top(&mut self, race: Option<Race>, power: Option<Power>) -> bool {
        let has_race = |r: Race| self.races.contains(&r) || self.offers.iter().any(|o| o.race == r);
        let has_power = |p: Power| self.powers.contains(&p) || self.offers.iter().any(|o| o.power == p);
        if self.offers.is_empty() || !race.is_none_or(has_race) || !power.is_none_or(has_power) {
            return false;
        }
        let top = self.offers[0];
        if let Some(race) = race {
            if let Some(r) = self.races.iter_mut().find(|r| **r == race) {
                *r = top.race;
            } else if let Some(o) = self.offers.iter_mut().find(|o| o.race == race) {
                o.race = top.race;
            }
            self.offers[0].race = race;
        }
        if let Some(power) = power {
            if let Some(p) = self.powers.iter_mut().find(|p| **p == power) {
                *p = top.power;
            } else if let Some(o) = self.offers.iter_mut().find(|o| o.power == power) {
                o.power = top.power;
            }
            self.offers[0].power = power;
        }
        true
    }

    pub fn seats(&self) -> usize {
        self.players.len()
    }

    /// How many game turns this game lasts.
    pub fn turns(&self) -> u8 {
        turns(self.seats())
    }

    /// Who has to act next: the current player, or someone putting lost
    /// tokens back. Nobody once the game is over.
    pub fn actor(&self) -> Option<usize> {
        match self.phase {
            Phase::Over => None,
            Phase::Reinforce => self.reinforce.first().copied(),
            _ => Some(self.current),
        }
    }

    // ----- who owns what -----------------------------------------------------------

    /// The seat playing `race`, and whether it is in decline.
    pub fn owner(&self, race: Race) -> Option<(usize, bool)> {
        self.players.iter().enumerate().find_map(|(seat, p)| {
            if p.active.is_some_and(|a| a.race == race) {
                Some((seat, false))
            } else if p.declined.iter().any(|a| a.race == race) {
                Some((seat, true))
            } else {
                None
            }
        })
    }

    fn army(&self, seat: usize, race: Race) -> Option<&Army> {
        let p = &self.players[seat];
        p.active.iter().chain(&p.declined).find(|a| a.race == race)
    }

    fn army_mut(&mut self, seat: usize, race: Race) -> Option<&mut Army> {
        let p = &mut self.players[seat];
        p.active.iter_mut().chain(p.declined.iter_mut()).find(|a| a.race == race)
    }

    /// The active army playing `race`, with its seat.
    fn active_race(&self, race: Race) -> Option<(usize, Army)> {
        self.players.iter().enumerate().find_map(|(s, p)| p.active.filter(|a| a.race == race).map(|a| (s, a)))
    }

    /// Whether `seat`'s active race has `power` (its own or copied).
    fn active_has(&self, seat: usize, power: Power) -> bool {
        self.players[seat].active.is_some_and(|a| a.has(power))
    }

    /// The regions `race` holds.
    pub fn regions_of(&self, race: Race) -> Vec<usize> {
        (0..self.regions.len()).filter(|&r| self.regions[r].race() == Some(race)).collect()
    }

    /// Tokens of `race` on the map (Scavengers' captives included).
    pub fn on_map(&self, race: Race) -> u8 {
        self.regions
            .iter()
            .map(|r| r.tokens.filter(|t| t.race == race).map_or(0, |t| t.count) + (r.captive == Some(race)) as u8)
            .sum()
    }

    /// Tokens of `race` still in the box.
    pub fn available(&self, race: Race) -> u8 {
        let (held, extra) =
            self.owner(race).and_then(|(seat, _)| self.army(seat, race)).map_or((0, 0), |a| (a.hand + a.resting, a.extra));
        let offered: u8 = self.offers.iter().filter(|o| o.race == race).map(|o| o.tokens).sum();
        (race.supply() + extra).saturating_sub(self.on_map(race) + held + offered)
    }

    /// The Ghouls in decline that conquer at the start of `seat`'s turns.
    fn decline_ghouls(&self, seat: usize) -> Option<&Army> {
        self.players[seat].declined.iter().find(|a| a.race == Race::Ghouls)
    }

    /// The race conquering in the current phase.
    fn conqueror(&self, seat: usize) -> Option<(Army, bool)> {
        match self.phase {
            Phase::Conquest { ghouls: true } => self.decline_ghouls(seat).map(|a| (*a, true)),
            Phase::Conquest { ghouls: false } => self.players[seat].active.map(|a| (a, false)),
            _ => None,
        }
    }

    /// What a Behemoth of `race` is worth: the Swamps the race holds.
    fn behemoth_strength(&self, race: Race) -> u8 {
        self.regions_of(race).iter().filter(|&&r| self.board.regions[r].terrain == Terrain::Swamp).count() as u8
    }

    // ----- conquest --------------------------------------------------------------------

    /// Next to one of `army`'s regions (with Underworld, every Cavern is
    /// next to every other).
    fn next_to(&self, army: &Army, declined: bool, region: usize) -> bool {
        let held = self.regions_of(army.race);
        held.iter().any(|&h| self.board.adjacent(h, region))
            || (!declined
                && army.has(Power::Underworld)
                && self.board.regions[region].cavern()
                && held.iter().any(|&h| self.board.regions[h].cavern()))
    }

    /// Whether `army` can reach `region`: through the border when it holds
    /// nothing yet, next to its regions otherwise. `Some(true)` for a
    /// Slingmen's shot two regions away.
    fn reach(&self, army: &Army, declined: bool, region: usize) -> Result<bool, Error> {
        let held = self.regions_of(army.race);
        let spot = &self.board.regions[region];
        let flying = !declined && army.has(Power::Flying);
        if held.is_empty() {
            let anywhere = !declined && (army.race == Race::Halflings || flying);
            let sky = !declined && army.race == Race::StormGiants && spot.sky;
            return if (anywhere && !spot.sky) || sky || self.board.border(region) {
                Ok(false)
            } else {
                Err(Error::NotABorder)
            };
        }
        if flying || self.next_to(army, declined, region) {
            return Ok(false);
        }
        if !declined && army.race == Race::Slingmen && held.iter().any(|&h| self.board.two_away(h, region)) {
            return Ok(true);
        }
        Err(Error::OutOfReach)
    }

    /// Whether `seat` may not touch `region`: immune to conquests and to the
    /// races' and powers' tricks.
    fn immune(&self, seat: usize, region: usize) -> bool {
        let r = &self.regions[region];
        let theirs = r.tokens.and_then(|t| self.owner(t.race)).is_some_and(|(s, _)| s != seat);
        r.hole
            || r.hero
            || r.dragon
            || r.catapult
            || r.cannon.is_some()
            || r.burning.is_some()
            || r.lava.is_some_and(|s| s != seat)
            || (theirs && r.tokens.is_some_and(|t| t.race == Race::WhiteLadies && t.declined))
            || (theirs
                && r.tokens.is_some_and(|t| t.race == Race::Shrubmen)
                && self.board.regions[region].terrain == Terrain::Forest)
    }

    /// Whether `region`'s active race made peace with `seat`.
    fn at_peace(&self, seat: usize, region: usize) -> bool {
        let Some(t) = self.regions[region].tokens.filter(|t| !t.declined) else { return false };
        let Some((owner, _)) = self.owner(t.race) else { return false };
        self.army(owner, t.race).is_some_and(|a| a.has(Power::Diplomat) && a.ally == Some(seat))
    }

    /// Checks that `seat`'s conquering race may attack `region` at all
    /// (whose region it is, the water, immunity, peace), wherever it is.
    fn target_anywhere(&self, seat: usize, army: &Army, declined: bool, region: usize) -> Result<(), Error> {
        let r = self.regions.get(region).ok_or(Error::NoSuchRegion)?;
        if r.race() == Some(army.race) {
            return Err(Error::AlreadyYours);
        }
        if self.board.regions[region].terrain.is_water() && (declined || !army.has(Power::Seafaring)) {
            return Err(Error::Water);
        }
        if self.immune(seat, region) {
            return Err(Error::Immune);
        }
        if self.this_turn.abandoned.contains(&region) {
            return Err(Error::Abandoned);
        }
        if self.this_turn.repelled.contains(&region) {
            return Err(Error::Repelled);
        }
        if !declined && self.at_peace(seat, region) {
            return Err(Error::AtPeace);
        }
        Ok(())
    }

    /// The same, and that it is within reach.
    fn target(&self, seat: usize, army: &Army, declined: bool, region: usize) -> Result<bool, Error> {
        self.target_anywhere(seat, army, declined, region)?;
        self.reach(army, declined, region)
    }

    /// What conquering `region` costs `seat`'s conquering race, before any
    /// die: 2, plus its defenses and defenders, less the race's and power's
    /// discounts, at least 1 (2 for Kobolds).
    fn cost(&self, seat: usize, army: &Army, declined: bool, region: usize) -> u8 {
        let spot = &self.board.regions[region];
        let r = &self.regions[region];
        let mut cost = 2
            + spot.mountain() as u8
            + r.lair as u8
            + r.fortress as u8
            + r.encampments
            + r.winter as u8
            + r.captive.is_some() as u8
            + r.lost_tribe as u8;
        if let Some(t) = r.tokens {
            if self.owner(t.race).map(|(s, _)| s) != Some(seat) {
                cost += t.count;
            }
            cost += r.behemoths * self.behemoth_strength(t.race);
        }
        if !declined {
            let mut off = 0;
            off += army.has(Power::Commando) as u8;
            off += (army.has(Power::Mounted) && matches!(spot.terrain, Terrain::Hill | Terrain::Farmland)) as u8;
            off += (army.has(Power::Underworld) && spot.cavern()) as u8;
            off += 2 * (army.has(Power::Were) && self.turn.is_multiple_of(2)) as u8;
            off += 2 * (army.has(Power::Airborne) && army.turns == 0) as u8;
            off += (army.race == Race::Tritons && self.board.coastal(region)) as u8;
            off += (army.race == Race::Goblins && r.tokens.is_some_and(|t| t.declined)) as u8;
            let held = self.regions_of(army.race);
            if army.race == Race::Giants {
                let near = held.iter().any(|&h| self.board.regions[h].mountain() && self.board.adjacent(h, region));
                off += near as u8;
            }
            if army.has(Power::Gunner) {
                let guns = held
                    .iter()
                    .filter(|&&h| self.regions[h].cannon == Some(true) && self.board.adjacent(h, region))
                    .count();
                off += 2 * guns as u8;
            }
            cost = cost.saturating_sub(off);
        }
        cost.max(min_tokens(army.race, declined))
    }

    /// The cost of conquering `region` now, if it can be attacked at all
    /// (Berserk's die already taken off).
    pub fn conquest_cost(&self, seat: usize, region: usize) -> Result<u8, Error> {
        let (army, declined) = self.conqueror(seat).ok_or(Error::NoRace)?;
        self.target(seat, &army, declined, region)?;
        Ok(self.die_cost(&army, declined, self.cost(seat, &army, declined, region)))
    }

    /// Berserk's die taken off a cost.
    fn die_cost(&self, army: &Army, declined: bool, cost: u8) -> u8 {
        match self.this_turn.berserk {
            Some(die) if !declined => cost.saturating_sub(die).max(min_tokens(army.race, declined)),
            _ => cost,
        }
    }

    /// Checks a move on a region and says what it costs in tokens (0 when
    /// it costs none). Used by [`State::apply`] and [`State::moves`].
    fn check(&self, seat: usize, kind: RegionMove, region: usize) -> Result<u8, Error> {
        if region >= self.regions.len() {
            return Err(Error::NoSuchRegion);
        }
        match kind {
            RegionMove::Conquer => {
                let (army, declined) = self.conqueror(seat).ok_or(Error::WrongPhase)?;
                self.target(seat, &army, declined, region)?;
                let cost = self.die_cost(&army, declined, self.cost(seat, &army, declined, region));
                self.affordable(&army, cost)
            }
            RegionMove::Gamble => {
                let (army, declined) = self.conqueror(seat).ok_or(Error::WrongPhase)?;
                if !declined && army.has(Power::Berserk) {
                    return Err(Error::NotYourPower);
                }
                if !declined && army.has(Power::Marauding) && !self.this_turn.marauded {
                    return Err(Error::NotYet);
                }
                self.target(seat, &army, declined, region)?;
                let cost = self.cost(seat, &army, declined, region);
                if army.hand == 0 {
                    return Err(Error::NoTokens);
                }
                if army.hand >= cost {
                    return Err(Error::EnoughTokens);
                }
                if cost - army.hand > DIE_MAX {
                    return Err(Error::TooFar);
                }
                if army.hand < min_tokens(army.race, declined) {
                    return Err(Error::NotEnoughTokens);
                }
                Ok(cost)
            }
            RegionMove::Hire => {
                let army = self.active_conqueror(seat, Power::Mercenary)?;
                if self.players[seat].coins == 0 {
                    return Err(Error::NotEnoughCoins);
                }
                self.target(seat, &army, false, region)?;
                let cost = self.die_cost(&army, false, self.cost(seat, &army, false, region));
                let hired = cost.saturating_sub(2).max(min_tokens(army.race, false));
                if hired == cost {
                    return Err(Error::NoUse);
                }
                self.affordable(&army, hired)
            }
            RegionMove::Fireball => {
                let army = self.active_conqueror(seat, Power::Fireball)?;
                if army.fireballs == 0 {
                    return Err(Error::NoneLeft);
                }
                self.target(seat, &army, false, region)?;
                let cost = self.die_cost(&army, false, self.cost(seat, &army, false, region));
                let (tokens, used) = self.fireballs(&army, cost);
                if used == 0 {
                    return Err(Error::NoUse);
                }
                self.affordable(&army, tokens)
            }
            RegionMove::Behemoth => {
                let army = self.active_conqueror(seat, Power::Behemoth)?;
                if army.behemoths == 0 {
                    return Err(Error::NoneLeft);
                }
                let strength = self.behemoth_strength(army.race);
                if strength == 0 {
                    return Err(Error::NoUse);
                }
                self.target(seat, &army, false, region)?;
                let cost = self.die_cost(&army, false, self.cost(seat, &army, false, region));
                self.affordable(&army, cost.saturating_sub(strength).max(min_tokens(army.race, false)))
            }
            RegionMove::Catapult => {
                let army = self.active_conqueror(seat, Power::Catapult)?;
                if self.this_turn.catapult {
                    return Err(Error::PowerUsed);
                }
                self.target_anywhere(seat, &army, false, region)?;
                if self.launcher(&army, region).is_none() {
                    return Err(Error::OutOfReach);
                }
                let cost = self.die_cost(&army, false, self.cost(seat, &army, false, region));
                self.affordable(&army, cost.saturating_sub(1).max(min_tokens(army.race, false)))
            }
            RegionMove::Dragon => {
                let (army, _) = self.conqueror(seat).filter(|(_, d)| !d).ok_or(Error::WrongPhase)?;
                let master = army.has(Power::DragonMaster) && !self.this_turn.dragon;
                let drakons = army.race == Race::Drakons && self.this_turn.drakons < DRAKON_DRAGONS;
                if !master && !drakons {
                    return Err(if army.has(Power::DragonMaster) || army.race == Race::Drakons {
                        Error::PowerUsed
                    } else {
                        Error::NotYourPower
                    });
                }
                self.target(seat, &army, false, region)?;
                // A Drakon turns one more token into the Dragon.
                let tokens = min_tokens(army.race, false);
                self.affordable(&army, if master { tokens } else { tokens + 1 })
            }
            RegionMove::Lightning => {
                let (army, _) = self.conqueror(seat).filter(|(_, d)| !d).ok_or(Error::WrongPhase)?;
                if army.race != Race::StormGiants {
                    return Err(Error::NotYourPower);
                }
                if self.this_turn.bolts >= BOLTS {
                    return Err(Error::PowerUsed);
                }
                if !self.board.regions[region].mountain() {
                    return Err(Error::NotHere);
                }
                self.target(seat, &army, false, region)?;
                self.affordable(&army, 1)
            }
            RegionMove::Zeppelin => {
                let army = self.active_conqueror(seat, Power::Zeppelined)?;
                if self.this_turn.zeppelins >= ZEPPELINS {
                    return Err(Error::PowerUsed);
                }
                self.target_anywhere(seat, &army, false, region)?;
                if army.hand == 0 {
                    return Err(Error::NoTokens);
                }
                Ok(self.cost(seat, &army, false, region))
            }
            RegionMove::Enchant => {
                let (army, _) = self.conqueror(seat).filter(|(_, d)| !d).ok_or(Error::WrongPhase)?;
                self.enchant_target(seat, &army, region).map(|_| 1)
            }
            RegionMove::Wendigo => {
                let (army, _) = self.conqueror(seat).filter(|(_, d)| !d).ok_or(Error::WrongPhase)?;
                if army.race != Race::Wendigos {
                    return Err(Error::NotYourPower);
                }
                if self.this_turn.wendigo {
                    return Err(Error::PowerUsed);
                }
                if self.board.regions[region].terrain != Terrain::Forest {
                    return Err(Error::NotHere);
                }
                let t = self.regions[region].tokens.ok_or(Error::NotHere)?;
                if t.race == army.race {
                    return Err(Error::AlreadyYours);
                }
                if self.immune(seat, region) {
                    return Err(Error::Immune);
                }
                if self.at_peace(seat, region) {
                    return Err(Error::AtPeace);
                }
                Ok(0)
            }
            RegionMove::Fire => {
                let army = self.active_conqueror(seat, Power::Gunner)?;
                let r = &self.regions[region];
                if r.race() != Some(army.race) || r.cannon.is_none() {
                    return Err(Error::NotHere);
                }
                if r.cannon == Some(true) {
                    return Err(Error::PowerUsed);
                }
                Ok(0)
            }
            RegionMove::Abandon => {
                let (army, _) = self.conqueror(seat).ok_or(Error::WrongPhase)?;
                if self.regions[region].race() != Some(army.race) {
                    return Err(Error::NotYours);
                }
                if self.this_turn.conquered.contains(&region) {
                    return Err(Error::StaysPut);
                }
                Ok(0)
            }
            RegionMove::Tower => {
                if !matches!(self.phase, Phase::Conquest { .. }) {
                    return Err(Error::WrongPhase);
                }
                let army = self.players[seat].active.ok_or(Error::NoRace)?;
                if army.race != Race::Priestesses {
                    return Err(Error::NotYourPower);
                }
                if !self.this_turn.may_decline || self.this_turn.moved {
                    return Err(Error::TooLateToDecline);
                }
                if self.regions[region].race() != Some(army.race) {
                    return Err(Error::NotYours);
                }
                Ok(0)
            }
            RegionMove::Put => {
                if !matches!(self.phase, Phase::Redeploy | Phase::Reinforce) {
                    return Err(Error::WrongPhase);
                }
                let race = self.regions[region].race().ok_or(Error::NotYours)?;
                let army = *self.army(seat, race).ok_or(Error::NotYours)?;
                let declined = self.owner(race).is_some_and(|(_, d)| d);
                if self.phase == Phase::Redeploy {
                    // In decline, only Ghouls redeploy on their own turn;
                    // Barbarians and Pixies never do.
                    if declined && race != Race::Ghouls {
                        return Err(Error::NotYours);
                    }
                    if !declined && matches!(race, Race::Barbarians | Race::Pixies) {
                        return Err(Error::NotYourPower);
                    }
                }
                if army.hand == 0 {
                    return Err(Error::HandEmpty);
                }
                Ok(1)
            }
            RegionMove::Take => {
                if self.phase != Phase::Redeploy {
                    return Err(Error::WrongPhase);
                }
                let t = self.regions[region].tokens.ok_or(Error::NotYours)?;
                let mine = self.owner(t.race).is_some_and(|(s, d)| s == seat && (!d || t.race == Race::Ghouls));
                if !mine {
                    return Err(Error::NotYours);
                }
                if !t.declined && matches!(t.race, Race::Barbarians | Race::Pixies) {
                    return Err(Error::NotYourPower);
                }
                if t.count <= min_tokens(t.race, t.declined) {
                    return Err(Error::LastToken);
                }
                Ok(1)
            }
            RegionMove::Camp => {
                self.own_redeploy_region(seat, region, Power::Bivouacking)?;
                let pitched: u8 = self.regions.iter().map(|r| r.encampments).sum();
                if pitched >= ENCAMPMENTS {
                    return Err(Error::NoneLeft);
                }
                Ok(0)
            }
            RegionMove::Uncamp => {
                self.own_redeploy_region(seat, region, Power::Bivouacking)?;
                if self.regions[region].encampments == 0 {
                    return Err(Error::NoEncampment);
                }
                Ok(0)
            }
            RegionMove::Fortify => {
                self.own_redeploy_region(seat, region, Power::Fortified)?;
                if self.this_turn.fortress {
                    return Err(Error::PowerUsed);
                }
                if self.regions[region].fortress {
                    return Err(Error::NotHere);
                }
                if self.regions.iter().filter(|r| r.fortress).count() >= FORTRESSES {
                    return Err(Error::NoneLeft);
                }
                Ok(0)
            }
            RegionMove::Hero => {
                self.own_redeploy_region(seat, region, Power::Heroic)?;
                if !self.regions[region].hero && self.heroes(seat) >= HEROES {
                    return Err(Error::NoneLeft);
                }
                Ok(0)
            }
            RegionMove::Pot => {
                let army = self.own_redeploy_race(seat, region, Race::Leprechauns)?;
                let placed = self.regions_of(army.race).iter().filter(|&&r| self.regions[r].pot).count() as u8;
                if !self.regions[region].pot && army.pots + placed >= POTS {
                    return Err(Error::NoneLeft);
                }
                Ok(0)
            }
            RegionMove::Winter => {
                if self.phase != Phase::Redeploy {
                    return Err(Error::WrongPhase);
                }
                let army = self.players[seat].active.filter(|a| a.race == Race::IceWitches).ok_or(Error::NotYourPower)?;
                if army.winter == 0 {
                    return Err(Error::NoneLeft);
                }
                if self.regions[region].winter || self.board.regions[region].terrain.is_water() {
                    return Err(Error::NotHere);
                }
                let mine = self.regions[region].race() == Some(army.race);
                if !mine && !self.next_to(&army, false, region) {
                    return Err(Error::OutOfReach);
                }
                Ok(0)
            }
            RegionMove::Lava => {
                if self.phase != Phase::Redeploy {
                    return Err(Error::WrongPhase);
                }
                let army = self.players[seat].active.filter(|a| a.has(Power::Lava)).ok_or(Error::NotYourPower)?;
                let mountains: Vec<usize> =
                    self.regions_of(army.race).into_iter().filter(|&r| self.board.regions[r].mountain()).collect();
                if self.this_turn.lava as usize >= mountains.len().min(LAVA as usize) {
                    return Err(Error::NoneLeft);
                }
                if !mountains.iter().any(|&m| self.board.adjacent(m, region)) {
                    return Err(Error::OutOfReach);
                }
                let r = &self.regions[region];
                if r.race() == Some(army.race) {
                    return Err(Error::AlreadyYours);
                }
                if r.lava.is_some() || self.board.regions[region].terrain.is_water() {
                    return Err(Error::NotHere);
                }
                if self.immune(seat, region) {
                    return Err(Error::Immune);
                }
                if self.at_peace(seat, region) {
                    return Err(Error::AtPeace);
                }
                Ok(0)
            }
            RegionMove::Herd => {
                let army = self.own_redeploy_region(seat, region, Power::Behemoth)?;
                if self.regions[region].behemoths == 0 && army.behemoths == 0 {
                    return Err(Error::NoneLeft);
                }
                Ok(0)
            }
            RegionMove::Cannon => {
                let army = self.own_redeploy_region(seat, region, Power::Gunner)?;
                match self.regions[region].cannon {
                    Some(true) => Err(Error::PowerUsed),
                    Some(false) => Ok(0),
                    None if self.cannons(army.race) >= CANNONS => Err(Error::NoneLeft),
                    None => Ok(0),
                }
            }
        }
    }

    /// The active race conquering now, which must have `power`.
    fn active_conqueror(&self, seat: usize, power: Power) -> Result<Army, Error> {
        if self.phase != (Phase::Conquest { ghouls: false }) {
            return Err(Error::WrongPhase);
        }
        let army = self.players[seat].active.ok_or(Error::NoRace)?;
        if !army.has(power) {
            return Err(Error::NotYourPower);
        }
        Ok(army)
    }

    /// Whether `army` has `cost` tokens in hand.
    fn affordable(&self, army: &Army, cost: u8) -> Result<u8, Error> {
        if army.hand == 0 {
            Err(Error::NoTokens)
        } else if army.hand < cost {
            Err(Error::NotEnoughTokens)
        } else {
            Ok(cost)
        }
    }

    /// Fireballs to throw at a region costing `cost`: the fewest that make
    /// it as cheap as it gets. The tokens still needed, and the Fireballs.
    fn fireballs(&self, army: &Army, cost: u8) -> (u8, u8) {
        let floor = min_tokens(army.race, false);
        let needed = cost.saturating_sub(floor).div_ceil(2);
        let used = needed.min(army.fireballs);
        (cost.saturating_sub(2 * used).max(floor), used)
    }

    /// Where the Catapult fires at `region` from: one of `army`'s regions
    /// two away from it, the most exposed one (fewest tokens).
    fn launcher(&self, army: &Army, region: usize) -> Option<usize> {
        self.regions_of(army.race)
            .into_iter()
            .filter(|&h| self.board.two_away(h, region))
            .min_by_key(|&h| (self.regions[h].tokens.map_or(0, |t| t.count), h))
    }

    /// Heroes standing in `seat`'s active race's regions.
    fn heroes(&self, seat: usize) -> usize {
        let Some(a) = self.players[seat].active else { return 0 };
        self.regions_of(a.race).iter().filter(|&&r| self.regions[r].hero).count()
    }

    /// Cannons standing in `race`'s regions.
    fn cannons(&self, race: Race) -> usize {
        self.regions_of(race).iter().filter(|&&r| self.regions[r].cannon.is_some()).count()
    }

    /// Checks that `region` is held by `seat`'s active race, whose power
    /// (its own or copied) is `power`, while redeploying.
    fn own_redeploy_region(&self, seat: usize, region: usize, power: Power) -> Result<Army, Error> {
        if self.phase != Phase::Redeploy {
            return Err(Error::WrongPhase);
        }
        let army = self.players[seat].active.ok_or(Error::NoRace)?;
        if !army.has(power) {
            return Err(Error::NotYourPower);
        }
        if self.regions[region].race() == Some(army.race) {
            Ok(army)
        } else {
            Err(Error::NotYours)
        }
    }

    /// The same for a race's own ability.
    fn own_redeploy_race(&self, seat: usize, region: usize, race: Race) -> Result<Army, Error> {
        if self.phase != Phase::Redeploy {
            return Err(Error::WrongPhase);
        }
        let army = self.players[seat].active.filter(|a| a.race == race).ok_or(Error::NotYourPower)?;
        if self.regions[region].race() == Some(race) {
            Ok(army)
        } else {
            Err(Error::NotYours)
        }
    }

    fn enchant_target(&self, seat: usize, army: &Army, region: usize) -> Result<(), Error> {
        if army.race != Race::Sorcerers {
            return Err(Error::NotYourPower);
        }
        let r = self.regions.get(region).ok_or(Error::NoSuchRegion)?;
        let t = r.tokens.ok_or(Error::NotAlone)?;
        let (victim, declined) = self.owner(t.race).ok_or(Error::NotAlone)?;
        if victim == seat {
            return Err(Error::AlreadyYours);
        }
        if declined || t.count != 1 || r.encampments > 0 || r.behemoths > 0 || r.captive.is_some() {
            return Err(Error::NotAlone);
        }
        if self.this_turn.enchanted.contains(&victim) {
            return Err(Error::PowerUsed);
        }
        self.target(seat, army, false, region)?;
        if !self.next_to(army, false, region) {
            return Err(Error::OutOfReach);
        }
        if army.hand == 0 {
            return Err(Error::NoTokens);
        }
        if self.available(Race::Sorcerers) == 0 {
            return Err(Error::NoneLeft);
        }
        Ok(())
    }

    // ----- taking regions --------------------------------------------------------------

    /// A Skag Attack waiting in `region` for whoever takes it from the
    /// Skags: the conquest fails and the attacker loses a token. Returns
    /// whether it struck.
    fn skag_attack(&mut self, seat: usize, race: Race, region: usize) -> bool {
        let skags = self.regions[region].race() == Some(Race::Skags);
        if !skags || self.loot[region] != Some(Loot::SkagAttack) {
            return false;
        }
        self.loot[region] = None;
        self.regions[region].loot = false;
        if let Some(a) = self.army_mut(seat, race) {
            a.hand = a.hand.saturating_sub(1);
        }
        self.this_turn.repelled.push(region);
        self.mark_moved(self.phase == Phase::Conquest { ghouls: true });
        self.push(Event::SkagAttack { seat, region });
        true
    }

    /// Coins changing hands: from a seat (as many as it has, at most
    /// `coins`), or from the bank.
    fn pay(&mut self, from: Option<usize>, to: usize, coins: u32, why: Why) {
        let coins = match from {
            Some(f) => coins.min(self.players[f].coins),
            None => coins,
        };
        if coins == 0 {
            return;
        }
        if let Some(f) = from {
            self.players[f].coins -= coins;
        }
        self.players[to].coins += coins;
        self.push(Event::Paid { from, to, coins, why });
    }

    /// A token lost for good in a conquest: the Igors collect it.
    fn lost_in_conquest(&mut self, count: u8) {
        if count == 0 {
            return;
        }
        if let Some((seat, _)) = self.active_race(Race::Igors) {
            let a = self.players[seat].active.as_mut().unwrap();
            a.collected = a.collected.saturating_add(count);
        }
    }

    /// Whoever was in `region` makes way for `seat`'s `race`: a Lost Tribe
    /// is gone, a race loses a token (Elves none) and keeps the rest in hand,
    /// with what the races and powers on either side do about it. Returns
    /// who it was, and the race in decline the Scavengers keep there.
    fn clear_defenders(&mut self, seat: usize, race: Race, declined: bool, region: usize) -> (Option<Foe>, Option<Race>) {
        let r = self.regions[region];
        let mut foe = None;
        let mut captive = None;
        let scavenging = !declined && race == Race::Scavengers;
        if r.lost_tribe {
            foe = Some(Foe::LostTribe);
            self.lost_in_conquest(1);
        }
        if r.captive.is_some() {
            // Another race takes the region: the captive is discarded.
            self.lost_in_conquest(1);
        }
        if let Some(t) = r.tokens {
            let (owner, owner_declined) = self.owner(t.race).expect("tokens on the map have an owner");
            let elves = t.race == Race::Elves && !owner_declined;
            let mut lost = if elves { 0 } else { 1 };
            if scavenging && owner_declined && lost == 1 {
                captive = Some(t.race);
                lost = 0;
            }
            let kept = t.count - lost - captive.is_some() as u8;
            let behemoths = r.behemoths;
            if let Some(a) = self.army_mut(owner, t.race) {
                a.hand += kept;
                a.behemoths += behemoths;
            }
            self.lost_in_conquest(lost);
            if lost > 0 && t.race == Race::Pygmies && !owner_declined {
                let die = self.roll();
                let raised = die.min(self.available(Race::Pygmies));
                if raised > 0 {
                    self.army_mut(owner, t.race).unwrap().hand += raised;
                    self.push(Event::Raised { seat: owner, race: t.race, tokens: raised });
                }
            }
            if owner != seat {
                if !owner_declined && !self.this_turn.attacked.contains(&owner) {
                    self.this_turn.attacked.push(owner);
                }
                if !self.this_turn.victims.contains(&t.race) {
                    self.this_turn.victims.push(t.race);
                }
                if !declined {
                    let me = self.players[seat].active.unwrap();
                    if !owner_declined {
                        if self.active_has(owner, Power::Corrupt) {
                            self.pay(Some(seat), owner, 1, Why::Corrupt);
                        }
                        if me.has(Power::Ransacking) {
                            self.pay(Some(owner), seat, 1, Why::Ransacking);
                        }
                        if t.race == Race::Scarecrows {
                            self.pay(None, seat, 1, Why::Scarecrows);
                        }
                        if me.race == Race::Fauns {
                            self.this_turn.fauns.push(owner);
                        }
                    }
                }
            }
            foe = Some(Foe::Race { seat: owner, race: t.race });
        }
        if r.pot {
            if let Some(t) = r.tokens {
                if let Some((owner, _)) = self.owner(t.race) {
                    if let Some(a) = self.army_mut(owner, t.race) {
                        a.pots += 1;
                    }
                }
            }
            self.pay(None, seat, 1, Why::Pot);
        }
        if r.loot {
            if let Some(Loot::Coins(n)) = self.loot[region].take() {
                self.pay(None, seat, n as u32, Why::Loot);
            }
        }
        (foe, captive)
    }

    /// Takes `region` with `tokens` of the conquering race (already counted
    /// against its hand by the caller's check), clearing the defenders out.
    fn take_region(&mut self, seat: usize, army: Army, declined: bool, region: usize, tokens: u8, how: How) {
        let before = self.regions[region];
        let (foe, captive) = self.clear_defenders(seat, army.race, declined, region);
        if foe.is_some() && !declined {
            self.this_turn.plundered += 1;
        }
        let me = *self.army(seat, army.race).expect("the conquering race");
        let mut placed = RegionState {
            tokens: Some(Tokens { race: army.race, count: tokens, declined }),
            winter: before.winter,
            captive,
            dragon: how == How::Dragon,
            behemoths: (how == How::Behemoth) as u8,
            ..RegionState::default()
        };
        if !declined {
            if army.race == Race::Trolls {
                placed.lair = true;
            }
            if army.race == Race::Halflings && me.holes < HOLES {
                placed.hole = true;
            }
            if army.race == Race::Skags {
                if let Some(loot) = self.loot_bag.pop() {
                    self.loot[region] = Some(loot);
                    placed.loot = true;
                }
            }
        }
        if !placed.loot {
            // Loot left behind in a region the Skags abandoned goes with it.
            self.loot[region] = None;
        }
        self.regions[region] = placed;
        let me = self.army_mut(seat, army.race).expect("the conquering race");
        me.hand -= tokens;
        if placed.hole {
            me.holes += 1;
        }
        if how == How::Behemoth {
            me.behemoths -= 1;
        }
        self.this_turn.conquered.push(region);
        self.mark_moved(declined);
        self.push(Event::Conquered { seat, race: army.race, region, tokens, foe, how });
        if how == How::Sling {
            self.pay(None, seat, 1, Why::Slingmen);
        }
    }

    fn mark_moved(&mut self, declined: bool) {
        self.this_turn.moved = true;
        if declined {
            self.this_turn.ghouls_moved = true;
        }
    }

    /// Empties `region` of `seat`'s tokens, back into their hand.
    fn abandon(&mut self, seat: usize, race: Race, region: usize) {
        let r = self.regions[region];
        let t = r.tokens.expect("an occupied region");
        if let Some(a) = self.army_mut(seat, race) {
            a.hand += t.count;
            a.behemoths += r.behemoths;
        }
        // The Loot stays behind, and the Winter.
        self.regions[region] = RegionState { loot: r.loot, winter: r.winter, ..RegionState::default() };
    }

    // ----- turn flow ---------------------------------------------------------------------

    /// The current player's turn begins: what lasted until now ends, then
    /// pick a race or ready the troops.
    fn begin_turn(&mut self) {
        let seat = self.current;
        self.this_turn = Turn::default();
        for r in self.regions.iter_mut() {
            if r.lava == Some(seat) {
                r.lava = None;
            }
            if r.burning == Some(seat) {
                r.burning = None;
            }
        }
        // Pots of Gold still standing pay out.
        let leprechauns: Vec<Race> = {
            let p = &self.players[seat];
            p.active.iter().chain(&p.declined).map(|a| a.race).filter(|&r| r == Race::Leprechauns).collect()
        };
        for race in leprechauns {
            for region in self.regions_of(race) {
                if self.regions[region].pot {
                    self.regions[region].pot = false;
                    self.army_mut(seat, race).unwrap().pots += 1;
                    self.pay(None, seat, 1, Why::Pot);
                }
            }
        }
        if let Some(a) = self.players[seat].active {
            // Peace made by this player ends now, and so does what was copied.
            self.players[seat].active.as_mut().unwrap().ally = None;
            if a.copied.is_some() {
                self.uncopy(seat);
            }
            if a.race == Race::Escargots {
                let coins = self.regions_of(a.race).len() as u32;
                self.pay(None, seat, coins, Why::Escargots);
            }
            if a.race == Race::Drakons {
                for r in self.regions_of(a.race) {
                    self.regions[r].dragon = false;
                }
            }
            for r in self.regions_of(a.race) {
                if self.regions[r].cannon.is_some() {
                    self.regions[r].cannon = Some(false);
                }
            }
        }
        if self.players[seat].active.is_none() {
            self.phase = Phase::Pick;
            return;
        }
        self.this_turn.may_decline = true;
        let race = self.players[seat].active.unwrap().race;
        self.this_turn.lifted = self.ready(seat, race);
        if race == Race::Igors {
            let a = self.players[seat].active.unwrap();
            let new = (a.collected as usize / self.seats()) as u8;
            let raised = new.min(self.available(race));
            let seats = self.seats() as u8;
            let a = self.players[seat].active.as_mut().unwrap();
            a.collected %= seats;
            if raised > 0 {
                a.hand += raised;
                self.push(Event::Raised { seat, race, tokens: raised });
            }
        }
        self.begin_conquests();
    }

    /// Leaves the fewest tokens allowed in each region of `race` and takes
    /// the others in hand (with those that sat out, and the Behemoths).
    /// Returns what was lifted where.
    fn ready(&mut self, seat: usize, race: Race) -> Vec<(usize, u8)> {
        let declined = self.owner(race).is_some_and(|(_, d)| d);
        let keep = min_tokens(race, declined);
        let mut lifted = Vec::new();
        let mut taken = 0;
        let mut behemoths = 0;
        for (i, r) in self.regions.iter_mut().enumerate() {
            if r.race() != Some(race) {
                continue;
            }
            behemoths += std::mem::take(&mut r.behemoths);
            let t = r.tokens.as_mut().unwrap();
            if t.count > keep {
                lifted.push((i, t.count - keep));
                taken += t.count - keep;
                t.count = keep;
            }
        }
        let army = self.army_mut(seat, race).expect("a race to ready");
        army.hand += taken + army.resting;
        army.resting = 0;
        army.behemoths += behemoths;
        lifted
    }

    /// Puts the tokens `ready` lifted back where they were, as far as the
    /// hand still holds them.
    fn unready(&mut self, seat: usize, race: Race, lifted: &[(usize, u8)]) {
        let mut hand = self.army(seat, race).map_or(0, |a| a.hand);
        for &(region, n) in lifted {
            if let Some(t) = self.regions[region].tokens.as_mut().filter(|t| t.race == race) {
                let back = n.min(hand);
                t.count += back;
                hand -= back;
            }
        }
        if let Some(a) = self.army_mut(seat, race) {
            a.hand = hand;
        }
    }

    /// Conquests start: with the Ghouls in decline, if any, else the active race.
    fn begin_conquests(&mut self) {
        let seat = self.current;
        if self.decline_ghouls(seat).is_some() {
            self.this_turn.ghouls_lifted = self.ready(seat, Race::Ghouls);
            self.phase = Phase::Conquest { ghouls: true };
        } else {
            self.phase = Phase::Conquest { ghouls: false };
        }
    }

    /// The Ghouls in decline put back what they lifted.
    fn unready_ghouls(&mut self, seat: usize) {
        let lifted = std::mem::take(&mut self.this_turn.ghouls_lifted);
        self.unready(seat, Race::Ghouls, &lifted);
    }

    /// Done conquering with the Ghouls in decline, or with the active race.
    fn end_conquests(&mut self) {
        let seat = self.current;
        self.this_turn.berserk = None;
        if self.phase == (Phase::Conquest { ghouls: true }) {
            if !self.this_turn.ghouls_moved {
                self.unready_ghouls(seat);
            }
            self.phase = Phase::Conquest { ghouls: false };
            return;
        }
        if let Some(a) = self.players[seat].active {
            // Marauding: take the troops back in hand and conquer again.
            if a.has(Power::Marauding) && !self.this_turn.marauded {
                self.this_turn.marauded = true;
                self.ready(seat, a.race);
                return;
            }
            let gain = match a.race {
                Race::Skeletons => self.this_turn.plundered / 2,
                Race::Fauns => self.this_turn.fauns.len() as u8,
                _ => 0,
            };
            let gain = gain.min(self.available(a.race));
            if gain > 0 {
                self.players[seat].active.as_mut().unwrap().hand += gain;
                self.push(Event::Raised { seat, race: a.race, tokens: gain });
            }
            // The Fauns' victims get a token for each region they lost.
            for victim in std::mem::take(&mut self.this_turn.fauns) {
                if let Some(v) = self.players[victim].active {
                    if self.available(v.race) > 0 {
                        self.players[victim].active.as_mut().unwrap().hand += 1;
                    }
                }
            }
            if a.race == Race::IceWitches {
                let magic = self.regions_of(a.race).iter().filter(|&&r| self.board.regions[r].magic()).count() as u8;
                self.players[seat].active.as_mut().unwrap().winter += magic;
            }
        }
        self.phase = Phase::Redeploy;
    }

    /// The tokens Amazons must keep in hand at the end of the turn: four,
    /// unless that would leave one of their regions empty.
    fn amazons_resting(&self, army: &Army) -> u8 {
        if army.race != Race::Amazons {
            return 0;
        }
        let regions = self.regions_of(army.race).len() as u8;
        AMAZON_EXTRA.min((self.on_map(army.race) + army.hand).saturating_sub(regions))
    }

    /// Why the turn can't end yet, if it can't.
    fn check_end_turn(&self, seat: usize) -> Result<(), Error> {
        let p = &self.players[seat];
        if let Some(a) = p.active {
            let held = self.regions_of(a.race);
            if !held.is_empty() {
                let rest = self.amazons_resting(&a);
                let sits_out = matches!(a.race, Race::Barbarians | Race::Pixies);
                if !sits_out && a.hand > rest {
                    return Err(if rest > 0 { Error::RestAmazons } else { Error::TokensInHand });
                }
                if a.hand < rest {
                    return Err(Error::RestAmazons);
                }
                if a.has(Power::Heroic) && self.heroes(seat) < HEROES.min(held.len()) {
                    return Err(Error::PlaceHeroes);
                }
                if a.has(Power::Gunner) && self.cannons(a.race) < CANNONS.min(held.len()) {
                    return Err(Error::PlaceCannons);
                }
            }
        }
        if let Some(g) = self.decline_ghouls(seat) {
            if g.hand > 0 && !self.regions_of(g.race).is_empty() {
                return Err(Error::TokensInHand);
            }
        }
        Ok(())
    }

    /// The end of a turn of conquests: tokens sit out, Fireballs are made,
    /// coins are scored, and a Stout race may go into decline.
    fn finish_turn(&mut self, stout_decline: bool) {
        let seat = self.current;
        if let Some(a) = self.players[seat].active {
            if a.race == Race::Pixies {
                // One Pixie stays per region; the others sit out.
                for r in self.regions_of(a.race) {
                    let t = self.regions[r].tokens.as_mut().unwrap();
                    let extra = t.count - 1;
                    t.count = 1;
                    self.players[seat].active.as_mut().unwrap().hand += extra;
                }
            }
            let a = self.players[seat].active.as_mut().unwrap();
            if matches!(a.race, Race::Amazons | Race::Barbarians | Race::Pixies) {
                a.resting += a.hand;
                a.hand = 0;
            }
            if a.has(Power::Fireball) {
                let race = a.race;
                let magic = self.regions_of(race).iter().filter(|&&r| self.board.regions[r].magic()).count() as u8;
                let a = self.players[seat].active.as_mut().unwrap();
                a.fireballs = (a.fireballs + magic).min(FIREBALLS);
            }
        }
        self.score(seat);
        if let Some(a) = self.players[seat].active.as_mut() {
            a.turns = a.turns.saturating_add(1);
        }
        if stout_decline {
            self.decline(seat, None);
        }
        self.after_turn();
    }

    /// Coins for the turn.
    fn score(&mut self, seat: usize) {
        let coins = self.turn_coins(seat);
        self.players[seat].coins += coins;
        self.push(Event::Scored { seat, coins });
    }

    /// What `seat` would score if their turn ended now: each region held by
    /// any of their races (in decline too) is worth a coin, more or less
    /// what the races and powers add or take off, never below nothing; then
    /// the bonuses of the active race and power, and the islands held whole.
    pub fn turn_coins(&self, seat: usize) -> u32 {
        let board = &self.board;
        let p = &self.players[seat];
        let mut coins: u32 = 0;
        let mut armies: Vec<(Army, bool)> = p.declined.iter().map(|a| (*a, true)).collect();
        if let Some(a) = p.active {
            armies.push((a, false));
        }
        for &(army, declined) in &armies {
            let held = self.regions_of(army.race);
            if declined && army.race == Race::Priestesses {
                // The Ivory Tower: a coin per Priestess in it.
                coins += held.iter().map(|&r| self.regions[r].tokens.map_or(0, |t| t.count) as u32).sum::<u32>();
                continue;
            }
            // Regions where the Scavengers keep this race's token score too.
            let captives = (0..self.regions.len()).filter(|&r| self.regions[r].captive == Some(army.race)).count();
            coins += captives as u32;
            for &r in &held {
                let spot = &board.regions[r];
                let state = &self.regions[r];
                let mut plus: i32 = 0;
                let mut minus: i32 = 0;
                if army.race == Race::Dwarves && spot.mine() {
                    plus += 1;
                }
                if state.winter && army.race != Race::IceWitches {
                    minus += 1;
                }
                if !declined {
                    let has = |p: Power| army.has(p);
                    plus += match army.race {
                        Race::Humans => (spot.terrain == Terrain::Farmland) as i32,
                        Race::Wizards => spot.magic() as i32,
                        Race::Khans if matches!(spot.terrain, Terrain::Hill | Terrain::Farmland) => 1,
                        _ => 0,
                    };
                    if army.race == Race::Khans && !matches!(spot.terrain, Terrain::Hill | Terrain::Farmland) {
                        minus += 1;
                    }
                    plus += has(Power::Forest) as i32 * (spot.terrain == Terrain::Forest) as i32;
                    plus += has(Power::Hill) as i32 * (spot.terrain == Terrain::Hill) as i32;
                    plus += has(Power::Swamp) as i32 * (spot.terrain == Terrain::Swamp) as i32;
                    plus += has(Power::Merchant) as i32;
                    plus += has(Power::Fortified) as i32 * state.fortress as i32;
                    if has(Power::Aquatic) {
                        if board.coastal(r) {
                            plus += 1;
                        } else {
                            minus += 1;
                        }
                    }
                    if has(Power::Goldsmith) {
                        if spot.mine() {
                            plus += 2;
                        } else {
                            minus += 1;
                        }
                    }
                }
                if !declined && army.race == Race::Escargots {
                    // Their regions paid at the start of the turn; only what
                    // the power adds comes now.
                    coins += plus.max(0) as u32;
                } else {
                    coins += (1 + plus - minus).max(0) as u32;
                }
            }
            // An island held whole by one race earns it a coin.
            for island in &board.islands {
                let land: Vec<usize> = island.iter().copied().filter(|&r| !board.regions[r].terrain.is_water()).collect();
                if !land.is_empty() && land.iter().all(|&r| self.regions[r].race() == Some(army.race)) {
                    coins += 1;
                }
            }
        }
        if let Some(a) = p.active {
            let held = self.regions_of(a.race);
            let plundered = self.this_turn.plundered as u32;
            let mine = self.current == seat;
            coins += match a.race {
                Race::Orcs if mine => plundered,
                Race::Gypsies if mine => self.this_turn.abandoned.len() as u32,
                _ => 0,
            };
            if a.has(Power::Pillaging) && mine {
                coins += plundered;
            }
            if a.has(Power::Alchemist) {
                coins += 2;
            }
            if a.power == Power::Wealthy && a.turns == 0 {
                coins += WEALTHY_COINS;
            }
            if a.has(Power::Barricade) && held.len() <= BARRICADE_REGIONS {
                coins += BARRICADE_COINS;
            }
            if a.has(Power::Imperial) {
                coins += held.len().saturating_sub(3) as u32;
            }
            if a.has(Power::PeaceLoving) && mine && self.this_turn.attacked.is_empty() {
                coins += PEACE_COINS;
            }
            if a.has(Power::Exploring) {
                let sky = held.iter().filter(|&&r| board.regions[r].sky).count();
                coins += sky.min(held.len() - sky) as u32;
            }
        }
        coins
    }

    /// The active race goes into decline: one token stays in each region
    /// (Ghouls keep them all; Priestesses stack theirs in their Ivory Tower,
    /// in `tower` or the best defended region), the power is lost (Spirit's
    /// is kept), and the older race in decline leaves the map (unless one of
    /// the two is Spirit's, or Soul-Touch brings it back instead).
    fn decline(&mut self, seat: usize, tower: Option<usize>) {
        let Some(active) = self.players[seat].active else { return };
        // Ghouls keep every token: what was lifted to conquer goes back.
        if active.race == Race::Ghouls {
            let lifted = std::mem::take(&mut self.this_turn.lifted);
            self.unready(seat, active.race, &lifted);
        }
        if active.copied.is_some() {
            self.uncopy(seat);
        }
        let mut army = self.players[seat].active.take().unwrap();
        // The Skags' loot is theirs now.
        if army.race == Race::Skags {
            self.collect_loot(seat);
        }
        for r in self.regions.iter_mut() {
            if r.race() == Some(army.race) {
                let t = r.tokens.as_mut().unwrap();
                if army.race != Race::Ghouls {
                    t.count = 1;
                }
                t.declined = true;
                r.hole = false;
                r.hero = false;
                r.dragon = false;
                r.encampments = 0;
                r.catapult = false;
                r.behemoths = 0;
                r.cannon = None;
                if army.race == Race::Scavengers {
                    r.captive = None;
                }
            }
            if army.race == Race::IceWitches {
                r.winter = false;
            }
        }
        if army.race == Race::Priestesses {
            let held = self.regions_of(army.race);
            let best = |&&r: &&usize| (self.regions[r].fortress, self.board.regions[r].mountain(), std::cmp::Reverse(r));
            let tower = tower.filter(|t| held.contains(t)).or_else(|| held.iter().max_by_key(best).copied());
            if let Some(tower) = tower {
                for &r in &held {
                    if r != tower {
                        self.regions[r] = RegionState { winter: self.regions[r].winter, ..RegionState::default() };
                    }
                }
                self.regions[tower].tokens.as_mut().unwrap().count = held.len() as u8;
            }
        }
        army.hand = 0;
        army.resting = 0;
        army.ally = None;
        army.extra = 0;
        army.fireballs = 0;
        army.winter = 0;
        army.behemoths = 0;
        self.push(Event::Declined { seat, race: army.race });
        // The Historians pay.
        for s in 0..self.seats() {
            if self.active_has(s, Power::Historian) {
                self.pay(None, s, 1, Why::Historian);
            }
        }
        if army.has(Power::Historian) {
            self.pay(None, seat, 1, Why::Historian);
        }
        if army.has(Power::Racketeering) {
            self.players[seat].free_pick = true;
        }
        let soul_touch = army.power == Power::SoulTouch;
        if army.power != Power::Spirit {
            self.discarded_powers.push(army.power);
            if let Some(old) = self.players[seat].declined.iter().position(|a| a.power != Power::Spirit) {
                let old = self.players[seat].declined.remove(old);
                if soul_touch {
                    self.revive(seat, old);
                } else {
                    self.vanish(seat, old);
                }
            } else if soul_touch && !self.players[seat].declined.is_empty() {
                let old = self.players[seat].declined.remove(0);
                self.revive(seat, old);
            }
        }
        self.players[seat].declined.push(army);
    }

    /// Soul-Touch: a race in decline comes back to life, with its power and
    /// as many tokens as when it was picked.
    fn revive(&mut self, seat: usize, mut old: Army) {
        self.take_badge(old.power);
        for r in self.regions.iter_mut() {
            if let Some(t) = r.tokens.as_mut().filter(|t| t.race == old.race) {
                t.declined = false;
            }
        }
        let full = old.race.tokens() + old.power.tokens();
        old.hand = full.saturating_sub(self.on_map(old.race)).min(self.available(old.race));
        old.copied = None;
        self.players[seat].active = Some(old);
        self.push(Event::Revived { seat, race: old.race });
    }

    /// Takes a power's badge back from wherever it went (for Soul-Touch).
    fn take_badge(&mut self, power: Power) {
        if let Some(i) = self.discarded_powers.iter().position(|&p| p == power) {
            self.discarded_powers.remove(i);
        } else if let Some(i) = self.powers.iter().position(|&p| p == power) {
            self.powers.remove(i);
        } else if let Some(i) = self.offers.iter().position(|o| o.power == power) {
            // On a combo in the column: that combo gets another badge.
            let next = self.powers.pop().or_else(|| self.discarded_powers.pop());
            match next {
                Some(p) => self.offers[i].power = p,
                None => {
                    let o = self.offers.remove(i);
                    self.races.push(o.race);
                }
            }
        }
    }

    /// The Skags turn over the Loot in their regions and keep the coins.
    fn collect_loot(&mut self, seat: usize) {
        let mut coins = 0;
        for r in self.regions_of(Race::Skags) {
            if let Some(Loot::Coins(n)) = self.loot[r].take() {
                coins += n as u32;
            }
            self.loot[r] = None;
            self.regions[r].loot = false;
        }
        self.pay(None, seat, coins, Why::Loot);
    }

    /// A race in decline leaves the map: its tokens go back in the box, its
    /// banner to the bottom of the stack.
    fn vanish(&mut self, seat: usize, army: Army) {
        for (i, r) in self.regions.iter_mut().enumerate() {
            if r.race() == Some(army.race) {
                *r = RegionState { winter: r.winter, loot: r.loot, ..RegionState::default() };
            }
            if r.captive == Some(army.race) {
                r.captive = None;
            }
            if army.race == Race::Skags {
                r.loot = false;
                self.loot[i] = None;
            }
        }
        if army.power == Power::Spirit {
            self.discarded_powers.push(army.power);
        }
        self.races.push(army.race);
        self.push(Event::Vanished { seat, race: army.race });
        self.refill();
    }

    /// Copycat's copy ends: the pieces of the copied power go.
    fn uncopy(&mut self, seat: usize) {
        let Some(a) = self.players[seat].active else { return };
        let Some(power) = a.copied else { return };
        for r in self.regions_of(a.race) {
            let s = &mut self.regions[r];
            match power {
                Power::Bivouacking => s.encampments = 0,
                Power::Fortified => s.fortress = false,
                Power::Heroic => s.hero = false,
                Power::DragonMaster => s.dragon = false,
                Power::Catapult => s.catapult = false,
                Power::Behemoth => s.behemoths = 0,
                Power::Gunner => s.cannon = None,
                _ => {}
            }
        }
        if power == Power::Seafaring {
            // Off the water: the tokens there come back in hand.
            for r in self.regions_of(a.race) {
                if self.board.regions[r].terrain.is_water() {
                    self.abandon(seat, a.race, r);
                }
            }
        }
        let a = self.players[seat].active.as_mut().unwrap();
        match power {
            Power::Behemoth => a.behemoths = 0,
            Power::Fireball => a.fireballs = 0,
            _ => {}
        }
        a.copied = None;
    }

    /// The end of a turn, however it ended: Trade Pacts are settled.
    fn close_turn(&mut self) {
        let seat = self.current;
        let pacts = std::mem::take(&mut self.pacts[seat]);
        if pacts == 0 {
            return;
        }
        let haggler = (0..self.seats()).find(|&s| s != seat && self.active_has(s, Power::Haggling));
        if let Some(h) = haggler {
            let race = self.players[h].active.unwrap().race;
            if self.this_turn.victims.contains(&race) {
                self.pay(Some(seat), h, pacts as u32, Why::Pact);
            } else {
                self.pay(Some(h), seat, pacts as u32, Why::Pact);
            }
        }
    }

    /// After a turn: whoever lost regions puts their tokens back, then the
    /// next player's turn.
    fn after_turn(&mut self) {
        self.close_turn();
        let n = self.seats();
        self.reinforce = (1..=n).map(|i| (self.current + i) % n).filter(|&s| self.must_reinforce(s)).collect();
        if self.reinforce.is_empty() {
            self.next_turn();
        } else {
            self.phase = Phase::Reinforce;
        }
    }

    /// Whether `seat` has tokens in hand and somewhere to put them. Tokens
    /// with nowhere to go stay in hand for an active race (it comes back as
    /// if it were new) and are lost for a race in decline.
    fn must_reinforce(&self, seat: usize) -> bool {
        let p = &self.players[seat];
        p.active.iter().chain(&p.declined).any(|a| a.hand > 0 && !self.regions_of(a.race).is_empty())
    }

    /// Clears out races in decline with nothing left, and their stray
    /// tokens; the next turn, or the end.
    fn next_turn(&mut self) {
        for seat in 0..self.seats() {
            let mut i = 0;
            while i < self.players[seat].declined.len() {
                let a = self.players[seat].declined[i];
                let captives = self.regions.iter().any(|r| r.captive == Some(a.race));
                if self.regions_of(a.race).is_empty() && !captives {
                    let a = self.players[seat].declined.remove(i);
                    self.vanish(seat, a);
                } else {
                    self.players[seat].declined[i].hand = 0;
                    i += 1;
                }
            }
        }
        self.phase = Phase::Pick;
        self.current = (self.current + 1) % self.seats();
        if self.current == 0 {
            if self.turn >= self.turns() {
                // The Skags that never declined turn their loot over now.
                if let Some((seat, _)) = self.active_race(Race::Skags) {
                    self.collect_loot(seat);
                }
                self.phase = Phase::Over;
                self.push(Event::GameOver);
                return;
            }
            self.turn += 1;
            self.push(Event::TurnStarted(self.turn));
        }
        self.begin_turn();
    }

    /// Fills the column back up to six from the stacks (the discarded
    /// powers are shuffled into a new stack when it runs out).
    fn refill(&mut self) {
        while self.offers.len() < OFFERS && !self.races.is_empty() {
            if self.powers.is_empty() {
                if self.discarded_powers.is_empty() {
                    break;
                }
                self.powers = std::mem::take(&mut self.discarded_powers);
                shuffle(&mut self.powers, &mut self.rng);
            }
            let race = self.races.remove(0);
            let power = self.powers.remove(0);
            self.offers.push(Offer { race, power, coins: 0, tokens: 0 });
        }
    }

    fn roll(&mut self) -> u8 {
        DIE[(next_random(&mut self.rng) % DIE.len() as u64) as usize]
    }

    fn push(&mut self, event: Event) {
        self.log.push(event);
        if self.log.len() > LOG_LEN {
            self.log.remove(0);
        }
    }

    // ----- scores ----------------------------------------------------------------------------

    /// Tokens a seat has on the map, all races together (the tie-break).
    pub fn tokens_on_map(&self, seat: usize) -> u32 {
        let p = &self.players[seat];
        p.active.iter().chain(&p.declined).map(|a| self.on_map(a.race) as u32).sum()
    }

    /// Once the game is over: the most coins, then the most tokens on the
    /// map. Whoever is still level shares the win.
    pub fn winners(&self) -> Vec<usize> {
        if self.phase != Phase::Over {
            return Vec::new();
        }
        let key = |seat: usize| (self.players[seat].coins, self.tokens_on_map(seat));
        let best = (0..self.seats()).map(key).max().unwrap_or_default();
        (0..self.seats()).filter(|&s| key(s) == best).collect()
    }

    /// What the combo at `offer` costs `seat`: a coin for each combo above
    /// (3 for a Cursed one), nothing after a Racketeering decline.
    pub fn pick_cost(&self, seat: usize, offer: usize) -> u32 {
        if self.players[seat].free_pick {
            return 0;
        }
        self.offers[..offer.min(self.offers.len())]
            .iter()
            .map(|o| if o.power == Power::Cursed { CURSED_SKIP } else { 1 })
            .sum()
    }

    // ----- moves -------------------------------------------------------------------------------

    /// Applies one move by `seat`, who must be the one to act.
    pub fn apply(&mut self, seat: usize, action: Action) -> Result<(), Error> {
        if self.phase == Phase::Over {
            return Err(Error::GameOver);
        }
        if seat >= self.seats() {
            return Err(Error::UnknownSeat);
        }
        if self.actor() != Some(seat) {
            return Err(Error::NotYourTurn);
        }
        match (self.phase, action) {
            (Phase::Pick, Action::Pick { offer }) => self.pick(seat, offer),
            (Phase::Conquest { .. }, Action::Decline) => {
                if !self.this_turn.may_decline || self.this_turn.moved {
                    return Err(Error::TooLateToDecline);
                }
                self.decline_now(seat, None);
                Ok(())
            }
            (_, Action::Tower { region }) => {
                self.check(seat, RegionMove::Tower, region)?;
                self.decline_now(seat, Some(region));
                Ok(())
            }
            (Phase::Conquest { .. }, Action::Conquer { region }) => {
                let cost = self.check(seat, RegionMove::Conquer, region)?;
                let (army, declined) = self.conqueror(seat).unwrap();
                let sling = self.reach(&army, declined, region)?;
                self.this_turn.berserk = None;
                if !self.skag_attack(seat, army.race, region) {
                    let how = if sling { How::Sling } else { How::Tokens };
                    self.take_region(seat, army, declined, region, cost, how);
                }
                Ok(())
            }
            (Phase::Conquest { .. }, Action::Gamble { region }) => {
                let cost = self.check(seat, RegionMove::Gamble, region)?;
                let (army, declined) = self.conqueror(seat).unwrap();
                let sling = self.reach(&army, declined, region)?;
                let die = self.roll();
                let mut won = army.hand + die >= cost;
                // Mercenaries can be hired once the die has spoken.
                if !won && !declined && army.has(Power::Mercenary) && self.players[seat].coins > 0 {
                    let hired = cost.saturating_sub(2).max(min_tokens(army.race, false));
                    if army.hand + die >= hired {
                        self.players[seat].coins -= 1;
                        won = true;
                    }
                }
                self.push(Event::Gambled { seat, region, die, won });
                if won && !self.skag_attack(seat, army.race, region) {
                    let tokens = self.army(seat, army.race).unwrap().hand;
                    let how = if sling { How::Sling } else { How::Tokens };
                    self.take_region(seat, army, declined, region, tokens, how);
                }
                self.end_conquests();
                Ok(())
            }
            (Phase::Conquest { ghouls: false }, Action::Roll) => {
                let army = self.players[seat].active.ok_or(Error::NoRace)?;
                if !army.has(Power::Berserk) {
                    return Err(Error::NotYourPower);
                }
                if self.this_turn.berserk.is_some() {
                    return Err(Error::PowerUsed);
                }
                if army.hand == 0 {
                    return Err(Error::NoTokens);
                }
                let die = self.roll();
                self.this_turn.berserk = Some(die);
                self.push(Event::Rolled { seat, die });
                Ok(())
            }
            (Phase::Conquest { ghouls: false }, Action::Hire { region }) => {
                let tokens = self.check(seat, RegionMove::Hire, region)?;
                let army = self.players[seat].active.unwrap();
                self.this_turn.berserk = None;
                self.players[seat].coins -= 1;
                if !self.skag_attack(seat, army.race, region) {
                    self.take_region(seat, army, false, region, tokens, How::Hire);
                }
                Ok(())
            }
            (Phase::Conquest { ghouls: false }, Action::Fireball { region }) => {
                let tokens = self.check(seat, RegionMove::Fireball, region)?;
                let army = self.players[seat].active.unwrap();
                let cost = self.die_cost(&army, false, self.cost(seat, &army, false, region));
                let (_, used) = self.fireballs(&army, cost);
                self.this_turn.berserk = None;
                self.players[seat].active.as_mut().unwrap().fireballs -= used;
                if !self.skag_attack(seat, army.race, region) {
                    self.take_region(seat, army, false, region, tokens, How::Fireball);
                }
                Ok(())
            }
            (Phase::Conquest { ghouls: false }, Action::Behemoth { region }) => {
                let tokens = self.check(seat, RegionMove::Behemoth, region)?;
                let army = self.players[seat].active.unwrap();
                self.this_turn.berserk = None;
                if !self.skag_attack(seat, army.race, region) {
                    self.take_region(seat, army, false, region, tokens, How::Behemoth);
                }
                Ok(())
            }
            (Phase::Conquest { ghouls: false }, Action::Catapult { region }) => {
                let tokens = self.check(seat, RegionMove::Catapult, region)?;
                let army = self.players[seat].active.unwrap();
                let from = self.launcher(&army, region).unwrap();
                self.this_turn.berserk = None;
                self.this_turn.catapult = true;
                for r in self.regions_of(army.race) {
                    self.regions[r].catapult = false;
                }
                self.regions[from].catapult = true;
                if !self.skag_attack(seat, army.race, region) {
                    self.take_region(seat, army, false, region, tokens, How::Catapult);
                }
                Ok(())
            }
            (Phase::Conquest { ghouls: false }, Action::Dragon { region }) => {
                let tokens = self.check(seat, RegionMove::Dragon, region)?;
                let army = self.players[seat].active.unwrap();
                let master = army.has(Power::DragonMaster) && !self.this_turn.dragon;
                self.this_turn.berserk = None;
                if self.skag_attack(seat, army.race, region) {
                    return Ok(());
                }
                if master {
                    // Dragon Master's Dragon moves.
                    for r in self.regions.iter_mut() {
                        if r.race() == Some(army.race) {
                            r.dragon = false;
                        }
                    }
                    self.this_turn.dragon = true;
                } else {
                    // A Drakon turns into the Dragon: back in the box.
                    self.this_turn.drakons += 1;
                    self.players[seat].active.as_mut().unwrap().hand -= 1;
                }
                let placed = if master { tokens } else { tokens - 1 };
                self.take_region(seat, army, false, region, placed, How::Dragon);
                Ok(())
            }
            (Phase::Conquest { ghouls: false }, Action::Lightning { region }) => {
                let tokens = self.check(seat, RegionMove::Lightning, region)?;
                let army = self.players[seat].active.unwrap();
                self.this_turn.berserk = None;
                self.this_turn.bolts += 1;
                if !self.skag_attack(seat, army.race, region) {
                    self.take_region(seat, army, false, region, tokens, How::Lightning);
                }
                Ok(())
            }
            (Phase::Conquest { ghouls: false }, Action::Zeppelin { region }) => {
                let cost = self.check(seat, RegionMove::Zeppelin, region)?;
                let army = self.players[seat].active.unwrap();
                self.this_turn.berserk = None;
                self.this_turn.zeppelins += 1;
                let die = self.roll();
                self.push(Event::Zeppelin { seat, region, die });
                if die == 0 {
                    // The Zeppelin crashes: the defenders are thrown out, a
                    // token is lost, and the region burns.
                    self.clear_defenders(seat, army.race, false, region);
                    let winter = self.regions[region].winter;
                    self.regions[region] = RegionState { burning: Some(seat), winter, ..RegionState::default() };
                    self.loot[region] = None;
                    let a = self.players[seat].active.as_mut().unwrap();
                    a.hand = a.hand.saturating_sub(1);
                    self.mark_moved(false);
                    return Ok(());
                }
                let needed = cost.saturating_sub(die).max(min_tokens(army.race, false));
                if army.hand < needed {
                    self.end_conquests();
                    return Ok(());
                }
                if !self.skag_attack(seat, army.race, region) {
                    self.take_region(seat, army, false, region, needed, How::Zeppelin);
                }
                Ok(())
            }
            (Phase::Conquest { ghouls: false }, Action::Enchant { region }) => {
                self.check(seat, RegionMove::Enchant, region)?;
                self.enchant(seat, region);
                Ok(())
            }
            (Phase::Conquest { ghouls: false }, Action::Wendigo { region }) => {
                self.check(seat, RegionMove::Wendigo, region)?;
                self.wendigo(seat, region);
                Ok(())
            }
            (Phase::Conquest { ghouls: false }, Action::Fire { region }) => {
                self.check(seat, RegionMove::Fire, region)?;
                self.regions[region].cannon = Some(true);
                self.mark_moved(false);
                Ok(())
            }
            (Phase::Conquest { .. }, Action::Copy { offer }) => {
                let army = self.players[seat].active.ok_or(Error::NoRace)?;
                if army.power != Power::Copycat {
                    return Err(Error::NotYourPower);
                }
                if self.this_turn.moved || self.this_turn.copied {
                    return Err(Error::PowerUsed);
                }
                let power = match offer {
                    Some(i) => {
                        let o = self.offers.get(i).ok_or(Error::NoSuchOffer)?;
                        if o.power == Power::Copycat {
                            return Err(Error::NotHere);
                        }
                        Some(o.power)
                    }
                    None => None,
                };
                self.uncopy(seat);
                self.players[seat].active.as_mut().unwrap().copied = power;
                self.this_turn.copied = true;
                self.push(Event::Copied { seat, power });
                Ok(())
            }
            (Phase::Conquest { .. }, Action::Abandon { region }) => {
                self.check(seat, RegionMove::Abandon, region)?;
                let (army, declined) = self.conqueror(seat).unwrap();
                self.abandon(seat, army.race, region);
                if !declined && army.race == Race::Gypsies {
                    self.this_turn.abandoned.push(region);
                }
                self.mark_moved(declined);
                self.push(Event::Abandoned { seat, region });
                Ok(())
            }
            (Phase::Conquest { .. }, Action::EndConquests) => {
                self.end_conquests();
                Ok(())
            }
            (Phase::Redeploy | Phase::Reinforce, Action::Put { region }) => {
                self.check(seat, RegionMove::Put, region)?;
                let race = self.regions[region].race().unwrap();
                self.army_mut(seat, race).unwrap().hand -= 1;
                self.regions[region].tokens.as_mut().unwrap().count += 1;
                if self.phase == Phase::Reinforce && !self.must_reinforce(seat) {
                    self.reinforce.remove(0);
                    self.reinforce = self.reinforce.iter().copied().filter(|&s| self.must_reinforce(s)).collect();
                    if self.reinforce.is_empty() {
                        self.next_turn();
                    }
                }
                Ok(())
            }
            (Phase::Redeploy, Action::Take { region }) => {
                self.check(seat, RegionMove::Take, region)?;
                let race = self.regions[region].race().unwrap();
                self.regions[region].tokens.as_mut().unwrap().count -= 1;
                self.army_mut(seat, race).unwrap().hand += 1;
                Ok(())
            }
            (Phase::Redeploy, Action::Camp { region }) => {
                self.check(seat, RegionMove::Camp, region)?;
                self.regions[region].encampments += 1;
                Ok(())
            }
            (Phase::Redeploy, Action::Uncamp { region }) => {
                self.check(seat, RegionMove::Uncamp, region)?;
                self.regions[region].encampments -= 1;
                Ok(())
            }
            (Phase::Redeploy, Action::Fortify { region }) => {
                self.check(seat, RegionMove::Fortify, region)?;
                self.regions[region].fortress = true;
                self.this_turn.fortress = true;
                Ok(())
            }
            (Phase::Redeploy, Action::Hero { region }) => {
                self.check(seat, RegionMove::Hero, region)?;
                self.regions[region].hero = !self.regions[region].hero;
                Ok(())
            }
            (Phase::Redeploy, Action::Pot { region }) => {
                self.check(seat, RegionMove::Pot, region)?;
                self.regions[region].pot = !self.regions[region].pot;
                Ok(())
            }
            (Phase::Redeploy, Action::Winter { region }) => {
                self.check(seat, RegionMove::Winter, region)?;
                self.regions[region].winter = true;
                self.players[seat].active.as_mut().unwrap().winter -= 1;
                Ok(())
            }
            (Phase::Redeploy, Action::Lava { region }) => {
                self.check(seat, RegionMove::Lava, region)?;
                self.lava(seat, region);
                Ok(())
            }
            (Phase::Redeploy, Action::Herd { region }) => {
                self.check(seat, RegionMove::Herd, region)?;
                let a = self.players[seat].active.as_mut().unwrap();
                if self.regions[region].behemoths > 0 {
                    self.regions[region].behemoths -= 1;
                    a.behemoths += 1;
                } else {
                    self.regions[region].behemoths += 1;
                    a.behemoths -= 1;
                }
                Ok(())
            }
            (Phase::Redeploy, Action::Cannon { region }) => {
                self.check(seat, RegionMove::Cannon, region)?;
                let c = &mut self.regions[region].cannon;
                *c = if c.is_some() { None } else { Some(false) };
                Ok(())
            }
            (Phase::Conquest { .. } | Phase::Redeploy, Action::Pact { seat: other, give }) => {
                let army = self.players[seat].active.ok_or(Error::NoRace)?;
                if !army.has(Power::Haggling) {
                    return Err(Error::NotYourPower);
                }
                if other >= self.seats() || other == seat {
                    return Err(Error::UnknownSeat);
                }
                if give {
                    let given: u8 = self.pacts.iter().sum();
                    if given >= PACTS {
                        return Err(Error::NoneLeft);
                    }
                    if given as u32 >= self.players[seat].coins {
                        return Err(Error::NotEnoughCoins);
                    }
                    self.pacts[other] += 1;
                } else {
                    if self.pacts[other] == 0 {
                        return Err(Error::NoneLeft);
                    }
                    self.pacts[other] -= 1;
                }
                Ok(())
            }
            (Phase::Redeploy, Action::Ally { seat: ally }) => {
                let army = self.players[seat].active.ok_or(Error::NoRace)?;
                if !army.has(Power::Diplomat) {
                    return Err(Error::NotYourPower);
                }
                if let Some(other) = ally {
                    if other >= self.seats() || other == seat {
                        return Err(Error::UnknownSeat);
                    }
                    if self.this_turn.attacked.contains(&other) {
                        return Err(Error::Attacked);
                    }
                }
                self.players[seat].active.as_mut().unwrap().ally = ally;
                if let Some(other) = ally {
                    self.push(Event::Peace { seat, ally: other });
                }
                Ok(())
            }
            (Phase::Redeploy, Action::EndTurn) => {
                self.check_end_turn(seat)?;
                self.finish_turn(false);
                Ok(())
            }
            (Phase::Redeploy, Action::Decline) => {
                let army = self.players[seat].active.ok_or(Error::NoRace)?;
                if !army.has(Power::Stout) {
                    return Err(Error::TooLateToDecline);
                }
                self.check_end_turn(seat)?;
                self.finish_turn(true);
                Ok(())
            }
            _ => Err(Error::WrongPhase),
        }
    }

    /// Going into decline instead of conquering: it scores and ends the turn.
    fn decline_now(&mut self, seat: usize, tower: Option<usize>) {
        if self.phase == (Phase::Conquest { ghouls: true }) {
            self.unready_ghouls(seat);
        }
        self.decline(seat, tower);
        self.score(seat);
        self.after_turn();
    }

    fn pick(&mut self, seat: usize, offer: usize) -> Result<(), Error> {
        if offer >= self.offers.len() {
            return Err(Error::NoSuchOffer);
        }
        let paid = self.pick_cost(seat, offer);
        if self.players[seat].coins < paid {
            return Err(Error::NotEnoughCoins);
        }
        if !self.players[seat].free_pick {
            for i in 0..offer {
                let o = self.offers[i];
                self.offers[i].coins += if o.power == Power::Cursed { CURSED_SKIP } else { 1 };
                // A skipped Homunculi combo gets a token too.
                if o.race == Race::Homunculi && self.available(o.race) > o.race.tokens() + o.power.tokens() {
                    self.offers[i].tokens += 1;
                }
            }
        }
        self.players[seat].free_pick = false;
        let o = self.offers.remove(offer);
        let tokens = (o.race.tokens() + o.power.tokens() + o.tokens).min(self.available(o.race));
        let p = &mut self.players[seat];
        p.coins = p.coins - paid + o.coins;
        let mut army = Army::new(o.race, o.power, tokens);
        if o.power == Power::HordesOf {
            army.extra = HORDES;
            army.hand += HORDES;
        }
        if o.power == Power::Behemoth {
            army.behemoths = BEHEMOTHS;
        }
        p.active = Some(army);
        self.push(Event::Picked { seat, race: o.race, power: o.power, paid, gained: o.coins });
        // Whoever copied this power loses it.
        for s in 0..self.seats() {
            if s != seat && self.players[s].active.is_some_and(|a| a.copied == Some(o.power)) {
                self.uncopy(s);
            }
        }
        // Racketeers get a coin for each combo skipped.
        if offer > 0 {
            for s in 0..self.seats() {
                if s != seat && self.active_has(s, Power::Racketeering) {
                    self.pay(None, s, offer as u32, Why::Racketeering);
                }
            }
        }
        if o.power == Power::Historian {
            let declined: usize = self.players.iter().map(|p| p.declined.len()).sum();
            self.pay(None, seat, declined as u32, Why::Historian);
        }
        self.refill();
        self.begin_conquests();
        Ok(())
    }

    /// Sorcerers replace a lone active enemy token next to them with one of
    /// their own from the box, once per opponent and turn.
    fn enchant(&mut self, seat: usize, region: usize) {
        let army = self.players[seat].active.unwrap();
        if self.skag_attack(seat, army.race, region) {
            return;
        }
        let t = self.regions[region].tokens.unwrap();
        let (victim, _) = self.owner(t.race).unwrap();
        let before = self.regions[region];
        if before.pot {
            if let Some(a) = self.army_mut(victim, t.race) {
                a.pots += 1;
            }
            self.pay(None, seat, 1, Why::Pot);
        }
        self.regions[region] = RegionState {
            tokens: Some(Tokens { race: Race::Sorcerers, count: 1, declined: false }),
            winter: before.winter,
            ..RegionState::default()
        };
        self.loot[region] = None;
        self.lost_in_conquest(1);
        self.this_turn.enchanted.push(victim);
        if !self.this_turn.attacked.contains(&victim) {
            self.this_turn.attacked.push(victim);
        }
        if !self.this_turn.victims.contains(&t.race) {
            self.this_turn.victims.push(t.race);
        }
        self.this_turn.plundered += 1;
        self.this_turn.conquered.push(region);
        self.this_turn.berserk = None;
        self.mark_moved(false);
        self.push(Event::Enchanted { seat, region, victim, race: t.race });
    }

    /// The Wendigos clear a Forest region: its tokens go back in hand, less
    /// one (all of them for a race in decline), and a Wendigo is born.
    fn wendigo(&mut self, seat: usize, region: usize) {
        let r = self.regions[region];
        let t = r.tokens.unwrap();
        let (owner, declined) = self.owner(t.race).unwrap();
        let kept = if declined { 0 } else { t.count - 1 };
        if let Some(a) = self.army_mut(owner, t.race) {
            a.hand += kept;
            a.behemoths += r.behemoths;
        }
        self.lost_in_conquest(t.count - kept);
        if !declined && !self.this_turn.attacked.contains(&owner) {
            self.this_turn.attacked.push(owner);
        }
        self.regions[region] = RegionState {
            lost_tribe: r.lost_tribe,
            winter: r.winter,
            loot: r.loot,
            ..RegionState::default()
        };
        self.this_turn.wendigo = true;
        self.mark_moved(false);
        if self.available(Race::Wendigos) > 0 {
            self.players[seat].active.as_mut().unwrap().hand += 1;
            self.push(Event::Raised { seat, race: Race::Wendigos, tokens: 1 });
        }
        self.push(Event::Wendigo { seat, region });
    }

    /// Lava fills a region: whoever was there takes their tokens back in
    /// hand (none lost), and nobody else may enter until the next turn.
    fn lava(&mut self, seat: usize, region: usize) {
        let r = self.regions[region];
        if let Some(t) = r.tokens {
            let (owner, declined) = self.owner(t.race).unwrap();
            if let Some(a) = self.army_mut(owner, t.race) {
                a.hand += t.count;
                a.behemoths += r.behemoths;
            }
            if !declined && !self.this_turn.attacked.contains(&owner) {
                self.this_turn.attacked.push(owner);
            }
        }
        self.regions[region] = RegionState { lava: Some(seat), winter: r.winter, ..RegionState::default() };
        self.loot[region] = None;
        self.this_turn.lava += 1;
        self.push(Event::Lava { seat, region });
    }
}

/// Where the Beanstalk and the Stairway stand: two ground regions away from
/// the edge and the water, with at least two regions between them.
fn access_points(map: &Map, rng: &mut u64) -> Vec<usize> {
    let board = Board::new(map.players, None, &[]);
    let mut inland: Vec<usize> =
        (0..board.len()).filter(|&r| !board.border(r) && !board.regions[r].terrain.is_water()).collect();
    shuffle(&mut inland, rng);
    for (i, &a) in inland.iter().enumerate() {
        for &b in &inland[i + 1..] {
            if board.distance(a, b) >= 3 {
                return vec![a, b];
            }
        }
    }
    inland.truncate(2);
    inland
}

/// What `seat` can do right now, worked out for the interface.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Moves {
    /// Positions in the column you can afford, with what each costs.
    pub pick: Vec<(usize, u32)>,
    pub decline: bool,
    /// What can be done to regions: the kind, the region, the tokens it costs.
    pub regions: Vec<Target>,
    /// While conquering: what each region the conquering race may attack
    /// would cost, within reach or not, for planning ahead.
    #[serde(default)]
    pub prices: Vec<(usize, u8)>,
    pub roll: bool,
    pub end_conquests: bool,
    /// Copycat: the combos whose power you can copy (empty: no copying now).
    pub copy: Vec<usize>,
    /// Seats you may make peace with.
    pub allies: Vec<usize>,
    /// Haggling: seats you may give a Trade Pact to, and take one back from.
    pub give_pact: Vec<usize>,
    pub take_pact: Vec<usize>,
    /// Why the turn can't end yet (`None`: it can, or it isn't the time).
    pub end_turn: Option<String>,
    pub can_end_turn: bool,
}

/// A move on a region, with the tokens it costs (0 when none).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub kind: RegionMove,
    pub region: usize,
    pub cost: u8,
}

impl Moves {
    /// The regions a kind of move can act on.
    pub fn list(&self, kind: RegionMove) -> Vec<usize> {
        self.regions.iter().filter(|t| t.kind == kind).map(|t| t.region).collect()
    }

    /// The regions a kind of move can act on, with their costs.
    pub fn costs(&self, kind: RegionMove) -> Vec<(usize, u8)> {
        self.regions.iter().filter(|t| t.kind == kind).map(|t| (t.region, t.cost)).collect()
    }

    pub fn conquer(&self) -> Vec<(usize, u8)> {
        self.costs(RegionMove::Conquer)
    }

    pub fn gamble(&self) -> Vec<(usize, u8)> {
        self.costs(RegionMove::Gamble)
    }

    pub fn can(&self, kind: RegionMove, region: usize) -> bool {
        self.regions.iter().any(|t| t.kind == kind && t.region == region)
    }
}

impl State {
    /// Everything `seat` may do now (nothing when it isn't their move).
    pub fn moves(&self, seat: usize) -> Moves {
        let mut m = Moves::default();
        if self.actor() != Some(seat) {
            return m;
        }
        let kinds: &[RegionMove] = match self.phase {
            Phase::Pick | Phase::Over => &[],
            Phase::Conquest { .. } => &[
                RegionMove::Conquer,
                RegionMove::Gamble,
                RegionMove::Hire,
                RegionMove::Fireball,
                RegionMove::Behemoth,
                RegionMove::Catapult,
                RegionMove::Dragon,
                RegionMove::Lightning,
                RegionMove::Zeppelin,
                RegionMove::Enchant,
                RegionMove::Wendigo,
                RegionMove::Fire,
                RegionMove::Abandon,
                RegionMove::Tower,
            ],
            Phase::Redeploy => &[
                RegionMove::Put,
                RegionMove::Take,
                RegionMove::Camp,
                RegionMove::Uncamp,
                RegionMove::Fortify,
                RegionMove::Hero,
                RegionMove::Pot,
                RegionMove::Winter,
                RegionMove::Lava,
                RegionMove::Herd,
                RegionMove::Cannon,
            ],
            Phase::Reinforce => &[RegionMove::Put],
        };
        // Skip whole kinds the race and power can't do at all.
        let army = self.players[seat].active;
        let usable = |kind: RegionMove| -> bool {
            let has = |p: Power| army.is_some_and(|a| a.has(p));
            let race = |r: Race| army.is_some_and(|a| a.race == r);
            match kind {
                RegionMove::Hire => has(Power::Mercenary),
                RegionMove::Fireball => has(Power::Fireball),
                RegionMove::Behemoth | RegionMove::Herd => has(Power::Behemoth),
                RegionMove::Catapult => has(Power::Catapult),
                RegionMove::Dragon => has(Power::DragonMaster) || race(Race::Drakons),
                RegionMove::Lightning => race(Race::StormGiants),
                RegionMove::Zeppelin => has(Power::Zeppelined),
                RegionMove::Enchant => race(Race::Sorcerers),
                RegionMove::Wendigo => race(Race::Wendigos),
                RegionMove::Fire | RegionMove::Cannon => has(Power::Gunner),
                RegionMove::Tower => race(Race::Priestesses),
                RegionMove::Camp | RegionMove::Uncamp => has(Power::Bivouacking),
                RegionMove::Fortify => has(Power::Fortified),
                RegionMove::Hero => has(Power::Heroic),
                RegionMove::Pot => race(Race::Leprechauns),
                RegionMove::Winter => race(Race::IceWitches),
                RegionMove::Lava => has(Power::Lava),
                _ => true,
            }
        };
        for &kind in kinds.iter().filter(|&&k| usable(k)) {
            for region in 0..self.regions.len() {
                if let Ok(cost) = self.check(seat, kind, region) {
                    m.regions.push(Target { kind, region, cost });
                }
            }
        }
        match self.phase {
            Phase::Pick => {
                m.pick = (0..self.offers.len())
                    .map(|i| (i, self.pick_cost(seat, i)))
                    .filter(|&(_, cost)| self.players[seat].coins >= cost)
                    .collect();
            }
            Phase::Conquest { ghouls } => {
                m.decline = self.this_turn.may_decline && !self.this_turn.moved;
                m.end_conquests = true;
                if let Some((army, declined)) = self.conqueror(seat) {
                    m.prices = (0..self.regions.len())
                        .filter(|&r| self.target_anywhere(seat, &army, declined, r).is_ok())
                        .map(|r| (r, self.die_cost(&army, declined, self.cost(seat, &army, declined, r))))
                        .collect();
                }
                if !ghouls {
                    if let Some(a) = army {
                        m.roll = a.has(Power::Berserk) && self.this_turn.berserk.is_none() && a.hand > 0;
                    }
                }
                if let Some(a) = army {
                    if a.power == Power::Copycat && !self.this_turn.moved && !self.this_turn.copied {
                        m.copy = (0..self.offers.len()).filter(|&i| self.offers[i].power != Power::Copycat).collect();
                    }
                }
            }
            Phase::Redeploy => {
                if army.is_some_and(|a| a.has(Power::Diplomat)) {
                    m.allies =
                        (0..self.seats()).filter(|&s| s != seat && !self.this_turn.attacked.contains(&s)).collect();
                }
                match self.check_end_turn(seat) {
                    Ok(()) => m.can_end_turn = true,
                    Err(e) => m.end_turn = Some(e.message().to_string()),
                }
                m.decline = m.can_end_turn && army.is_some_and(|a| a.has(Power::Stout));
            }
            Phase::Reinforce | Phase::Over => {}
        }
        if matches!(self.phase, Phase::Conquest { .. } | Phase::Redeploy) && army.is_some_and(|a| a.has(Power::Haggling)) {
            let given: u8 = self.pacts.iter().sum();
            if given < PACTS && (given as u32) < self.players[seat].coins {
                m.give_pact = (0..self.seats()).filter(|&s| s != seat).collect();
            }
            m.take_pact = (0..self.seats()).filter(|&s| self.pacts[s] > 0).collect();
        }
        m
    }
}
