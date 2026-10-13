//! Every game the server hosts, behind one interface. The rooms run the
//! lobbies, the seats and the broadcasts without knowing any game by name;
//! a game says how it starts ([`Game`]) and, once running, takes moves and
//! shows each seat its own view ([`Running`]).
//!
//! Adding a game: a module here implementing both traits, and its arm in
//! [`game`] (see `docs/adding-a-game.md`).

pub mod barrage;
pub mod catan;
pub mod hanabi;
pub mod smallworld;
pub mod wonderful;

use protocol::{GameKind, GameMove, GameOptions, GameView};
use rand::RngCore;
use std::any::Any;

/// What a player is told when a move meant for another game reaches a room.
pub const WRONG_GAME: &str = "That move belongs to a different game.";

/// How a game is set up. One value per game, without any state.
pub trait Game: Sync {
    /// Whether the seats are shuffled when the game starts. If not, the
    /// join order is the seat order and the host sits first.
    fn shuffles_seats(&self) -> bool;

    /// The options a new lobby starts with; `None` for a game without any.
    fn default_options(&self) -> Option<GameOptions> {
        None
    }

    /// Checks options the host picked (they are always this game's) and
    /// returns what will actually be played, which every seat is shown.
    fn check_options(&self, options: GameOptions) -> Result<GameOptions, String> {
        Ok(options)
    }

    /// A new game for these players, listed in seat order.
    fn start(&self, names: Vec<String>, options: Option<&GameOptions>, rng: &mut dyn RngCore) -> Box<dyn Running>;
}

/// A game in progress: the authoritative state, which only the room holds.
pub trait Running: Any + Send {
    /// Applies `seat`'s move. `Err` is what that player is told.
    fn play(&mut self, seat: usize, mv: GameMove) -> Result<(), String>;

    /// The game as `seat` may see it.
    fn view(&self, seat: usize) -> GameView;
}

/// The game `kind` names.
pub fn game(kind: GameKind) -> &'static dyn Game {
    match kind {
        GameKind::Catan => &catan::Catan,
        GameKind::Hanabi => &hanabi::Hanabi,
        GameKind::Wonderful => &wonderful::Wonderful,
        GameKind::SmallWorld => &smallworld::SmallWorld,
        GameKind::Barrage => &barrage::Barrage,
    }
}
