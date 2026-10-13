//! Barrage: the board (`board.rs`, with the map drawn in `map.rs`), the
//! screens before the game and the page frame (`screens.rs`), and the texts
//! and what a click means (`look.rs`). Styles in `barrage.css`, scoped under
//! `.barrage`.
//!
//! The game screen is redrawn from the state the server last sent (kept in
//! [`Signals`]) on every message. What only the browser knows (the action
//! space or tile picked before the rest of the move is chosen) lives in
//! [`Ui`], which outlives those redraws until the game moves on.

pub mod board;
pub mod look;
pub mod map;
pub mod screens;

use crate::app::{load, save, App, Lobby};
use barrage_core::{Pick, Spot, Tile, View, HEADSTREAMS};
use leptos::prelude::*;
use protocol::{GameKind, GameView};

/// Interface state that survives the screen being rebuilt.
#[derive(Clone, Copy)]
pub struct Ui {
    /// A Management space picked, waiting for the rest of the move (which
    /// production, where the drops go, which Contracts...).
    pub spot: RwSignal<Option<Spot>>,
    /// The Technology tile picked to build with (and the tile Anton Krylov
    /// copies): the map then marks where it can build.
    pub tile: RwSignal<Option<(Option<Tile>, Option<Tile>)>>,
    /// Jill McDowell: pay Conduits with Concrete Mixers.
    pub mixers: RwSignal<bool>,
    /// Drops placed on the Headstreams so far, before confirming.
    pub drops: RwSignal<[u8; HEADSTREAMS]>,
    /// Contracts picked in the Contract Office.
    pub picks: RwSignal<Vec<Pick>>,
    /// Engineers to send to the Bank.
    pub bank: RwSignal<u8>,
    /// Setup: the pair and the Starting Contract picked.
    pub pair: RwSignal<Option<usize>>,
    pub contract: RwSignal<Option<usize>>,
    /// The map drawn in flat colours rather than the printed board's
    /// picture. A preference, kept by the browser.
    pub drawn: RwSignal<bool>,
}

/// Where the map style is kept.
const KEY_MAP: &str = "games.pref.barrage.map";

impl Ui {
    pub fn new() -> Ui {
        Ui {
            spot: RwSignal::new(None),
            tile: RwSignal::new(None),
            mixers: RwSignal::new(false),
            drops: RwSignal::new([0; HEADSTREAMS]),
            picks: RwSignal::new(Vec::new()),
            bank: RwSignal::new(1),
            pair: RwSignal::new(None),
            contract: RwSignal::new(None),
            drawn: RwSignal::new(load(KEY_MAP).as_deref() == Some("drawn")),
        }
    }

    /// Switches between the printed board and the drawn map, and remembers it.
    pub fn set_drawn(&self, drawn: bool) {
        self.drawn.set(drawn);
        save(KEY_MAP, if drawn { "drawn" } else { "printed" });
    }

    /// Forget a move half chosen.
    pub fn clear(&self) {
        self.spot.set(None);
        self.tile.set(None);
        self.drops.set([0; HEADSTREAMS]);
        self.picks.set(Vec::new());
        self.bank.set(1);
        self.pair.set(None);
        self.contract.set(None);
    }
}

impl Default for Ui {
    fn default() -> Self {
        Ui::new()
    }
}

/// What Barrage keeps between server messages.
#[derive(Clone, Copy)]
pub struct Signals {
    /// The game as our seat sees it, as the server last sent it.
    pub view: RwSignal<Option<View>>,
    /// Seat names, indexed by seat.
    pub names: RwSignal<Vec<String>>,
    /// Who is connected, indexed by seat.
    pub connected: RwSignal<Vec<bool>>,
    pub ui: Ui,
}

impl Signals {
    pub fn new() -> Signals {
        Signals {
            view: RwSignal::new(None),
            names: RwSignal::new(Vec::new()),
            connected: RwSignal::new(Vec::new()),
            ui: Ui::new(),
        }
    }

    /// Forget the game, e.g. when leaving the room.
    pub fn reset(&self) {
        self.view.set(None);
        self.names.set(Vec::new());
        self.connected.set(Vec::new());
        self.ui.clear();
        self.ui.mixers.set(false);
    }
}

impl Default for Signals {
    fn default() -> Self {
        Signals::new()
    }
}

pub struct Barrage;

impl crate::games::Game for Barrage {
    fn kind(&self) -> GameKind {
        GameKind::Barrage
    }

    fn icon(&self) -> AnyView {
        screens::icon().into_any()
    }

    fn blurb(&self) -> &'static str {
        "Build dams, conduits and powerhouses in the Alps, hold back the water and turn it into energy to fulfill contracts."
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

    fn board(&self, app: App) -> AnyView {
        board::board(app)
    }

    fn receive(&self, app: App, view: GameView, names: Vec<String>, connected: Vec<bool>) {
        let GameView::Barrage(view) = view else { return };
        let barrage = app.games.barrage;
        // A move half chosen only makes sense in the state it was chosen in
        // (a seat coming or going changes nothing in the game).
        if barrage.view.get_untracked().as_ref() != Some(&*view) {
            barrage.ui.clear();
        }
        barrage.names.set(names);
        barrage.connected.set(connected);
        barrage.view.set(Some(*view));
    }

    fn reset(&self, app: App) {
        app.games.barrage.reset();
    }

    fn snapshot(&self, app: App) -> serde_json::Value {
        let barrage = app.games.barrage;
        let ui = barrage.ui;
        serde_json::json!({
            "view": barrage.view.get_untracked(),
            "names": barrage.names.get_untracked(),
            "connected": barrage.connected.get_untracked(),
            "spot": ui.spot.get_untracked(),
            "tile": ui.tile.get_untracked(),
            "drops": ui.drops.get_untracked(),
            "picks": ui.picks.get_untracked(),
        })
    }
}
