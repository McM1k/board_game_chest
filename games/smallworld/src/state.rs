//! The game state and its rules.
//!
//! Players take turns, clockwise from seat 0, for as many game turns as the
//! board says (10 with two or three players, 9 with four, 8 with five).
//!
//! * **First turn**, and whenever a player has no active race: pick a race
//!   and power from the column of six. The top one is free; each one lower
//!   costs a coin more, dropped on the combos above it, and whoever takes a
//!   combo pockets the coins on it. The race comes with as many tokens as
//!   the banner and the badge add up to.
//! * **Conquest.** A region costs 2 tokens, plus 1 for each Mountain,
//!   Troll's Lair, Fortress or Encampment in it, plus 1 for each token
//!   defending it (a Lost Tribe, another player's race), less what the race
//!   and power take off (never below 1). A race enters the map through a
//!   border region, then conquers next to the regions it holds. Each
//!   defender loses one token for good (Elves none) and puts the others back
//!   on their own regions at the end of the turn. The last conquest of a
//!   turn can be tried with up to 3 tokens missing: the reinforcement die
//!   (0, 0, 0, 1, 2, 3) makes up the difference, or the conquests end there.
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
//! Coins are hidden from the other players until the end. The most coins
//! win; a tie goes to whoever has the most tokens on the map.
//!
//! The rules are pure: the only randomness, the shuffles and the die, comes
//! from the seed the game is created with.

use crate::maps::{Map, Terrain};
use crate::races::{Power, Race};
use serde::{Deserialize, Serialize};

pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 5;
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
/// How many events [`State::log`] keeps.
const LOG_LEN: usize = 40;

/// A race and a power face up in the column, with the coins dropped on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub race: Race,
    pub power: Power,
    pub coins: u32,
}

/// A race a player controls, active or in decline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Army {
    pub race: Race,
    pub power: Power,
    /// Tokens in hand: to conquer with during the turn, or to put back on
    /// the map after losing a region.
    pub hand: u8,
    /// Amazons: the four tokens set aside after redeploying, back in hand at
    /// the next turn.
    pub resting: u8,
    /// Turns finished with this race (Wealthy pays at the end of the first).
    pub turns: u8,
    /// Halflings: Holes-in-the-Ground placed so far (two, ever).
    pub holes: u8,
    /// Diplomat: the seat at peace with this race, who can't attack it
    /// until its owner's next turn.
    pub ally: Option<usize>,
}

impl Army {
    fn new(race: Race, power: Power, tokens: u8) -> Army {
        Army { race, power, hand: tokens, resting: 0, turns: 0, holes: 0, ally: None }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    /// Victory coins (only their owner sees them before the end).
    pub coins: u32,
    pub active: Option<Army>,
    /// Races in decline: one, plus one with the Spirit power.
    pub declined: Vec<Army>,
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
    /// Dragon Master: immune.
    pub dragon: bool,
}

impl RegionState {
    /// Immune to conquests and to the races' and powers' tricks.
    pub fn immune(&self) -> bool {
        self.hole || self.hero || self.dragon
    }

    /// Holds a Lost Tribe or race tokens (a Mountain alone is empty).
    pub fn occupied(&self) -> bool {
        self.lost_tribe || self.tokens.is_some()
    }

    fn race(&self) -> Option<Race> {
        self.tokens.map(|t| t.race)
    }
}

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
    /// Seats whose active race was attacked (Diplomat can't ally with them).
    pub attacked: Vec<usize>,
    /// Seats the Sorcerers already took a token from.
    pub enchanted: Vec<usize>,
    /// The Dragon has flown this turn.
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Take the combo at this position of the column (0 is free).
    Pick { offer: usize },
    /// Put the active race in decline: at the start of the turn instead of
    /// conquering, or (Stout) at the end of a turn of conquests.
    Decline,
    /// Conquer a region with the tokens it costs.
    Conquer { region: usize },
    /// The last conquest of the turn, with tokens missing: roll the die.
    Gamble { region: usize },
    /// Berserk: roll the die for the next conquest.
    Roll,
    /// Dragon Master: conquer with one token, whatever defends the region.
    Dragon { region: usize },
    /// Sorcerers: replace a lone enemy token with one of yours.
    Enchant { region: usize },
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
    /// Diplomat: make peace with this seat until your next turn (`None`:
    /// nobody).
    Ally { seat: Option<usize> },
    /// Score and end the turn.
    EndTurn,
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
    NoEncampment,
    Attacked,
    NotHere,
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
            Error::NotEnoughCoins => "You don't have enough coins for that one.",
            Error::NoSuchRegion => "There's no such region.",
            Error::NoRace => "You have no race to do that with.",
            Error::TooLateToDecline => "You can only go into decline before conquering anything this turn.",
            Error::AlreadyYours => "That region is already yours.",
            Error::Water => "Only a Seafaring race can conquer the seas and the lake.",
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
            Error::NoEncampment => "There's no encampment there.",
            Error::Attacked => "You attacked that player this turn: no peace with them.",
            Error::NotHere => "That can't go there.",
        }
    }
}

