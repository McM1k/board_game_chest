//! Catan: the island (`map.rs`, with the tiles drawn in `art.rs`), the game
//! screen around it (`board.rs`), the screens before the game
//! (`screens.rs`) and the ember contour on what just changed (`look.rs`).
//! Styles in `catan.css`, scoped under `.theme-catan`.
//!
//! The game screen is redrawn on every server update; what has to outlive
//! that (what is being built, a trade being put together) is in [`Ui`].

pub mod art;
pub mod board;
pub mod look;
pub mod map;
pub mod screens;

use crate::app::{App, Lobby};
use leptos::prelude::*;
use protocol::{GameKind, GameView};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    None,
    Road,
    Settlement,
    City,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Picker {
    YearOfPlenty,
    Monopoly,
}

/// Purely client-side interface state, which survives the game screen
/// being redrawn on every server update.
#[derive(Clone, Copy)]
pub struct Ui {
    pub mode: RwSignal<Mode>,
    pub robber_tile: RwSignal<Option<usize>>,
    pub give: RwSignal<[u8; 5]>,
    pub want: RwSignal<[u8; 5]>,
    pub bank_give: RwSignal<Option<usize>>,
    pub bank_get: RwSignal<Option<usize>>,
    pub discard: RwSignal<[u8; 5]>,
    pub picker: RwSignal<Option<Picker>>,
    pub picks: RwSignal<Vec<usize>>,
}

impl Ui {
    fn new() -> Ui {
        Ui {
            mode: RwSignal::new(Mode::None),
            robber_tile: RwSignal::new(None),
            give: RwSignal::new([0; 5]),
            want: RwSignal::new([0; 5]),
            bank_give: RwSignal::new(None),
            bank_get: RwSignal::new(None),
            discard: RwSignal::new([0; 5]),
            picker: RwSignal::new(None),
            picks: RwSignal::new(vec![]),
        }
    }

    fn reset(&self) {
        self.mode.set(Mode::None);
        self.robber_tile.set(None);
        self.give.set([0; 5]);
        self.want.set([0; 5]);
        self.bank_give.set(None);
        self.bank_get.set(None);
        self.discard.set([0; 5]);
        self.picker.set(None);
        self.picks.set(vec![]);
    }
}

/// What Catan keeps between server messages.
#[derive(Clone, Copy)]
pub struct Signals {
    /// The game as our seat sees it, as the server last sent it.
    pub view: RwSignal<Option<catan_core::GameView>>,
    /// Who is connected, indexed by seat.
    pub connected: RwSignal<Vec<bool>>,
    pub ui: Ui,
    /// Board changes still showing their ember contour.
    pub fresh: RwSignal<Vec<look::Fresh>>,
    /// What the board looked like at the previous update.
    pub snap: StoredValue<Option<look::Snap>>,
}

impl Signals {
    pub fn new() -> Signals {
        Signals {
            view: RwSignal::new(None),
            connected: RwSignal::new(vec![]),
            ui: Ui::new(),
            fresh: RwSignal::new(vec![]),
            snap: StoredValue::new(None),
        }
    }

    /// Forget the game, e.g. when leaving the room.
    pub fn reset(&self) {
        self.view.set(None);
        self.connected.set(vec![]);
        self.ui.reset();
        self.fresh.set(vec![]);
        self.snap.set_value(None);
    }
}

impl Default for Signals {
    fn default() -> Self {
        Signals::new()
    }
}

pub struct Catan;

impl crate::games::Game for Catan {
    fn kind(&self) -> GameKind {
        GameKind::Catan
    }

    fn icon(&self) -> AnyView {
        screens::icon().into_any()
    }

    fn blurb(&self) -> &'static str {
        "Build roads and settlements, trade resources and race to 10 points on a random island."
    }

    fn frame(&self, _app: App, content: AnyView) -> AnyView {
        view! { <div class="theme-catan">{content}</div> }.into_any()
    }

    fn home(&self, app: App) -> AnyView {
        screens::home_screen(app).into_any()
    }

    fn lobby(&self, app: App, lobby: &Lobby) -> AnyView {
        screens::lobby_screen(app, lobby.room.clone(), lobby.players.clone(), lobby.is_host).into_any()
    }

    fn board(&self, app: App) -> AnyView {
        board::board(app)
    }

    fn receive(&self, app: App, view: GameView, _names: Vec<String>, connected: Vec<bool>) {
        // The names come with Catan's own view.
        let GameView::Catan(view) = view else { return };
        look::note_changes(app, &view);
        app.games.catan.connected.set(connected);
        app.games.catan.view.set(Some(*view));
    }

    fn reset(&self, app: App) {
        app.games.catan.reset();
    }

    fn snapshot(&self, app: App) -> serde_json::Value {
        let catan = app.games.catan;
        serde_json::json!({
            "view": catan.view.get_untracked(),
            "connected": catan.connected.get_untracked(),
        })
    }
}
