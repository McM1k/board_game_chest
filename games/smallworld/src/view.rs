//! What one seat is allowed to see: everything on the table, but only your
//! own Victory coins until the game is over (the others keep theirs
//! face down). The view has no maps, so it survives JSON.

use crate::races::Race;
use crate::state::{Army, Event, Moves, Offer, Phase, RegionState, State, Turn};
use serde::{Deserialize, Serialize};

/// What everybody can see about one player.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerView {
    /// Your own coins; the others' once the game is over.
    pub coins: Option<u32>,
    pub active: Option<Army>,
    pub declined: Vec<Army>,
    /// Tokens on the map, every race together.
    pub on_map: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct View {
    pub you: usize,
    /// Which board: the number of players it is for.
    pub board: usize,
    pub turn: u8,
    pub turns: u8,
    /// Whose turn it is.
    pub current: usize,
    /// Who has to act now.
    pub actor: Option<usize>,
    pub phase: Phase,
    pub regions: Vec<RegionState>,
    pub players: Vec<PlayerView>,
    /// The column, top (free) first.
    pub offers: Vec<Offer>,
    pub races_left: usize,
    pub powers_left: usize,
    pub this_turn: Turn,
    /// What you can do now.
    pub moves: Moves,
    /// On your turn: the coins you would score if it ended now.
    pub coins_now: Option<u32>,
    /// Tokens of each race still in the box, for the ones in play.
    pub supply: Vec<(Race, u8)>,
    pub log: Vec<Event>,
    pub winners: Vec<usize>,
}

impl State {
    pub fn view_for(&self, seat: usize) -> View {
        let over = self.phase == Phase::Over;
        let players = self
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| PlayerView {
                coins: (i == seat || over).then_some(p.coins),
                active: p.active,
                declined: p.declined.clone(),
                on_map: self.tokens_on_map(i),
            })
            .collect();
        let supply = self
            .players
            .iter()
            .flat_map(|p| p.active.iter().chain(&p.declined))
            .map(|a| (a.race, self.available(a.race)))
            .collect();
        View {
            you: seat,
            board: self.seats(),
            turn: self.turn,
            turns: self.map().turns,
            current: self.current,
            actor: self.actor(),
            phase: self.phase,
            regions: self.regions.clone(),
            players,
            offers: self.offers.clone(),
            races_left: self.races.len(),
            powers_left: self.powers.len(),
            this_turn: self.this_turn.clone(),
            moves: self.moves(seat),
            coins_now: (self.current == seat && matches!(self.phase, Phase::Conquest { .. } | Phase::Redeploy))
                .then(|| self.turn_coins(seat)),
            supply,
            log: self.log.clone(),
            winners: self.winners(),
        }
    }
}