/// Who defended a conquered region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Foe {
    LostTribe,
    Race { seat: usize, race: Race },
}

/// Something that happened, kept (briefly) for the log. Seats are indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    TurnStarted(u8),
    Picked { seat: usize, race: Race, power: Power, paid: u32, gained: u32 },
    Conquered { seat: usize, race: Race, region: usize, tokens: u8, foe: Option<Foe>, dragon: bool },
    /// The reinforcement die, for a last conquest.
    Gambled { seat: usize, region: usize, die: u8, won: bool },
    /// Berserk's die, before a conquest.
    Rolled { seat: usize, die: u8 },
    Enchanted { seat: usize, region: usize, victim: usize, race: Race },
    Abandoned { seat: usize, region: usize },
    /// Skeletons raised new tokens.
    Raised { seat: usize, tokens: u8 },
    Declined { seat: usize, race: Race },
    /// A race in decline left the map.
    Vanished { seat: usize, race: Race },
    Scored { seat: usize, coins: u32 },
    Peace { seat: usize, ally: usize },
    GameOver,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
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

impl State {
    /// A new game for 2 to 5 players, on the board for that many. The same
    /// seed gives the same game.
    pub fn new(players: usize, seed: u64) -> State {
        assert!((MIN_PLAYERS..=MAX_PLAYERS).contains(&players), "the game is for {MIN_PLAYERS} to {MAX_PLAYERS} players");
        let map = Map::for_players(players);
        let mut rng = seed;
        let mut races = Race::ALL.to_vec();
        shuffle(&mut races, &mut rng);
        let mut powers = Power::ALL.to_vec();
        shuffle(&mut powers, &mut rng);
        let regions = map
            .regions
            .iter()
            .map(|r| RegionState { lost_tribe: r.lost_tribe(), ..RegionState::default() })
            .collect();
        let mut state = State {
            players: vec![Player { coins: START_COINS, active: None, declined: Vec::new() }; players],
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
            rng,
        };
        state.refill();
        state.push(Event::TurnStarted(1));
        state.begin_turn();
        state
    }

    pub fn seats(&self) -> usize {
        self.players.len()
    }

    /// The board this game is played on.
    pub fn map(&self) -> &'static Map {
        Map::for_players(self.seats())
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

    /// The regions `race` holds.
    pub fn regions_of(&self, race: Race) -> Vec<usize> {
        (0..self.regions.len()).filter(|&r| self.regions[r].race() == Some(race)).collect()
    }

    /// Tokens of `race` on the map.
    pub fn on_map(&self, race: Race) -> u8 {
        self.regions.iter().filter_map(|r| r.tokens).filter(|t| t.race == race).map(|t| t.count).sum()
    }

    /// Tokens of `race` still in the box.
    pub fn available(&self, race: Race) -> u8 {
        let held = self.owner(race).and_then(|(seat, _)| self.army(seat, race)).map_or(0, |a| a.hand + a.resting);
        race.supply().saturating_sub(self.on_map(race) + held)
    }

    /// The Ghouls in decline that conquer at the start of `seat`'s turns.
    fn decline_ghouls(&self, seat: usize) -> Option<&Army> {
        self.players[seat].declined.iter().find(|a| a.race == Race::Ghouls)
    }

    /// The race conquering in the current phase.
    fn conqueror(&self, seat: usize) -> Option<(Army, bool)> {
        match self.phase {
            Phase::Conquest { ghouls: true } => self.decline_ghouls(seat).map(|a| (*a, true)),
            _ => self.players[seat].active.map(|a| (a, false)),
        }
    }

    // ----- conquest --------------------------------------------------------------------

