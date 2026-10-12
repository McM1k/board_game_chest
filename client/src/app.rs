//! The shell's state and the connection: which screen is up, the room we
//! are in, the WebSocket, and what to do with each message from the server.
//! Nothing here knows a game by name; each game keeps its own state in
//! [`games::Games`] and gets its messages through [`games::game`].

use crate::games::{self, Games};
use crate::report;
use leptos::prelude::*;
use protocol::{ClientMsg, GameKind, LobbyPlayer, ServerMsg};
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

const KEY_ROOM: &str = "games.room";
pub const KEY_TOKEN: &str = "games.token";
pub const KEY_NAME: &str = "games.name";

#[derive(Clone)]
pub enum Screen {
    /// Pick a game.
    Menu,
    /// Name + create/join a room for the chosen game.
    Home(GameKind),
    /// The waiting room. Which game it is for comes with the room: the code
    /// decides, whatever was picked in the menu.
    Lobby(Lobby),
    /// A running game. Carries no data: the game keeps what the server sent
    /// in its own signals (see [`Games`]), so a new state doesn't have to
    /// rebuild the whole screen.
    Playing(GameKind),
}

/// The waiting room, as the server last described it.
#[derive(Clone, Debug, PartialEq)]
pub struct Lobby {
    pub room: String,
    pub game: GameKind,
    pub players: Vec<LobbyPlayer>,
    /// Our own seat, i.e. our index in `players`.
    pub you: usize,
    pub is_host: bool,
}

#[derive(Clone, Copy)]
pub struct App {
    pub ws: StoredValue<Option<WebSocket>, LocalStorage>,
    pub online: RwSignal<bool>,
    pub screen: RwSignal<Screen>,
    pub error: RwSignal<Option<String>>,
    /// The last error shown, kept after the toast is gone (for bug reports).
    pub last_error: RwSignal<Option<String>>,
    pub name: RwSignal<String>,
    pub room_input: RwSignal<String>,
    /// The code of the room we are in, for the games' title bars.
    pub room: RwSignal<String>,
    /// What each game keeps beyond a single screen.
    pub games: Games,
    /// The bug report button's dialog.
    pub report: report::Signals,
}

impl App {
    pub fn new() -> App {
        App {
            ws: StoredValue::new_local(None),
            online: RwSignal::new(false),
            screen: RwSignal::new(Screen::Menu),
            error: RwSignal::new(None),
            last_error: RwSignal::new(None),
            name: RwSignal::new(load(KEY_NAME).unwrap_or_default()),
            room_input: RwSignal::new(String::new()),
            room: RwSignal::new(String::new()),
            games: Games::new(),
            report: report::Signals::new(),
        }
    }
}

/// Where `key` is kept. The name is shared by every tab, as a convenience.
/// The room and the seat token belong to one tab (and survive its reloads):
/// shared, a second tab - another player on the same machine - would
/// reconnect into the first tab's seat instead of reaching the join screen.
#[cfg(target_arch = "wasm32")]
fn storage(key: &str) -> Option<web_sys::Storage> {
    let window = web_sys::window()?;
    if key == KEY_NAME {
        window.local_storage().ok()?
    } else {
        window.session_storage().ok()?
    }
}

// Native builds only exist for `cargo check` and render tests.
#[cfg(not(target_arch = "wasm32"))]
fn storage(_key: &str) -> Option<web_sys::Storage> {
    None
}

pub fn load(key: &str) -> Option<String> {
    storage(key)?.get_item(key).ok()?
}

pub fn save(key: &str, value: &str) {
    if let Some(s) = storage(key) {
        let _ = s.set_item(key, value);
    }
}

fn clear_session() {
    if let Some(s) = storage(KEY_ROOM) {
        let _ = s.remove_item(KEY_ROOM);
        let _ = s.remove_item(KEY_TOKEN);
    }
}

fn ws_url() -> String {
    let loc = web_sys::window().unwrap().location();
    let secure = loc.protocol().unwrap_or_default() == "https:";
    let host = loc.host().unwrap_or_default();
    format!("{}://{}/ws", if secure { "wss" } else { "ws" }, host)
}

