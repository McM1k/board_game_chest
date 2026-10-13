//! The game state and its rules.
//!
//! A game is four rounds. Every round has three phases in which **all
//! players act at the same time**:
//!
//! 1. **Draft.** Everybody is dealt a hand (7 cards, 10 with two players;
//!    with Corruption & Ascension, cards from both decks, see [`hand_sizes`]),
//!    secretly picks one card, then passes the rest on: to the next seat in
//!    rounds 1 and 3, to the previous one in rounds 2 and 4. Once every pick
//!    is in, the new hands are looked at and the next pick starts, until
//!    each player holds 7 cards. Whatever is left over is discarded.
//! 2. **Planning.** Each drafted card is either put under construction or
//!    recycled: it is discarded and its recycling bonus, a cube, goes onto a
//!    building or onto the Empire straight away. A building whose spaces are
//!    all filled is finished at once, and pays its construction bonus
//!    (characters or Krystallium, which are kept until used).
//! 3. **Production.** Five steps, one per resource (Materials, Energy,
//!    Science, Gold, Exploration). In each, everybody produces as many cubes
//!    as they have icons for that resource, less the corrupted ones, the
//!    player who produced the most (and strictly more than everyone else;
//!    the two best from six players on, see [`supremacy`]) takes the
//!    matching character, and the cubes are placed on buildings or the
//!    Empire; cubes that are not placed are lost. A sixth, **wrap-up** step follows, in which
//!    Krystallium and characters that are still held can be placed.
//!
//! After the fourth round the victory points are counted.
//!
//! The Empires are dealt at random. The rules are pure: no networking, no
//! randomness except the seed that shuffles the decks and the Empires.

use crate::cards::{self, Bonus, CardId, Cost, Kind, Per, Res, Token, EMPIRES};
use serde::{Deserialize, Serialize};

pub const MIN_PLAYERS: usize = 2;
/// The most players a game seats, with Corruption & Ascension (see
/// [`Options::max_players`]).
pub const MAX_PLAYERS: usize = 7;
pub const ROUNDS: u8 = 4;
/// Cards each player ends the draft with.
pub const DRAFT_PICKS: usize = 7;
/// The production step after Exploration, for held Krystallium and characters.
pub const WRAP_UP: u8 = 5;
/// How many events [`State::log`] keeps.
const LOG_LEN: usize = 24;
/// Empire cubes that turn into one Krystallium.
const CUBES_PER_KRYSTALLIUM: u8 = 5;

/// What the host picked in the lobby.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Options {
    /// The Corruption & Ascension expansion: its deck of 56 cards, dealt
    /// beside the base one, and two more Empires, for up to 7 players.
    #[serde(default)]
    pub ascension: bool,
}

impl Options {
    /// The most players a game with these options seats.
    pub fn max_players(self) -> usize {
        if self.ascension {
            7
        } else {
            5
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Draft,
    Planning,
    /// `step` 0 to 4 are the resources in [`Res::ALL`] order, [`WRAP_UP`] the last one.
    Production { step: u8 },
    Over,
}

/// Where a cube goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    /// A building: a card under construction.
    Card(CardId),
    /// The Empire: five cubes there make a Krystallium.
    Empire,
}

/// Something a player holds and can place on a building.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Piece {
    /// A cube produced in the current step.
    Cube(Res),
    /// Krystallium: `Some(res)` standing in for a cube of that resource,
    /// `None` on a space that asks for Krystallium itself.
    Krystallium(Option<Res>),
    General,
    Financier,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Draft: take this card from the hand you hold.
    Draft { card: CardId },
    /// Planning: put a drafted card under construction.
    Build { card: CardId },
    /// Planning: recycle a drafted card; its cube goes to `to`.
    Recycle { card: CardId, to: Target },
    /// Planning or production: give up a building. What was placed on it is
    /// lost; its recycling cube goes to the Empire.
    Scrap { card: CardId },
    /// Planning or production: place something you hold.
    Place { piece: Piece, target: Target },
    /// Production, Science: pick the character the supremacy bonus gives.
    Choose { token: Token },
    /// Planning: no more to decide. Production: done with this step.
    Ready,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    GameOver,
    UnknownSeat,
    WrongPhase,
    AlreadyPicked,
    NotInHand,
    NotDrafted,
    NotUnderConstruction,
    InvalidTarget,
    NothingToPlace,
    CardsLeftToPlan,
    MustChoose,
    NothingToChoose,
}