    /// Whether `army` can reach `region`: through the border when it holds
    /// nothing yet, next to its regions otherwise.
    fn reach(&self, army: &Army, declined: bool, region: usize) -> Result<(), Error> {
        let map = self.map();
        let held = self.regions_of(army.race);
        let flying = !declined && army.power == Power::Flying;
        if held.is_empty() {
            let anywhere = !declined && (army.race == Race::Halflings || flying);
            return if anywhere || map.border(region) { Ok(()) } else { Err(Error::NotABorder) };
        }
        if flying || self.next_to(army, declined, region) {
            Ok(())
        } else {
            Err(Error::OutOfReach)
        }
    }

    /// Next to one of `army`'s regions (with Underworld, every Cavern is
    /// next to every other).
    fn next_to(&self, army: &Army, declined: bool, region: usize) -> bool {
        let map = self.map();
        let held = self.regions_of(army.race);
        held.iter().any(|&h| map.adjacent(h, region))
            || (!declined
                && army.power == Power::Underworld
                && map.regions[region].cavern()
                && held.iter().any(|&h| map.regions[h].cavern()))
    }

    /// Checks that `seat`'s conquering race may attack `region` at all
    /// (whose region it is, the water, immunity, peace, reach).
    fn target(&self, seat: usize, army: &Army, declined: bool, region: usize) -> Result<(), Error> {
        let map = self.map();
        let r = self.regions.get(region).ok_or(Error::NoSuchRegion)?;
        if r.race() == Some(army.race) {
            return Err(Error::AlreadyYours);
        }
        if map.regions[region].terrain.is_water() && (declined || army.power != Power::Seafaring) {
            return Err(Error::Water);
        }
        if r.immune() {
            return Err(Error::Immune);
        }
        if !declined {
            if let Some(t) = r.tokens.filter(|t| !t.declined) {
                let (owner, _) = self.owner(t.race).expect("tokens on the map have an owner");
                if self.army(owner, t.race).is_some_and(|a| a.power == Power::Diplomat && a.ally == Some(seat)) {
                    return Err(Error::AtPeace);
                }
            }
        }
        self.reach(army, declined, region)
    }

    /// What conquering `region` costs `seat`'s conquering race, before any
    /// die: 2, plus its defenses and defenders, less the race's and power's
    /// discounts, at least 1.
    fn cost(&self, seat: usize, army: &Army, declined: bool, region: usize) -> u8 {
        let map = self.map();
        let r = &self.regions[region];
        let def = &map.regions[region];
        let mut cost = 2 + def.mountain() as u8 + r.lair as u8 + r.fortress as u8 + r.encampments;
        cost += r.lost_tribe as u8;
        if let Some(t) = r.tokens {
            if self.owner(t.race).map(|(s, _)| s) != Some(seat) {
                cost += t.count;
            }
        }
        if !declined {
            let mut off = 0;
            off += (army.power == Power::Commando) as u8;
            off += (army.power == Power::Mounted && matches!(def.terrain, Terrain::Hill | Terrain::Farmland)) as u8;
            off += (army.power == Power::Underworld && def.cavern()) as u8;
            off += (army.race == Race::Tritons && map.coastal(region)) as u8;
            if army.race == Race::Giants {
                let near_mountain = self
                    .regions_of(Race::Giants)
                    .iter()
                    .any(|&h| map.regions[h].mountain() && map.adjacent(h, region));
                off += near_mountain as u8;
            }
            cost = cost.saturating_sub(off);
        }
        cost.max(1)
    }

    /// The cost of conquering `region` now, if it can be attacked at all
    /// (Berserk's die already taken off).
    pub fn conquest_cost(&self, seat: usize, region: usize) -> Result<u8, Error> {
        let (army, declined) = self.conqueror(seat).ok_or(Error::NoRace)?;
        self.target(seat, &army, declined, region)?;
        let cost = self.cost(seat, &army, declined, region);
        Ok(match self.this_turn.berserk {
            Some(die) if !declined => cost.saturating_sub(die).max(1),
            _ => cost,
        })
    }

