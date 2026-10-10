//! Test helpers: fake connections that drive the rooms without sockets.

use crate::games::Running;
use crate::rooms::{process, Ident, Rooms, Shared, Tx};
use protocol::{ClientMsg, GameKind, GameView, ServerMsg};
use std::any::Any;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, UnboundedReceiver};

/// A fake browser connection: what it sends goes through `process`, what
/// the server sends back lands in `rx`.
pub struct Client {
    pub tx: Tx,
    pub rx: UnboundedReceiver<ServerMsg>,
    pub ident: Option<Ident>,
}

impl Client {
    pub fn new() -> Client {
        let (tx, rx) = mpsc::unbounded_channel();
        Client { tx, rx, ident: None }
    }

    pub fn send(&mut self, state: &Shared, msg: ClientMsg) -> Option<String> {
        process(state, &self.tx, &mut self.ident, msg)
    }

    pub fn drain(&mut self) -> Vec<ServerMsg> {
        let mut out = Vec::new();
        while let Ok(m) = self.rx.try_recv() {
            out.push(m);
        }
        out
    }

    pub fn token(&self) -> String {
        self.ident.as_ref().expect("joined").token.clone()
    }

    /// The latest game state among the messages received since the last
    /// drain: the view, the names and who is connected.
    pub fn latest_game(&mut self) -> Option<(GameView, Vec<String>, Vec<bool>)> {
        self.drain().into_iter().rev().find_map(|m| match m {
            ServerMsg::Game { view, names, connected } => Some((view, names, connected)),
            _ => None,
        })
    }
}

pub fn new_state() -> Shared {
    Arc::new(Mutex::new(Rooms::default()))
}

pub fn create(state: &Shared, game: GameKind, name: &str) -> (Client, String) {
    let mut c = Client::new();
    assert_eq!(c.send(state, ClientMsg::Create { game, name: name.into() }), None);
    let code = c.ident.as_ref().unwrap().room.clone();
    (c, code)
}

pub fn join(state: &Shared, code: &str, name: &str) -> Result<Client, String> {
    let mut c = Client::new();
    match c.send(state, ClientMsg::Join { room: code.into(), name: name.into(), token: None }) {
        None => Ok(c),
        Some(e) => Err(e),
    }
}

/// A lobby with `n` seated players called P0, P1, ...; the first one hosts.
pub fn lobby(state: &Shared, game: GameKind, n: usize) -> (String, Vec<Client>) {
    let (host, code) = create(state, game, "P0");
    let mut clients = vec![host];
    for i in 1..n {
        clients.push(join(state, &code, &format!("P{i}")).unwrap());
    }
    for c in &mut clients {
        c.drain();
    }
    (code, clients)
}

/// Runs `f` on the game running in room `code`, as the rules crate's own
/// type `T` (a test may look inside, or set up a position).
pub fn with_game<T: Running, R>(state: &Shared, code: &str, f: impl FnOnce(&mut T) -> R) -> R {
    let mut rooms = state.lock().unwrap();
    let game = rooms.map.get_mut(code).expect("no such room").game.as_mut().expect("the game hasn't started");
    let game: &mut dyn Any = game.as_mut();
    f(game.downcast_mut::<T>().expect("a game of another type"))
}
