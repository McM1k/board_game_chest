//! Every game the client can show, behind one interface. The shell (the
//! menu, the connection, the routing between screens) knows no game by
//! name: it asks a game for its menu card, its screens and its page title,
//! and hands it what the server sends.
//!
//! Each game is a folder here: its screens, the pure helpers behind them
//! (with native tests) and its stylesheet, scoped under the class of its
//! frame so the games can't restyle each other. Adding a game: a folder
//! implementing [`Game`], its arm in [`game`], its signals in [`Games`] and
//! its stylesheet in `index.html` (see `docs/adding-a-game.md`).

pub mod catan;
pub mod hanabi;
pub mod smallworld;
pub mod wonderful;

use crate::app::{App, Lobby};
use leptos::prelude::*;
use protocol::{GameKind, GameOptions, GameView};

/// One game, as the shell sees it. One value per game, without any state:
/// what a game keeps between messages lives in its field of [`Games`].
pub trait Game: Sync {
    /// Which game this is.
    fn kind(&self) -> GameKind;

    /// The picture on the game's menu card.
    fn icon(&self) -> AnyView;

    /// One sentence about the game, on its menu card.
    fn blurb(&self) -> &'static str;

    /// The browser tab's title while the game is on screen.
    fn tab_title(&self, app: App) -> &'static str {
        let _ = app;
        self.kind().title()
    }

    /// Puts one of the game's screens in its frame: the element its
    /// stylesheet is scoped to.
    fn frame(&self, app: App, content: AnyView) -> AnyView;

    /// Your name, and creating or joining a room.
    fn home(&self, app: App) -> AnyView;

    /// The waiting room.
    fn lobby(&self, app: App, lobby: &Lobby) -> AnyView;

    /// The lobby's options, as the server echoed them (only for a game that
    /// has options).
    fn options(&self, app: App, options: GameOptions) {
        let _ = (app, options);
    }

    /// The running game, drawn from what [`Game::receive`] kept.
    fn board(&self, app: App) -> AnyView;

    /// The server sent the running game as our seat sees it (always a view
    /// of this game), with the seats' names and who is connected.
    fn receive(&self, app: App, view: GameView, names: Vec<String>, connected: Vec<bool>);

    /// Forgets the game: on leaving the room, or arriving in a new one.
    fn reset(&self, app: App);
}

/// The game `kind` names.
pub fn game(kind: GameKind) -> &'static dyn Game {
    match kind {
        GameKind::Catan => &catan::Catan,
        GameKind::Hanabi => &hanabi::Hanabi,
        GameKind::Wonderful => &wonderful::Wonderful,
        GameKind::SmallWorld => &smallworld::SmallWorld,
    }
}

/// What each game keeps beyond a single screen: what the server last sent,
/// and the interface state that has to survive a redraw.
#[derive(Clone, Copy)]
pub struct Games {
    pub catan: catan::Signals,
    pub hanabi: hanabi::Signals,
    pub wonderful: wonderful::Signals,
    pub smallworld: smallworld::Signals,
}

impl Games {
    pub fn new() -> Games {
        Games {
            catan: catan::Signals::new(),
            hanabi: hanabi::Signals::new(),
            wonderful: wonderful::Signals::new(),
            smallworld: smallworld::Signals::new(),
        }
    }
}

impl Default for Games {
    fn default() -> Self {
        Games::new()
    }
}