impl Error {
    /// What the error means, in words for the player.
    pub fn message(self) -> &'static str {
        match self {
            Error::GameOver => "The game is over.",
            Error::UnknownSeat => "You aren't seated in this game.",
            Error::WrongPhase => "You can't do that right now.",
            Error::AlreadyPicked => "You already picked a card: wait for the others.",
            Error::NotInHand => "That card isn't in your hand.",
            Error::NotDrafted => "That card isn't one of your drafted cards.",
            Error::NotUnderConstruction => "That card isn't under construction.",
            Error::InvalidTarget => "It can't go there: that building has no free space for it.",
            Error::NothingToPlace => "You don't have that to place.",
            Error::CardsLeftToPlan => "Decide what to do with all your drafted cards first.",
            Error::MustChoose => "Choose your General or Financier first.",
            Error::NothingToChoose => "There's nothing to choose.",
        }
    }
}

/// Something that happened, kept (briefly) for the log. Seats are indices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    RoundStarted(u8),
    /// A building was finished.
    Completed { seat: usize, card: CardId },
    /// Who produced the most of `res` and takes its character (nobody when
    /// the best were tied, see [`supremacy`]).
    Supremacy { res: Res, seats: Vec<usize> },
    Chose { seat: usize, token: Token },
}

/// A card under construction, with what has been placed on it so far.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Building {
    pub card: CardId,
    pub filled: Cost,
}

impl Building {
    /// What is still missing.
    pub fn remaining(&self) -> Cost {
        self.card.def().cost.minus(&self.filled)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    /// The hand being drafted from (what you can pick from right now).
    pub hand: Vec<CardId>,
    /// The card picked in this draft step, not yet revealed to the others.
    pub picked: Option<CardId>,
    /// The draft area: cards kept, waiting for the planning phase.
    pub drafted: Vec<CardId>,
    pub buildings: Vec<Building>,
    /// Finished cards.
    pub empire: Vec<CardId>,
    /// Cubes on the Empire card, fewer than five.
    pub empire_cubes: u8,
    pub krystallium: u8,
    pub generals: u8,
    pub financiers: u8,
    /// Cubes produced in the current step and not placed yet.
    pub pool: u8,
    /// What was produced in the current step.
    pub produced: u8,
    /// Planning: done planning. Production: done with this step.
    pub ready: bool,
    /// Won the Science supremacy and has to pick a character.
    pub choose: bool,
}

/// How a player's points add up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Score {
    /// Points printed on cards, whatever else you own.
    pub gross: u32,
    /// Points that depend on card types or characters.
    pub combo: u32,
    /// One point per General held.
    pub generals: u32,
    /// One point per Financier held.
    pub financiers: u32,
    pub total: u32,
    /// Finished cards, the first tie-break.
    pub cards: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub players: Vec<PlayerState>,
    pub options: Options,
    /// The Empire each seat plays, as an index into [`EMPIRES`].
    pub empires: Vec<usize>,
    pub deck: Vec<CardId>,
    /// The Corruption & Ascension deck (empty without the expansion).
    pub ascension_deck: Vec<CardId>,
    pub discard: Vec<CardId>,
    /// 1 to [`ROUNDS`].
    pub round: u8,
    pub phase: Phase,
    /// The latest events, oldest first.
    pub log: Vec<Event>,
    /// Who took the character of each production step played so far this
    /// round, in production order (empty when nobody did: a tie, or nobody
    /// produced any). Kept for the round's production board.
    pub awards: Vec<Vec<usize>>,
    /// Filled in when the game is over: one entry per seat.
    pub scores: Vec<Score>,
}

