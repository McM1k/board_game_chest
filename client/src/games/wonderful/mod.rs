//! It's a Wonderful World: the board (`board.rs`), the screens before the
//! game and the page frame (`screens.rs`), and what a click means
//! (`look.rs`). Styles in `wonderful.css`, scoped under `.wonderful`.
//!
//! The game screen is redrawn from the state the server last sent (kept in
//! [`Signals`]) on every message, as Catan's is. What only the browser
//! knows (the piece picked in the tray, a cube waiting for its place) lives
//! in [`Ui`], which outlives those redraws.

pub mod board;
pub mod look;
pub mod screens;

use crate::app::{App, Lobby};
use leptos::prelude::*;
use protocol::{GameKind, GameOptions, GameView};
use wonderful_core::{Options, View};

/// Interface state that survives the screen being rebuilt.
#[derive(Clone, Copy)]
pub struct Ui {
    /// The piece picked in the tray, if any (see `look::effective_held`).
    pub held: RwSignal<Option<look::Held>>,
    /// A recycled card or a scrapped building whose cube is waiting to be
    /// given a place. Shown as a dialog until a place is picked or it's
    /// cancelled.
    pub cube_from: RwSignal<Option<look::CubeFrom>>,
}

impl Ui {
    pub fn new() -> Ui {
        Ui { held: RwSignal::new(None), cube_from: RwSignal::new(None) }
    }

    /// Forget everything, e.g. when leaving the room.
    pub fn reset(&self) {
        self.held.set(None);
        self.cube_from.set(None);
    }
}

impl Default for Ui {
    fn default() -> Self {
        Ui::new()
    }
}

/// What It's a Wonderful World keeps between server messages.
#[derive(Clone, Copy)]
pub struct Signals {
    /// The game as our seat sees it, as the server last sent it.
    pub view: RwSignal<Option<View>>,
    /// Seat names, indexed by seat.
    pub names: RwSignal<Vec<String>>,
    /// Who is connected, indexed by seat.
    pub connected: RwSignal<Vec<bool>>,
    /// What the host picked in the lobby (the server's echo).
    pub options: RwSignal<Options>,
    pub ui: Ui,
}

impl Signals {
    pub fn new() -> Signals {
        Signals {
            view: RwSignal::new(None),
            names: RwSignal::new(Vec::new()),
            connected: RwSignal::new(Vec::new()),
            options: RwSignal::new(Options::default()),
            ui: Ui::new(),
        }
    }

    /// Forget the game, e.g. when leaving the room.
    pub fn reset(&self) {
        self.view.set(None);
        self.names.set(Vec::new());
        self.connected.set(Vec::new());
        self.options.set(Options::default());
        self.ui.reset();
    }
}

impl Default for Signals {
    fn default() -> Self {
        Signals::new()
    }
}

pub struct Wonderful;

impl crate::games::Game for Wonderful {
    fn kind(&self) -> GameKind {
        GameKind::Wonderful
    }

    fn icon(&self) -> AnyView {
        screens::icon().into_any()
    }

    fn blurb(&self) -> &'static str {
        "Draft cards, build an empire and out-produce your rivals over four rounds. Everybody plays at once."
    }

    fn frame(&self, _app: App, content: AnyView) -> AnyView {
        screens::shell(content).into_any()
    }

    fn home(&self, app: App) -> AnyView {
        screens::home_screen(app).into_any()
    }

    fn lobby(&self, app: App, lobby: &Lobby) -> AnyView {
        screens::lobby_screen(app, lobby.room.clone(), lobby.players.clone(), lobby.you, lobby.is_host).into_any()
    }

    fn options(&self, app: App, options: GameOptions) {
        let GameOptions::Wonderful(options) = options else { return };
        app.games.wonderful.options.set(options);
    }

    fn board(&self, app: App) -> AnyView {
        board::board(app)
    }

    fn receive(&self, app: App, view: GameView, names: Vec<String>, connected: Vec<bool>) {
        let GameView::Wonderful(view) = view else { return };
        let wonderful = app.games.wonderful;
        wonderful.names.set(names);
        wonderful.connected.set(connected);
        // A new step drops the pick: what was picked and not placed stays in
        // your reserve, and isn't spent by a click in the next step.
        if wonderful.view.with_untracked(|v| v.as_ref().map(|v| v.phase)) != Some(view.phase) {
            wonderful.ui.held.set(None);
        }
        wonderful.view.set(Some(*view));
    }

    fn reset(&self, app: App) {
        app.games.wonderful.reset();
    }

    fn snapshot(&self, app: App) -> serde_json::Value {
        let wonderful = app.games.wonderful;
        serde_json::json!({
            "view": wonderful.view.get_untracked(),
            "names": wonderful.names.get_untracked(),
            "connected": wonderful.connected.get_untracked(),
            "options": wonderful.options.get_untracked(),
        })
    }
}
