//! Rules for *Small World*: pure game logic, no networking and no UI.
//!
//! Fantasy races, each paired with a special power, conquer the regions of
//! a board too small for all of them, score a coin a turn for every region
//! they hold, and go into decline when they run out of steam, so that their
//! player can take up a new one. See [`state`] for the rules in full, and
//! [`races`] for the 14 races and 20 powers.
//!
//! The four boards (one per number of players, 2 to 5) are in [`maps`].

pub mod maps;
pub mod races;
pub mod state;
pub mod view;

pub use maps::*;
pub use races::*;
pub use state::*;
pub use view::*;

#[cfg(test)]
mod tests;