/// The cards dealt to each hand at the start of a round: from the base deck,
/// and from the Corruption & Ascension deck.
pub fn hand_sizes(players: usize, options: Options) -> (usize, usize) {
    match (options.ascension, players) {
        (false, 2) => (10, 0),
        (false, _) => (DRAFT_PICKS, 0),
        (true, 2) => (8, 4),
        (true, 3 | 4) => (6, 3),
        (true, 5 | 6) => (6, 2),
        (true, _) => (5, 2),
    }
}

/// Who takes the character for producing the most of a resource: the seat
/// that produced strictly more than everyone else, or with six or more
/// players the two best. Two tied for first share both places; a tie for
/// second leaves the second place empty; three or more tied for first, or
/// nobody producing anything, gives it to nobody.
pub fn supremacy(made: &[u8]) -> Vec<usize> {
    let places = if made.len() >= 6 { 2 } else { 1 };
    let at = |score: u8| (0..made.len()).filter(|&i| made[i] == score).collect::<Vec<usize>>();
    let most = made.iter().copied().max().unwrap_or(0);
    if most == 0 {
        return Vec::new();
    }
    let mut winners = at(most);
    if winners.len() > places {
        return Vec::new();
    }
    if winners.len() < places {
        let next = made.iter().copied().filter(|&n| n < most).max().unwrap_or(0);
        let second = at(next);
        if next > 0 && second.len() <= places - winners.len() {
            winners.extend(second);
        }
    }
    winners
}

/// SplitMix64: a tiny seeded generator, so the rules need no `rand`.
struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

