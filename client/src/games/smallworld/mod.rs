//! Small World: the board (`board.rs`, with the map drawn in `map.rs` from
//! the outlines in `shapes.rs`), the screens before the game and the page
//! frame (`screens.rs`), and what a click means (`look.rs`). Styles in
//! `smallworld.css`, scoped under `.smallworld`.
//!
//! The game screen is redrawn from the state the server last sent (kept in
//! [`Signals`]) on every message. What only the browser knows (the tool in
//! hand, a die roll waiting to be confirmed) lives in [`Ui`], which outlives
//! those redraws.

pub mod board;
pub mod look;
pub mod map;
pub mod screens;
pub mod shapes;

use crate::app::{load, save, App, Lobby};
use leptos::prelude::*;
use protocol::{GameKind, GameOptions, GameView};
use smallworld_core::{Options, RegionMove, View};

/// Interface state that survives the screen being rebuilt.
#[derive(Clone, Copy)]
pub struct Ui {
    /// What a click on a region does (see `look::effective_tool`).
    pub tool: RwSignal<look::Tool>,
    /// A region picked for the last conquest with the die, waiting for the
    /// player to confirm the roll.
    pub gamble: RwSignal<Option<usize>>,
    /// The map drawn in flat colours rather than the printed board's
    /// picture. A preference, kept by the browser.
    pub drawn: RwSignal<bool>,
}

/// Where the map style is kept.
const KEY_MAP: &str = "games.pref.smallworld.map";

impl Ui {
    pub fn new() -> Ui {
        let drawn = load(KEY_MAP).as_deref() == Some("drawn");
        Ui { tool: RwSignal::new(RegionMove::Conquer), gamble: RwSignal::new(None), drawn: RwSignal::new(drawn) }
    }

    /// Switches between the printed board and the drawn map, and remembers it.
    pub fn set_drawn(&self, drawn: bool) {
        self.drawn.set(drawn);
        save(KEY_MAP, if drawn { "drawn" } else { "printed" });
    }

    /// Forget everything, e.g. when leaving the room (but the map style).
    pub fn reset(&self) {
        self.tool.set(RegionMove::Conquer);
        self.gamble.set(None);
    }
}

impl Default for Ui {
    fn default() -> Self {
        Ui::new()
    }
}

/// What Small World keeps between server messages.
#[derive(Clone, Copy)]
pub struct Signals {
    /// The game as our seat sees it, as the server last sent it.
    pub view: RwSignal<Option<View>>,
    /// Seat names, indexed by seat.
    pub names: RwSignal<Vec<String>>,
    /// Who is connected, indexed by seat.
    pub connected: RwSignal<Vec<bool>>,
    /// The expansions the host picked in the lobby (the server's echo).
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

pub struct SmallWorld;

impl crate::games::Game for SmallWorld {
    fn kind(&self) -> GameKind {
        GameKind::SmallWorld
    }

    fn icon(&self) -> AnyView {
        screens::icon().into_any()
    }

    fn blurb(&self) -> &'static str {
        "Lead fantasy races to conquer a world too small for all of them, then send them into decline and pick a new one. With all six expansions."
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
        let GameOptions::SmallWorld(options) = options else { return };
        app.games.smallworld.options.set(options);
    }

    fn board(&self, app: App) -> AnyView {
        board::board(app)
    }

    fn receive(&self, app: App, view: GameView, names: Vec<String>, connected: Vec<bool>) {
        let GameView::SmallWorld(view) = view else { return };
        let small_world = app.games.smallworld;
        // A die roll waiting for confirmation only makes sense in the state it
        // was picked in.
        small_world.ui.gamble.set(None);
        small_world.names.set(names);
        small_world.connected.set(connected);
        small_world.view.set(Some(*view));
    }

    fn reset(&self, app: App) {
        app.games.smallworld.reset();
    }

    fn snapshot(&self, app: App) -> serde_json::Value {
        let small_world = app.games.smallworld;
        serde_json::json!({
            "view": small_world.view.get_untracked(),
            "names": small_world.names.get_untracked(),
            "connected": small_world.connected.get_untracked(),
            "options": small_world.options.get_untracked(),
            "tool": small_world.ui.tool.get_untracked(),
            "die_asked_for": small_world.ui.gamble.get_untracked(),
        })
    }
}
