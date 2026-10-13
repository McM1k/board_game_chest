//! What one seat sees. Barrage hides almost nothing: only the order of the
//! face-down piles (Contracts and Advanced Technology tiles), which the view
//! gives as counts. It also lists what the seat can do now, for the
//! interface.

use crate::data::*;
use crate::map::*;
use crate::state::*;
use serde::{Deserialize, Serialize};

/// A production that can be made now: up to `max` drops from `dam`
/// through `conduit` (worth `value` a drop) to `powerhouse`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductionOption {
    pub powerhouse: usize,
    pub conduit: usize,
    pub dam: usize,
    pub max: u8,
    pub value: u8,
    /// The Conduit is somebody else's: 1 Credit a drop to them.
    pub paid: bool,
}

impl ProductionOption {
    /// The most drops it can send with `credits` left once the action space
    /// is paid for (somebody else's Conduit costs a Credit a drop).
    pub fn most(&self, credits: u32) -> u8 {
        if self.paid {
            self.max.min(credits.min(255) as u8)
        } else {
            self.max
        }
    }
}

/// What the seat can do right now (all empty when it isn't their move).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Moves {
    /// Setup: picking a pair and a Starting Contract.
    pub choose: bool,
    /// Management spaces that can be taken now.
    pub spots: Vec<Spot>,
    /// The most Engineers the Bank can take (0: not now).
    pub bank: u8,
    pub builds: Vec<BuildOption>,
    /// For the Turbine Station, or a production waiting in the todo list.
    pub productions: Vec<ProductionOption>,
    /// Mahiri Sekibo: the officers she can copy now.
    pub copy: Vec<Officer>,
    pub pass: bool,
    /// Machineries of your choice to take.
    pub machinery: Option<u8>,
    /// Drops to place, and whether they flow at once.
    pub water: Option<(u8, bool)>,
    /// A free building's possible targets.
    pub places: Vec<Target>,
    /// Contracts the energy just produced can fulfill.
    pub fulfill: Vec<u8>,
    /// A Contract has to be discarded.
    pub discard: bool,
    /// The waiting decision can be left undone.
    pub skip: bool,
    /// What a production adds: the Company board's bonus (the Turbine
    /// row's comes on top), or for a production in the todo list, all of it.
    pub bonus: i32,
    /// Viktor Fiesler: at least 4 before bonuses.
    pub fiesler: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct View {
    pub you: usize,
    pub phase: Phase,
    pub round: u8,
    /// Seats in turn order.
    pub order: Vec<usize>,
    /// Whose turn it is in the actions.
    pub current: Option<usize>,
    /// Who has to act now.
    pub actor: Option<usize>,
    pub players: Vec<Player>,
    pub setup: Option<Setup>,
    pub dams: Vec<Option<Dam>>,
    pub conduits: Vec<Option<usize>>,
    pub powerhouses: Vec<Option<usize>>,
    pub headstreams: [u8; HEADSTREAMS],
    pub headstream_tiles: [usize; HEADSTREAMS],
    pub offers: [[Option<u8>; 2]; 3],
    /// Tiles left in the green, yellow and red piles.
    pub piles: [usize; 3],
    pub nationals: Vec<u8>,
    pub patents: [Option<u8>; 3],
    /// Tiles left in the I, II and III piles.
    pub tech: [usize; 3],
    pub bonus: Vec<Bonus>,
    pub objective: Objective,
    /// Each seat's count on the Objective tile so far.
    pub objective_counts: Vec<u32>,
    pub placements: Vec<Placement>,
    /// The decision waiting, and whose it is.
    pub todo: Option<Todo>,
    pub moves: Moves,
    pub log: Vec<Event>,
    pub winners: Vec<usize>,
}

