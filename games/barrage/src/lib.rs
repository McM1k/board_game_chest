//! Rules for *Barrage* (Cranio Creations, 2019): pure game logic, no
//! networking and no UI.
//!
//! Companies build dams, conduits and powerhouses in the Alps, hold back the
//! water that runs down from the headstreams and turn it into energy to
//! fulfill contracts, over five rounds of worker placement. See [`state`]
//! for the rules in full, [`map`] for the twelve basins and how they are
//! connected, and [`data`] for the printed components.

pub mod data;
pub mod map;
pub mod state;
pub mod view;

pub use data::*;
pub use map::*;
pub use state::*;
pub use view::*;

#[cfg(test)]
mod tests;
