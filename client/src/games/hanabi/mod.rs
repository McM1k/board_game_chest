//! Hanabi: the board (`board.rs`), the screens before the game and the page
//! frame with its rules picker (`screens.rs`), and the pure helpers behind
//! them (`look.rs`). Styles in `hanabi.css`, scoped under `.hanabi`.
//!
//! Ported from the stand-alone Hanabi client (McM1k/hanabii, Leptos 0.6).
//! The rooms are the shared ones, so the old name-and-code join screen
//! became the same create/join flow every game uses, and the host starts
//! the game.

pub mod board;
pub mod look;
pub mod screens;

use crate::app::{App, Lobby};
use hanabi_core::{GameRules, PlayerView};
use leptos::prelude::*;
use protocol::{GameKind, GameOptions, GameView};

/// The Hanabi state shared by its screens. The board is built once per game
/// and only these signals change, which is what lets hands slide to their
/// new spot, drags survive and highlights time out undisturbed.
#[derive(Clone, Copy)]
pub struct Signals {
    /// What the server last sent for the running game, seen from our seat.
    pub view: RwSignal<Option<PlayerView>>,
    /// Seat names, indexed by `PlayerId`.
    pub names: RwSignal<Vec<String>>,
    /// Who is currently connected, indexed by `PlayerId`.
    pub connected: RwSignal<Vec<bool>>,
    /// The variant rules picked in the lobby (the host changes them, everyone
    /// sees them).
    pub rules: RwSignal<GameRules>,
}

impl Signals {
    pub fn new() -> Signals {
        Signals {
            view: RwSignal::new(None),
            names: RwSignal::new(Vec::new()),
            connected: RwSignal::new(Vec::new()),
            rules: RwSignal::new(GameRules::default()),
        }
    }

    /// Forget the game, e.g. when leaving the room.
    pub fn reset(&self) {
        self.view.set(None);
        self.names.set(Vec::new());
        self.connected.set(Vec::new());
        self.rules.set(GameRules::default());
    }

    /// Whether the "hanabii" mode is on: what the lobby has picked until the
    /// game starts, then what the game is actually being played with.
    pub fn hanabii(&self) -> bool {
        self.view.with(|view| match view {
            Some(view) => view.rules.hanabii,
            None => self.rules.with(|rules| rules.hanabii),
        })
    }
}

impl Default for Signals {
    fn default() -> Self {
        Signals::new()
    }
}

pub struct Hanabi;

impl crate::games::Game for Hanabi {
    fn kind(&self) -> GameKind {
        GameKind::Hanabi
    }

    fn icon(&self) -> AnyView {
        screens::icon().into_any()
    }

    fn blurb(&self) -> &'static str {
        "A cooperative card game: build the fireworks in order, but you can't see your own hand."
    }

    /// "Hanabii" in that mode.
    fn tab_title(&self, app: App) -> &'static str {
        screens::tab_title(app)
    }

    fn frame(&self, app: App, content: AnyView) -> AnyView {
        screens::shell(app, content).into_any()
    }

    fn home(&self, app: App) -> AnyView {
        screens::home_screen(app).into_any()
    }

    fn lobby(&self, app: App, lobby: &Lobby) -> AnyView {
        screens::lobby_screen(app, lobby.room.clone(), lobby.players.clone(), lobby.you, lobby.is_host).into_any()
    }

    fn options(&self, app: App, options: GameOptions) {
        let GameOptions::Hanabi(rules) = options;
        app.games.hanabi.rules.set(rules);
    }

    fn board(&self, app: App) -> AnyView {
        board::board(app).into_any()
    }

    fn receive(&self, app: App, view: GameView, names: Vec<String>, connected: Vec<bool>) {
        let GameView::Hanabi(view) = view else { return };
        let hanabi = app.games.hanabi;
        hanabi.names.set(names);
        hanabi.connected.set(connected);
        hanabi.view.set(Some(*view));
    }

    fn reset(&self, app: App) {
        app.games.hanabi.reset();
    }
}