impl State {
    pub fn view_for(&self, seat: usize) -> View {
        View {
            you: seat,
            phase: self.phase,
            round: self.round,
            order: self.order.clone(),
            current: (self.phase == Phase::Actions).then(|| self.current()),
            actor: self.actor(),
            players: self.players.clone(),
            setup: self.setup.clone().filter(|_| self.phase == Phase::Setup),
            dams: self.dams.clone(),
            conduits: self.conduits.clone(),
            powerhouses: self.powerhouses.clone(),
            headstreams: self.headstreams,
            headstream_tiles: self.headstream_tiles,
            offers: self.offers,
            piles: [self.piles[0].len(), self.piles[1].len(), self.piles[2].len()],
            nationals: self.nationals.clone(),
            patents: self.patents,
            tech: [self.tech[0].len(), self.tech[1].len(), self.tech[2].len()],
            bonus: self.bonus.clone(),
            objective: self.objective,
            objective_counts: (0..self.seats()).map(|s| self.objective_count(s)).collect(),
            placements: self.placements.clone(),
            todo: self.todo.first().copied(),
            moves: self.moves(seat),
            log: self.log.clone(),
            winners: self.winners(),
        }
    }

    fn production_options(&self, seat: usize, not: Option<usize>) -> Vec<ProductionOption> {
        self.productions(seat, not)
            .into_iter()
            .map(|(powerhouse, conduit, dam, max)| ProductionOption {
                powerhouse,
                conduit,
                dam,
                max,
                value: CONDUITS[conduit].value,
                paid: self.conduits[conduit] != Some(seat),
            })
            .collect()
    }

    /// Everything `seat` may do now.
    pub fn moves(&self, seat: usize) -> Moves {
        let mut m = Moves::default();
        if self.actor() != Some(seat) {
            return m;
        }
        let p = &self.players[seat];
        m.fiesler = p.has(Officer::Fiesler);
        if let Some(todo) = self.todo.first() {
            match todo.task {
                Task::Reward(r) => match r {
                    Reward::Machinery(n) => m.machinery = Some(n),
                    Reward::Water(n) => m.water = Some((n, false)),
                    Reward::WaterNow(n) => m.water = Some((n, true)),
                    _ => m.places = self.free_targets(seat, r),
                },
                Task::Contract { energy } => m.fulfill = self.fulfillable(seat, energy),
                Task::Discard => m.discard = true,
                Task::Produce { bonus, plain, not } => {
                    m.productions = self.production_options(seat, not);
                    m.bonus = if plain { 0 } else { bonus + self.company_bonus(seat) };
                }
            }
            m.skip = match todo.task {
                Task::Reward(r) => !matches!(r, Reward::Machinery(_)),
                Task::Contract { .. } | Task::Produce { .. } => true,
                Task::Discard => false,
            };
            return m;
        }
        match self.phase {
            Phase::Setup => m.choose = true,
            Phase::Actions => {
                m.builds = self.build_options(seat);
                if p.engineers == 0 {
                    m.pass = true;
                    return m;
                }
                m.bank = p.engineers;
                m.productions = self.production_options(seat, None);
                m.bonus = self.company_bonus(seat);
                // A Turbine space is open if a production makes energy there,
                // once the space's fee is paid.
                let produces = |row: usize, fee: u32| {
                    let credits = p.credits.saturating_sub(fee);
                    m.productions.iter().any(|o| {
                        let drops = o.most(credits);
                        drops > 0 && production_energy(drops, o.value, m.fiesler, m.bonus + TURBINE_BONUS[row]) >= 1
                    })
                };
                for section in [Section::Turbine, Section::Water, Section::Workshop, Section::Shop, Section::Contracts, Section::Patent]
                {
                    for (r, row) in rows(section).iter().enumerate() {
                        for right in [false, true] {
                            if right && row.right.is_none() {
                                continue;
                            }
                            let spot = Spot::new(section, r as u8, right);
                            if !self.spot_open(seat, spot) {
                                continue;
                            }
                            let possible = match section {
                                Section::Turbine => {
                                    let fee = if spot.def().is_some_and(|(d, _)| d.fee) { FEE } else { 0 };
                                    produces(r, fee)
                                }
                                Section::Contracts => self.offers.iter().flatten().any(Option::is_some),
                                Section::Patent => self.patents[r].is_some(),
                                _ => true,
                            };
                            if possible {
                                m.spots.push(spot);
                            }
                        }
                    }
                }
                if p.officer == Some(Officer::Sekibo) && p.copied.is_none() && p.sekibo < 2 {
                    let fee = if p.sekibo == 1 { FEE } else { 0 };
                    if p.credits >= fee {
                        m.copy = Officer::ALL.into_iter().filter(|&o| o != Officer::Sekibo).collect();
                    }
                }
            }
            _ => {}
        }
        m
    }
}