fn shuffle<T>(items: &mut [T], seed: u64) {
    let mut rng = SplitMix(seed);
    for i in (1..items.len()).rev() {
        let j = (rng.next() % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
}

impl State {
    /// A new base game for 2 to 5 players. The same seed gives the same deal.
    pub fn new(players: usize, seed: u64) -> State {
        State::with_options(players, seed, Options::default())
    }

    /// A new game with the lobby's options: 2 to 5 players, or 2 to 7 with
    /// Corruption & Ascension. The same seed gives the same deal.
    pub fn with_options(players: usize, seed: u64, options: Options) -> State {
        let max = options.max_players();
        assert!((MIN_PLAYERS..=max).contains(&players), "the game is for {MIN_PLAYERS} to {max} players");
        let mut deck = cards::deck(false);
        shuffle(&mut deck, seed);
        let mut ascension_deck = if options.ascension { cards::deck(true) } else { Vec::new() };
        shuffle(&mut ascension_deck, seed ^ 0xA5CE_0510_0000_0001);
        // The Empires in the box, dealt one per seat.
        let mut empires: Vec<usize> = (0..EMPIRES.len()).filter(|&e| options.ascension || !EMPIRES[e].ascension).collect();
        shuffle(&mut empires, seed ^ 0xE3B1_2E00_0000_0002);
        empires.truncate(players);
        let mut state = State {
            players: vec![PlayerState::default(); players],
            options,
            empires,
            deck,
            ascension_deck,
            discard: Vec::new(),
            round: 1,
            phase: Phase::Draft,
            log: Vec::new(),
            awards: Vec::new(),
            scores: Vec::new(),
        };
        state.start_round();
        state
    }

    pub fn seats(&self) -> usize {
        self.players.len()
    }

    /// Whether this round's hands go to the next seat (rounds 1 and 3) or
    /// the previous one (rounds 2 and 4).
    pub fn passes_to_next(&self) -> bool {
        self.round % 2 == 1
    }

    /// How many cubes of `res` the seat produces right now: the Empire's own
    /// icons plus those of every finished card, less the corrupted ones (but
    /// never below zero).
    pub fn production(&self, seat: usize, res: Res) -> u8 {
        let p = &self.players[seat];
        let mut total = EMPIRES[self.empires[seat]].base[res.index()] as i32;
        for id in &p.empire {
            let card = id.def();
            total += card.produces[res.index()] as i32 - card.corruption[res.index()] as i32;
            if let Some((r, kind)) = card.scaled {
                if r == res {
                    total += count_kind(&p.empire, kind) as i32;
                }
            }
        }
        total.clamp(0, u8::MAX as i32) as u8
    }

    /// The seat's points as things stand.
    pub fn score_of(&self, seat: usize) -> Score {
        let p = &self.players[seat];
        let held = |token: Token| match token {
            Token::General => p.generals as u32,
            Token::Financier => p.financiers as u32,
        };
        let count = |per: Per| match per {
            Per::Kind(kind) => count_kind(&p.empire, kind),
            Per::Token(token) => held(token),
        };
        let mut score = Score { cards: p.empire.len() as u32, ..Score::default() };
        let empire = &EMPIRES[self.empires[seat]];
        let combos = p.empire.iter().map(|id| (id.def().combo, id.def().per_token));
        for (combo, per_token) in combos.chain([(empire.combo, empire.per_token)]) {
            if let Some((kind, points)) = combo {
                score.combo += points as u32 * count_kind(&p.empire, kind);
            }
            if let Some((token, points)) = per_token {
                score.combo += points as u32 * held(token);
            }
        }
        // Duos: one of each of the two, as many times as both can be paired.
        for id in &p.empire {
            if let Some((a, b, points)) = id.def().duo {
                score.combo += points as u32 * count(a).min(count(b));
            }
        }
        for id in &p.empire {
            score.gross += id.def().vp as u32;
        }
        score.generals = p.generals as u32;
        score.financiers = p.financiers as u32;
        score.total = score.gross + score.combo + score.generals + score.financiers;
        score
    }

    /// The winners once the game is over: most points, then most finished
    /// cards, then most characters. Whoever is still level shares the win.
    pub fn winners(&self) -> Vec<usize> {
        if self.phase != Phase::Over || self.scores.is_empty() {
            return Vec::new();
        }
        let key = |seat: usize| {
            let s = &self.scores[seat];
            let p = &self.players[seat];
            (s.total, s.cards, p.generals as u32 + p.financiers as u32)
        };
        let best = (0..self.seats()).map(key).max().unwrap_or_default();
        (0..self.seats()).filter(|&s| key(s) == best).collect()
    }

    /// Applies one move by `seat`. Moves are checked and applied right away,
    /// whoever else is still deciding; when the last player of a step is
    /// done, the game moves on by itself.
    pub fn apply(&mut self, seat: usize, action: Action) -> Result<(), Error> {
        if seat >= self.seats() {
            return Err(Error::UnknownSeat);
        }
        if self.phase == Phase::Over {
            return Err(Error::GameOver);
        }
        match action {
            Action::Draft { card } => self.draft(seat, card)?,
            Action::Build { card } => self.build(seat, card)?,
            Action::Recycle { card, to } => self.recycle(seat, card, to)?,
            Action::Scrap { card } => self.scrap(seat, card)?,
            Action::Place { piece, target } => self.place(seat, piece, target)?,
            Action::Choose { token } => self.choose(seat, token)?,
            Action::Ready => self.ready(seat)?,
        }
        self.settle();
        Ok(())
    }

    // ----- draft -----------------------------------------------------

    fn start_round(&mut self) {
        let (base, ascension) = hand_sizes(self.seats(), self.options);
        self.phase = Phase::Draft;
        for p in &mut self.players {
            let at = self.deck.len() - base;
            p.hand = self.deck.split_off(at);
            let at = self.ascension_deck.len() - ascension;
            p.hand.append(&mut self.ascension_deck.split_off(at));
            p.picked = None;
            p.drafted.clear();
            p.pool = 0;
            p.produced = 0;
            p.ready = false;
            p.choose = false;
        }
        self.awards.clear();
        self.push_event(Event::RoundStarted(self.round));
    }

    fn draft(&mut self, seat: usize, card: CardId) -> Result<(), Error> {
        if self.phase != Phase::Draft {
            return Err(Error::WrongPhase);
        }
        let p = &mut self.players[seat];
        if p.picked.is_some() {
            return Err(Error::AlreadyPicked);
        }
        let at = p.hand.iter().position(|&c| c == card).ok_or(Error::NotInHand)?;
        p.hand.remove(at);
        p.picked = Some(card);
        Ok(())
    }

    /// Everybody has picked: reveal the picks and pass the hands on.
    fn finish_draft_step(&mut self) {
        let n = self.seats();
        for p in &mut self.players {
            if let Some(card) = p.picked.take() {
                p.drafted.push(card);
            }
        }
        let next = self.passes_to_next();
        let mut passed: Vec<Vec<CardId>> = vec![Vec::new(); n];
        for (i, p) in self.players.iter_mut().enumerate() {
            let to = if next { (i + 1) % n } else { (i + n - 1) % n };
            passed[to] = std::mem::take(&mut p.hand);
        }
        for (p, hand) in self.players.iter_mut().zip(passed) {
            p.hand = hand;
        }
        if self.players.iter().all(|p| p.drafted.len() >= DRAFT_PICKS) {
            // Whatever is left (3 cards each with two players, more with
            // Corruption & Ascension) is discarded
            // without its recycling bonus.
            for p in &mut self.players {
                self.discard.append(&mut p.hand);
                p.ready = false;
            }
            self.phase = Phase::Planning;
        }
    }

    // ----- planning and construction ---------------------------------

    fn build(&mut self, seat: usize, card: CardId) -> Result<(), Error> {
        if self.phase != Phase::Planning {
            return Err(Error::WrongPhase);
        }
        let p = &mut self.players[seat];
        let at = p.drafted.iter().position(|&c| c == card).ok_or(Error::NotDrafted)?;
        p.drafted.remove(at);
        p.buildings.push(Building { card, filled: Cost::default() });
        Ok(())
    }

    fn recycle(&mut self, seat: usize, card: CardId, to: Target) -> Result<(), Error> {
        if self.phase != Phase::Planning {
            return Err(Error::WrongPhase);
        }
        let at = self.players[seat]
            .drafted
            .iter()
            .position(|&c| c == card)
            .ok_or(Error::NotDrafted)?;
        let res = card.def().recycle;
        self.check_cube_target(seat, res, to)?;
        self.players[seat].drafted.remove(at);
        self.discard.push(card);
        self.place_cube(seat, res, to);
        Ok(())
    }

    fn scrap(&mut self, seat: usize, card: CardId) -> Result<(), Error> {
        if !matches!(self.phase, Phase::Planning | Phase::Production { .. }) {
            return Err(Error::WrongPhase);
        }
        let at = self.players[seat]
            .buildings
            .iter()
            .position(|b| b.card == card)
            .ok_or(Error::NotUnderConstruction)?;
        self.players[seat].buildings.remove(at);
        self.discard.push(card);
        self.add_empire_cube(seat);
        Ok(())
    }

    /// Whether a cube of `res` can go to `to`.
    fn check_cube_target(&self, seat: usize, res: Res, to: Target) -> Result<(), Error> {
        match to {
            Target::Empire => Ok(()),
            Target::Card(id) => {
                let b = self.players[seat]
                    .buildings
                    .iter()
                    .find(|b| b.card == id)
                    .ok_or(Error::InvalidTarget)?;
                if b.remaining().res[res.index()] > 0 {
                    Ok(())
                } else {
                    Err(Error::InvalidTarget)
                }
            }
        }
    }

    /// Puts a cube where `check_cube_target` said it can go.
    fn place_cube(&mut self, seat: usize, res: Res, to: Target) {
        match to {
            Target::Empire => self.add_empire_cube(seat),
            Target::Card(id) => {
                if let Some(b) = self.players[seat].buildings.iter_mut().find(|b| b.card == id) {
                    b.filled.res[res.index()] += 1;
                }
                self.complete_if_done(seat, id);
            }
        }
    }

    fn add_empire_cube(&mut self, seat: usize) {
        let p = &mut self.players[seat];
        p.empire_cubes += 1;
        if p.empire_cubes >= CUBES_PER_KRYSTALLIUM {
            p.empire_cubes -= CUBES_PER_KRYSTALLIUM;
            p.krystallium += 1;
        }
    }

    /// Finishes the building if every space is filled: the card joins the
    /// Empire and its construction bonus is paid out.
    fn complete_if_done(&mut self, seat: usize, id: CardId) {
        let p = &mut self.players[seat];
        let Some(at) = p.buildings.iter().position(|b| b.card == id) else {
            return;
        };
        if !p.buildings[at].remaining().is_zero() {
            return;
        }
        p.buildings.remove(at);
        p.empire.push(id);
        for bonus in id.def().bonus {
            match bonus {
                Bonus::Krystallium => p.krystallium += 1,
                Bonus::Token(Token::General) => p.generals += 1,
                Bonus::Token(Token::Financier) => p.financiers += 1,
            }
        }
        self.push_event(Event::Completed { seat, card: id });
    }

    fn place(&mut self, seat: usize, piece: Piece, target: Target) -> Result<(), Error> {
        match self.phase {
            Phase::Planning | Phase::Production { .. } => {}
            _ => return Err(Error::WrongPhase),
        }
        match piece {
            Piece::Cube(res) => {
                // Only the cubes produced in this step are held.
                let in_pool = matches!(self.phase, Phase::Production { step }
                    if (step as usize) < Res::ALL.len() && Res::ALL[step as usize] == res)
                    && self.players[seat].pool > 0;
                if !in_pool {
                    return Err(Error::NothingToPlace);
                }
                self.check_cube_target(seat, res, target)?;
                let p = &mut self.players[seat];
                p.pool -= 1;
                // Everything placed: nothing left to wait for in this step.
                if p.pool == 0 && !p.choose {
                    p.ready = true;
                }
                self.place_cube(seat, res, target);
                Ok(())
            }
            Piece::Krystallium(space) => {
                let id = match space {
                    Some(res) => self.building_with_room(seat, target, |c| c.res[res.index()] > 0)?,
                    None => self.building_with_room(seat, target, |c| c.krystallium > 0)?,
                };
                let p = &mut self.players[seat];
                if p.krystallium == 0 {
                    return Err(Error::NothingToPlace);
                }
                p.krystallium -= 1;
                match space {
                    Some(res) => self.fill(seat, id, |f| f.res[res.index()] += 1),
                    None => self.fill(seat, id, |f| f.krystallium += 1),
                }
                Ok(())
            }
            Piece::General => {
                let id = self.building_with_room(seat, target, |c| c.generals > 0)?;
                let p = &mut self.players[seat];
                if p.generals == 0 {
                    return Err(Error::NothingToPlace);
                }
                p.generals -= 1;
                self.fill(seat, id, |f| f.generals += 1);
                Ok(())
            }
            Piece::Financier => {
                let id = self.building_with_room(seat, target, |c| c.financiers > 0)?;
                let p = &mut self.players[seat];
                if p.financiers == 0 {
                    return Err(Error::NothingToPlace);
                }
                p.financiers -= 1;
                self.fill(seat, id, |f| f.financiers += 1);
                Ok(())
            }
        }
    }

    /// The building `target` names, if it still has a space `wanted` accepts.
    fn building_with_room(&self, seat: usize, target: Target, wanted: impl Fn(&Cost) -> bool) -> Result<CardId, Error> {
        let Target::Card(id) = target else {
            return Err(Error::InvalidTarget);
        };
        let b = self.players[seat]
            .buildings
            .iter()
            .find(|b| b.card == id)
            .ok_or(Error::InvalidTarget)?;
        if wanted(&b.remaining()) {
            Ok(id)
        } else {
            Err(Error::InvalidTarget)
        }
    }

    fn fill(&mut self, seat: usize, id: CardId, add: impl FnOnce(&mut Cost)) {
        if let Some(b) = self.players[seat].buildings.iter_mut().find(|b| b.card == id) {
            add(&mut b.filled);
        }
        self.complete_if_done(seat, id);
    }

    // ----- production --------------------------------------------------

    fn choose(&mut self, seat: usize, token: Token) -> Result<(), Error> {
        if !matches!(self.phase, Phase::Production { .. }) {
            return Err(Error::WrongPhase);
        }
        let p = &mut self.players[seat];
        if !p.choose {
            return Err(Error::NothingToChoose);
        }
        p.choose = false;
        match token {
            Token::General => p.generals += 1,
            Token::Financier => p.financiers += 1,
        }
        if p.pool == 0 {
            p.ready = true;
        }
        self.push_event(Event::Chose { seat, token });
        Ok(())
    }

    fn ready(&mut self, seat: usize) -> Result<(), Error> {
        match self.phase {
            Phase::Planning => {
                if !self.players[seat].drafted.is_empty() {
                    return Err(Error::CardsLeftToPlan);
                }
                self.players[seat].ready = true;
                Ok(())
            }
            Phase::Production { .. } => {
                if self.players[seat].choose {
                    return Err(Error::MustChoose);
                }
                self.players[seat].ready = true;
                Ok(())
            }
            _ => Err(Error::WrongPhase),
        }
    }

    /// Moves the game on for as long as every player is done with a step.
    fn settle(&mut self) {
        loop {
            let everyone_done = self.players.iter().all(|p| p.ready);
            let everyone_picked = self.players.iter().all(|p| p.picked.is_some());
            match self.phase {
                Phase::Draft if everyone_picked => self.finish_draft_step(),
                Phase::Planning if everyone_done => self.start_step(0),
                Phase::Production { step } if everyone_done => {
                    if step < WRAP_UP {
                        self.start_step(step + 1);
                    } else {
                        self.end_round();
                    }
                }
                _ => break,
            }
        }
    }

    fn start_step(&mut self, step: u8) {
        self.phase = Phase::Production { step };
        let n = self.seats();
        if (step as usize) < Res::ALL.len() {
            let res = Res::ALL[step as usize];
            let made: Vec<u8> = (0..n).map(|i| self.production(i, res)).collect();
            for (p, &amount) in self.players.iter_mut().zip(&made) {
                p.produced = amount;
                p.pool = amount;
                p.choose = false;
            }
            let anyone = made.iter().any(|&m| m > 0);
            let winners = if anyone { supremacy(&made) } else { Vec::new() };
            if anyone {
                // For Science, each winner chooses their own character.
                for &w in &winners {
                    match res.supremacy_token() {
                        Some(Token::General) => self.players[w].generals += 1,
                        Some(Token::Financier) => self.players[w].financiers += 1,
                        None => self.players[w].choose = true,
                    }
                }
                self.push_event(Event::Supremacy { res, seats: winners.clone() });
            }
            self.awards.push(winners);
            for p in &mut self.players {
                p.ready = p.pool == 0 && !p.choose;
            }
        } else {
            for i in 0..n {
                let can_act = self.can_wrap_up(i);
                let p = &mut self.players[i];
                p.pool = 0;
                p.produced = 0;
                p.choose = false;
                p.ready = !can_act;
            }
        }
    }

    /// Whether the seat holds Krystallium or a character that fits a space
    /// of one of its buildings.
    fn can_wrap_up(&self, seat: usize) -> bool {
        let p = &self.players[seat];
        p.buildings.iter().any(|b| {
            let left = b.remaining();
            (p.krystallium > 0 && (left.krystallium > 0 || left.res.iter().any(|&n| n > 0)))
                || (p.generals > 0 && left.generals > 0)
                || (p.financiers > 0 && left.financiers > 0)
        })
    }

    fn end_round(&mut self) {
        for p in &mut self.players {
            p.pool = 0;
            p.produced = 0;
            p.ready = false;
        }
        if self.round >= ROUNDS {
            self.scores = (0..self.seats()).map(|s| self.score_of(s)).collect();
            self.phase = Phase::Over;
        } else {
            self.round += 1;
            self.start_round();
        }
    }

    fn push_event(&mut self, event: Event) {
        self.log.push(event);
        if self.log.len() > LOG_LEN {
            let extra = self.log.len() - LOG_LEN;
            self.log.drain(..extra);
        }
    }
}

fn count_kind(cards: &[CardId], kind: Kind) -> u32 {
    cards.iter().filter(|id| id.def().kind == kind).count() as u32
}