    /// Takes `region` with `tokens` of the conquering race. The defenders
    /// lose a token (Elves none) and keep the rest in hand.
    fn take_region(&mut self, seat: usize, army: Army, declined: bool, region: usize, tokens: u8, dragon: bool) {
        let r = self.regions[region];
        let mut foe = None;
        if r.lost_tribe {
            foe = Some(Foe::LostTribe);
        }
        if let Some(t) = r.tokens {
            let (owner, owner_declined) = self.owner(t.race).expect("tokens on the map have an owner");
            let lost = if t.race == Race::Elves && !owner_declined { 0 } else { 1 };
            if let Some(a) = self.army_mut(owner, t.race) {
                a.hand += t.count.saturating_sub(lost);
            }
            if !owner_declined && owner != seat && !self.this_turn.attacked.contains(&owner) {
                self.this_turn.attacked.push(owner);
            }
            foe = Some(Foe::Race { seat: owner, race: t.race });
        }
        if foe.is_some() && !declined {
            self.this_turn.plundered += 1;
        }
        let mut placed = RegionState {
            tokens: Some(Tokens { race: army.race, count: tokens, declined }),
            dragon,
            ..RegionState::default()
        };
        if !declined {
            if army.race == Race::Trolls {
                placed.lair = true;
            }
            if army.race == Race::Halflings && army.holes < HOLES {
                placed.hole = true;
            }
        }
        self.regions[region] = placed;
        let me = self.army_mut(seat, army.race).expect("the conquering race");
        me.hand -= tokens;
        if placed.hole {
            me.holes += 1;
        }
        self.this_turn.conquered.push(region);
        self.mark_moved(declined);
        self.push(Event::Conquered { seat, race: army.race, region, tokens, foe, dragon });
    }

    fn mark_moved(&mut self, declined: bool) {
        self.this_turn.moved = true;
        if declined {
            self.this_turn.ghouls_moved = true;
        }
    }

    // ----- turn flow ---------------------------------------------------------------------

    /// The current player's turn begins: pick a race, or ready the troops.
    fn begin_turn(&mut self) {
        let seat = self.current;
        self.this_turn = Turn::default();
        // Peace made by this player ends now.
        if let Some(a) = self.players[seat].active.as_mut() {
            a.ally = None;
        }
        if self.players[seat].active.is_none() {
            self.phase = Phase::Pick;
            return;
        }
        self.this_turn.may_decline = true;
        let race = self.players[seat].active.unwrap().race;
        self.this_turn.lifted = self.ready(seat, race);
        self.begin_conquests();
    }