pub fn send(app: App, msg: &ClientMsg) {
    let Ok(text) = serde_json::to_string(msg) else { return };
    app.ws.with_value(|ws| {
        if let Some(ws) = ws {
            let _ = ws.send_with_str(&text);
        }
    });
}

pub fn show_error(app: App, msg: String) {
    app.last_error.set(Some(msg.clone()));
    app.error.set(Some(msg));
    set_timeout(move || app.error.set(None), Duration::from_secs(5));
}

/// Opens the WebSocket and keeps it alive: on close it retries and, if a
/// saved session exists, rejoins the same seat.
pub fn connect(app: App) {
    let ws = match WebSocket::new(&ws_url()) {
        Ok(ws) => ws,
        Err(_) => {
            set_timeout(move || connect(app), Duration::from_secs(2));
            return;
        }
    };

    let onopen = Closure::<dyn FnMut()>::new(move || {
        app.online.set(true);
        if let (Some(room), Some(token)) = (load(KEY_ROOM), load(KEY_TOKEN)) {
            let name = load(KEY_NAME).unwrap_or_default();
            send(
                app,
                &ClientMsg::Join {
                    room,
                    name,
                    token: Some(token),
                },
            );
        }
    });
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
        let Some(text) = e.data().as_string() else { return };
        match serde_json::from_str::<ServerMsg>(&text) {
            Ok(msg) => handle(app, msg),
            Err(err) => show_error(app, format!("Bad message from server: {err}")),
        }
    });
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let onclose = Closure::<dyn FnMut()>::new(move || {
        app.online.set(false);
        set_timeout(move || connect(app), Duration::from_secs(2));
    });
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    app.ws.set_value(Some(ws));
}

pub fn handle(app: App, msg: ServerMsg) {
    match msg {
        ServerMsg::Joined { room, token } => {
            save(KEY_ROOM, &room);
            save(KEY_TOKEN, &token);
            app.room.set(room);
            app.error.set(None);
        }
        ServerMsg::Lobby {
            room,
            game,
            players,
            you,
            is_host,
            options,
        } => {
            app.room.set(room.clone());
            let lobby = Lobby { room, game, players, you, is_host };
            let (arriving, unchanged) = app.screen.with_untracked(|s| match s {
                Screen::Lobby(shown) => (shown.room != lobby.room, *shown == lobby),
                _ => (true, false),
            });
            // A game starts afresh in its waiting room.
            if arriving {
                games::game(game).reset(app);
            }
            if let Some(options) = options {
                games::game(game).options(app, options);
            }
            // The lobby is sent again whenever somebody joins, leaves or the
            // host changes an option. Only rebuild the screen when what it
            // shows changed (the options have their own signals), so the
            // controls the host is using aren't torn down under their cursor.
            if !unchanged {
                app.screen.set(Screen::Lobby(lobby));
            }
        }
        ServerMsg::Game { view, names, connected } => {
            let kind = view.game();
            games::game(kind).receive(app, view, names, connected);
            // The board is built once; later states only change its signals.
            if !matches!(app.screen.get_untracked(), Screen::Playing(k) if k == kind) {
                app.screen.set(Screen::Playing(kind));
            }
        }
        ServerMsg::Error(e) => {
            // A failed automatic rejoin means the saved session is dead.
            if matches!(app.screen.get_untracked(), Screen::Menu | Screen::Home(_)) {
                clear_session();
            }
            show_error(app, e);
        }
    }
}

/// Opens a room for `game` under the name typed on the home screen.
pub fn create_room(app: App, game: GameKind) {
    let name = app.name.get_untracked();
    save(KEY_NAME, &name);
    send(app, &ClientMsg::Create { game, name });
}

/// Joins the room whose code is typed on the home screen. The code decides
/// which game you land in.
pub fn join_room(app: App) {
    let name = app.name.get_untracked();
    save(KEY_NAME, &name);
    send(
        app,
        &ClientMsg::Join {
            room: app.room_input.get_untracked(),
            name,
            token: None,
        },
    );
}

pub fn leave(app: App) {
    send(app, &ClientMsg::Leave);
    clear_session();
    // Leave the screen first, so no board outlives its game's state.
    app.screen.set(Screen::Menu);
    app.room.set(String::new());
    for kind in GameKind::ALL {
        games::game(kind).reset(app);
    }
}