    /// Leaves one token in each region of `race` and takes the others in
    /// hand (with the Amazons' resting four). Returns what was lifted where.
    fn ready(&mut self, seat: usize, race: Race) -> Vec<(usize, u8)> {
        let mut lifted = Vec::new();
        let mut taken = 0;
        for (i, r) in self.regions.iter_mut().enumerate() {
            if let Some(t) = r.tokens.as_mut().filter(|t| t.race == race && t.count > 1) {
                lifted.push((i, t.count - 1));
                taken += t.count - 1;
                t.count = 1;
            }
        }
        let army = self.army_mut(seat, race).expect("a race to ready");
        army.hand += taken + army.resting;
        army.resting = 0;
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
        // Skeletons raise a token for every two non-empty regions conquered.
        if let Some(a) = self.players[seat].active.filter(|a| a.race == Race::Skeletons) {
            let raised = (self.this_turn.plundered / 2).min(self.available(a.race));
            if raised > 0 {
                self.players[seat].active.as_mut().unwrap().hand += raised;
                self.push(Event::Raised { seat, tokens: raised });
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
                if a.hand > rest {
                    return Err(if rest > 0 { Error::RestAmazons } else { Error::TokensInHand });
                }
                if a.hand < rest {
                    return Err(Error::RestAmazons);
                }
                if a.power == Power::Heroic && self.heroes() < HEROES.min(held.len()) {
                    return Err(Error::PlaceHeroes);
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

    fn heroes(&self) -> usize {
        self.regions.iter().filter(|r| r.hero).count()
    }

    /// The end of a turn of conquests: Amazons rest, coins are scored, and
    /// a Stout race may go into decline.
    fn finish_turn(&mut self, stout_decline: bool) {
        let seat = self.current;
        if let Some(a) = self.players[seat].active.as_mut() {
            if a.race == Race::Amazons {
                a.resting = a.hand;
                a.hand = 0;
            }
        }
        self.score(seat);
        if let Some(a) = self.players[seat].active.as_mut() {
            a.turns = a.turns.saturating_add(1);
        }
        if stout_decline {
            self.decline(seat);
        }
        self.after_turn();
    }

    /// Coins for the turn: one per region of every race the player has on
    /// the map, plus the active race's and power's bonuses.
    fn score(&mut self, seat: usize) {
        let coins = self.turn_coins(seat);
        self.players[seat].coins += coins;
        self.push(Event::Scored { seat, coins });
    }

    /// What `seat` would score if their turn ended now.
    pub fn turn_coins(&self, seat: usize) -> u32 {
        let map = self.map();
        let p = &self.players[seat];
        let count = |regions: &[usize], f: &dyn Fn(usize) -> bool| regions.iter().filter(|&&r| f(r)).count() as u32;
        let mut coins = 0;
        for army in p.active.iter().chain(&p.declined) {
            let held = self.regions_of(army.race);
            coins += held.len() as u32;
            // Dwarves keep their mines in decline.
            if army.race == Race::Dwarves {
                coins += count(&held, &|r| map.regions[r].mine());
            }
        }
        if let Some(a) = p.active {
            let held = self.regions_of(a.race);
            let terrain = |t: Terrain| count(&held, &|r| map.regions[r].terrain == t);
            let plundered = self.this_turn.plundered as u32;
            coins += match a.race {
                Race::Humans => terrain(Terrain::Farmland),
                Race::Wizards => count(&held, &|r| map.regions[r].magic()),
                Race::Orcs => plundered,
                _ => 0,
            };
            coins += match a.power {
                Power::Alchemist => 2,
                Power::Forest => terrain(Terrain::Forest),
                Power::Hill => terrain(Terrain::Hill),
                Power::Swamp => terrain(Terrain::Swamp),
                Power::Merchant => held.len() as u32,
                Power::Pillaging => plundered,
                Power::Fortified => count(&held, &|r| self.regions[r].fortress),
                Power::Wealthy if a.turns == 0 => WEALTHY_COINS,
                _ => 0,
            };
        }
        coins
    }

    /// The active race goes into decline: one token stays in each region
    /// (Ghouls keep them all), the power is lost (Spirit's is kept), and the
    /// older race in decline leaves the map (unless one of the two is
    /// Spirit's).
    fn decline(&mut self, seat: usize) {
        let Some(active) = self.players[seat].active else { return };
        // Ghouls keep every token: what was lifted to conquer goes back.
        if active.race == Race::Ghouls {
            let lifted = std::mem::take(&mut self.this_turn.lifted);
            self.unready(seat, active.race, &lifted);
        }
        let mut army = self.players[seat].active.take().unwrap();
        for r in self.regions.iter_mut() {
            if let Some(t) = r.tokens.as_mut().filter(|t| t.race == army.race) {
                if army.race != Race::Ghouls {
                    t.count = 1;
                }
                t.declined = true;
                r.hole = false;
                r.hero = false;
                r.dragon = false;
                r.encampments = 0;
            }
        }
        army.hand = 0;
        army.resting = 0;
        army.ally = None;
        if army.power != Power::Spirit {
            self.discarded_powers.push(army.power);
            if let Some(old) = self.players[seat].declined.iter().position(|a| a.power != Power::Spirit) {
                let old = self.players[seat].declined.remove(old);
                self.vanish(seat, old);
            }
        }
        self.players[seat].declined.push(army);
        self.push(Event::Declined { seat, race: army.race });
    }

    /// A race in decline leaves the map: its tokens go back in the box, its
    /// banner to the bottom of the stack.
    fn vanish(&mut self, seat: usize, army: Army) {
        for r in self.regions.iter_mut() {
            if r.race() == Some(army.race) {
                *r = RegionState::default();
            }
        }
        if army.power == Power::Spirit {
            self.discarded_powers.push(army.power);
        }
        self.races.push(army.race);
        self.push(Event::Vanished { seat, race: army.race });
        self.refill();
    }

    /// After a turn: whoever lost regions puts their tokens back, then the
    /// next player's turn.
    fn after_turn(&mut self) {
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
                if self.regions_of(a.race).is_empty() {
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
            if self.turn >= self.map().turns {
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
            self.offers.push(Offer { race, power, coins: 0 });
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
                if self.phase == (Phase::Conquest { ghouls: true }) {
                    self.unready_ghouls(seat);
                }
                self.decline(seat);
                self.score(seat);
                self.after_turn();
                Ok(())
            }
            (Phase::Conquest { .. }, Action::Conquer { region }) => {
                let (army, declined) = self.conqueror(seat).ok_or(Error::NoRace)?;
                let cost = self.conquest_cost(seat, region)?;
                if army.hand == 0 {
                    return Err(Error::NoTokens);
                }
                if army.hand < cost {
                    return Err(Error::NotEnoughTokens);
                }
                self.this_turn.berserk = None;
                self.take_region(seat, army, declined, region, cost, false);
                Ok(())
            }
            (Phase::Conquest { .. }, Action::Gamble { region }) => {
                let (army, declined) = self.conqueror(seat).ok_or(Error::NoRace)?;
                if !declined && army.power == Power::Berserk {
                    return Err(Error::NotYourPower);
                }
                let cost = self.conquest_cost(seat, region)?;
                if army.hand == 0 {
                    return Err(Error::NoTokens);
                }
                if army.hand >= cost {
                    return Err(Error::EnoughTokens);
                }
                if cost - army.hand > DIE_MAX {
                    return Err(Error::TooFar);
                }
                let die = self.roll();
                let won = army.hand + die >= cost;
                self.push(Event::Gambled { seat, region, die, won });
                if won {
                    self.take_region(seat, army, declined, region, army.hand, false);
                }
                self.end_conquests();
                Ok(())
            }
            (Phase::Conquest { ghouls: false }, Action::Roll) => {
                let army = self.players[seat].active.ok_or(Error::NoRace)?;
                if army.power != Power::Berserk {
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
            (Phase::Conquest { ghouls: false }, Action::Dragon { region }) => {
                let army = self.players[seat].active.ok_or(Error::NoRace)?;
                if army.power != Power::DragonMaster {
                    return Err(Error::NotYourPower);
                }
                if self.this_turn.dragon {
                    return Err(Error::PowerUsed);
                }
                self.target(seat, &army, false, region)?;
                if army.hand == 0 {
                    return Err(Error::NoTokens);
                }
                for r in self.regions.iter_mut() {
                    r.dragon = false;
                }
                self.this_turn.dragon = true;
                self.this_turn.berserk = None;
                self.take_region(seat, army, false, region, 1, true);
                Ok(())
            }
            (Phase::Conquest { ghouls: false }, Action::Enchant { region }) => self.enchant(seat, region),
            (Phase::Conquest { .. }, Action::Abandon { region }) => {
                let (army, declined) = self.conqueror(seat).ok_or(Error::NoRace)?;
                let r = self.regions.get(region).ok_or(Error::NoSuchRegion)?;
                let t = r.tokens.filter(|t| t.race == army.race).ok_or(Error::NotYours)?;
                if self.this_turn.conquered.contains(&region) {
                    return Err(Error::StaysPut);
                }
                self.regions[region] = RegionState::default();
                self.army_mut(seat, army.race).unwrap().hand += t.count;
                self.mark_moved(declined);
                self.push(Event::Abandoned { seat, region });
                Ok(())
            }
            (Phase::Conquest { .. }, Action::EndConquests) => {
                self.end_conquests();
                Ok(())
            }
            (Phase::Redeploy | Phase::Reinforce, Action::Put { region }) => {
                let r = self.regions.get(region).ok_or(Error::NoSuchRegion)?;
                let race = r.race().ok_or(Error::NotYours)?;
                let army = *self.army(seat, race).ok_or(Error::NotYours)?;
                let declined = self.owner(race).is_some_and(|(_, d)| d);
                // In decline, only Ghouls redeploy on their own turn.
                if self.phase == Phase::Redeploy && declined && race != Race::Ghouls {
                    return Err(Error::NotYours);
                }
                if army.hand == 0 {
                    return Err(Error::HandEmpty);
                }
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
                let r = self.regions.get(region).ok_or(Error::NoSuchRegion)?;
                let t = r.tokens.ok_or(Error::NotYours)?;
                let mine = self.owner(t.race).is_some_and(|(s, d)| s == seat && (!d || t.race == Race::Ghouls));
                if !mine {
                    return Err(Error::NotYours);
                }
                if t.count <= 1 {
                    return Err(Error::LastToken);
                }
                self.regions[region].tokens.as_mut().unwrap().count -= 1;
                self.army_mut(seat, t.race).unwrap().hand += 1;
                Ok(())
            }
            (Phase::Redeploy, Action::Camp { region }) => {
                self.own_active_region(seat, region, Power::Bivouacking)?;
                let pitched: u8 = self.regions.iter().map(|r| r.encampments).sum();
                if pitched >= ENCAMPMENTS {
                    return Err(Error::NoneLeft);
                }
                self.regions[region].encampments += 1;
                Ok(())
            }
            (Phase::Redeploy, Action::Uncamp { region }) => {
                self.own_active_region(seat, region, Power::Bivouacking)?;
                if self.regions[region].encampments == 0 {
                    return Err(Error::NoEncampment);
                }
                self.regions[region].encampments -= 1;
                Ok(())
            }
            (Phase::Redeploy, Action::Fortify { region }) => {
                self.own_active_region(seat, region, Power::Fortified)?;
                if self.this_turn.fortress {
                    return Err(Error::PowerUsed);
                }
                if self.regions[region].fortress {
                    return Err(Error::NotHere);
                }
                if self.regions.iter().filter(|r| r.fortress).count() >= FORTRESSES {
                    return Err(Error::NoneLeft);
                }
                self.regions[region].fortress = true;
                self.this_turn.fortress = true;
                Ok(())
            }
            (Phase::Redeploy, Action::Hero { region }) => {
                self.own_active_region(seat, region, Power::Heroic)?;
                if self.regions[region].hero {
                    self.regions[region].hero = false;
                } else if self.heroes() >= HEROES {
                    return Err(Error::NoneLeft);
                } else {
                    self.regions[region].hero = true;
                }
                Ok(())
            }
            (Phase::Redeploy, Action::Ally { seat: ally }) => {
                let army = self.players[seat].active.ok_or(Error::NoRace)?;
                if army.power != Power::Diplomat {
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
                if army.power != Power::Stout {
                    return Err(Error::TooLateToDecline);
                }
                self.check_end_turn(seat)?;
                self.finish_turn(true);
                Ok(())
            }
            _ => Err(Error::WrongPhase),
        }
    }

    fn pick(&mut self, seat: usize, offer: usize) -> Result<(), Error> {
        if offer >= self.offers.len() {
            return Err(Error::NoSuchOffer);
        }
        let paid = offer as u32;
        if self.players[seat].coins < paid {
            return Err(Error::NotEnoughCoins);
        }
        for o in &mut self.offers[..offer] {
            o.coins += 1;
        }
        let o = self.offers.remove(offer);
        let tokens = (o.race.tokens() + o.power.tokens()).min(self.available(o.race));
        let p = &mut self.players[seat];
        p.coins = p.coins - paid + o.coins;
        p.active = Some(Army::new(o.race, o.power, tokens));
        self.push(Event::Picked { seat, race: o.race, power: o.power, paid, gained: o.coins });
        self.refill();
        self.begin_conquests();
        Ok(())
    }

    /// Sorcerers replace a lone active enemy token next to them with one of
    /// their own from the box, once per opponent and turn.
    fn enchant(&mut self, seat: usize, region: usize) -> Result<(), Error> {
        let army = self.players[seat].active.ok_or(Error::NoRace)?;
        self.enchant_target(seat, &army, region)?;
        let t = self.regions[region].tokens.unwrap();
        let (victim, _) = self.owner(t.race).unwrap();
        self.regions[region] = RegionState {
            tokens: Some(Tokens { race: Race::Sorcerers, count: 1, declined: false }),
            ..RegionState::default()
        };
        self.this_turn.enchanted.push(victim);
        if !self.this_turn.attacked.contains(&victim) {
            self.this_turn.attacked.push(victim);
        }
        self.this_turn.plundered += 1;
        self.this_turn.conquered.push(region);
        self.this_turn.berserk = None;
        self.mark_moved(false);
        self.push(Event::Enchanted { seat, region, victim, race: t.race });
        Ok(())
    }

    fn enchant_target(&self, seat: usize, army: &Army, region: usize) -> Result<(), Error> {
        if army.race != Race::Sorcerers {
            return Err(Error::NotYourPower);
        }
        let r = self.regions.get(region).ok_or(Error::NoSuchRegion)?;
        let t = r.tokens.ok_or(Error::NotAlone)?;
        let (victim, declined) = self.owner(t.race).unwrap();
        if victim == seat {
            return Err(Error::AlreadyYours);
        }
        if declined || t.count != 1 || r.encampments > 0 {
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

    /// Checks that `region` is held by `seat`'s active race, whose power is `power`.
    fn own_active_region(&self, seat: usize, region: usize, power: Power) -> Result<(), Error> {
        let army = self.players[seat].active.ok_or(Error::NoRace)?;
        if army.power != power {
            return Err(Error::NotYourPower);
        }
        let r = self.regions.get(region).ok_or(Error::NoSuchRegion)?;
        if r.tokens.is_some_and(|t| t.race == army.race) {
            Ok(())
        } else {
            Err(Error::NotYours)
        }
    }
}

/// What `seat` can do right now, worked out for the interface.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Moves {
    /// Positions in the column you can afford.
    pub pick: Vec<usize>,
    pub decline: bool,
    /// Regions you can conquer, and what each costs.
    pub conquer: Vec<(usize, u8)>,
    /// Regions to try with the die, and what each costs.
    pub gamble: Vec<(usize, u8)>,
    pub roll: bool,
    pub dragon: Vec<usize>,
    pub enchant: Vec<usize>,
    pub abandon: Vec<usize>,
    pub end_conquests: bool,
    pub put: Vec<usize>,
    pub take: Vec<usize>,
    pub camp: Vec<usize>,
    pub uncamp: Vec<usize>,
    pub fortify: Vec<usize>,
    pub hero: Vec<usize>,
    /// Seats you may make peace with.
    pub allies: Vec<usize>,
    /// Why the turn can't end yet (`None`: it can, or it isn't the time).
    pub end_turn: Option<String>,
    pub can_end_turn: bool,
}

impl State {
    /// Everything `seat` may do now (nothing when it isn't their move).
    pub fn moves(&self, seat: usize) -> Moves {
        let mut m = Moves::default();
        if self.actor() != Some(seat) {
            return m;
        }
        let regions = 0..self.regions.len();
        let ok = |action: Action| {
            let mut copy = self.clone();
            copy.apply(seat, action).is_ok()
        };
        match self.phase {
            Phase::Pick => {
                m.pick = (0..self.offers.len()).filter(|&i| self.players[seat].coins >= i as u32).collect();
            }
            Phase::Conquest { ghouls } => {
                m.decline = self.this_turn.may_decline && !self.this_turn.moved;
                m.end_conquests = true;
                if let Some((army, _)) = self.conqueror(seat) {
                    for r in regions.clone() {
                        if let Ok(cost) = self.conquest_cost(seat, r) {
                            if army.hand > 0 && cost <= army.hand {
                                m.conquer.push((r, cost));
                            } else if army.hand > 0
                                && (ghouls || army.power != Power::Berserk)
                                && cost - army.hand <= DIE_MAX
                            {
                                m.gamble.push((r, cost));
                            }
                        }
                        let mine = self.regions[r].race() == Some(army.race);
                        if mine && !self.this_turn.conquered.contains(&r) {
                            m.abandon.push(r);
                        }
                    }
                    if !ghouls {
                        m.roll = army.power == Power::Berserk && self.this_turn.berserk.is_none() && army.hand > 0;
                        if army.power == Power::DragonMaster && !self.this_turn.dragon && army.hand > 0 {
                            m.dragon = regions.clone().filter(|&r| self.target(seat, &army, false, r).is_ok()).collect();
                        }
                        if army.race == Race::Sorcerers {
                            m.enchant = regions.clone().filter(|&r| self.enchant_target(seat, &army, r).is_ok()).collect();
                        }
                    }
                }
            }
            Phase::Redeploy | Phase::Reinforce => {
                for r in regions {
                    if ok(Action::Put { region: r }) {
                        m.put.push(r);
                    }
                    if self.phase == Phase::Reinforce {
                        continue;
                    }
                    for (action, list) in [
                        (Action::Take { region: r }, &mut m.take),
                        (Action::Camp { region: r }, &mut m.camp),
                        (Action::Uncamp { region: r }, &mut m.uncamp),
                        (Action::Fortify { region: r }, &mut m.fortify),
                        (Action::Hero { region: r }, &mut m.hero),
                    ] {
                        if ok(action) {
                            list.push(r);
                        }
                    }
                }
                if self.phase == Phase::Redeploy {
                    if self.players[seat].active.is_some_and(|a| a.power == Power::Diplomat) {
                        m.allies = (0..self.seats())
                            .filter(|&s| s != seat && !self.this_turn.attacked.contains(&s))
                            .collect();
                    }
                    match self.check_end_turn(seat) {
                        Ok(()) => m.can_end_turn = true,
                        Err(e) => m.end_turn = Some(e.message().to_string()),
                    }
                    m.decline = m.can_end_turn && self.players[seat].active.is_some_and(|a| a.power == Power::Stout);
                }
            }
            Phase::Over => {}
        }
        m
    }
}
