//! Rooms: one join-code system for every game.
//!
//! A player creates a room for a game and gets a 4-letter code; the others
//! join with that code, the host (first seat) starts the game, and a
//! per-seat token lets a player come back after a dropped connection. The
//! room's code decides which game you end up in. Nothing here knows a game
//! by name: how one starts, which options it takes, its moves and what each
//! seat sees are behind [`games::Game`] and [`games::Running`].
//!
//! Everything here is synchronous and holds the lock only briefly, so it
//! can be tested without sockets: a "connection" is just a channel.

use crate::games::{self, Running, WRONG_GAME};
use protocol::{ClientMsg, GameKind, GameOptions, LobbyPlayer, ServerMsg};
use rand::{seq::SliceRandom, Rng};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::mpsc::UnboundedSender;

const MAX_NAME_LEN: usize = 20;

pub type Tx = UnboundedSender<ServerMsg>;
pub type Shared = Arc<Mutex<Rooms>>;

#[derive(Default)]
pub struct Rooms {
    pub map: HashMap<String, Room>,
}

pub struct Room {
    pub code: String,
    pub kind: GameKind,
    pub seats: Vec<Seat>,
    /// The options picked in the lobby, for a game that has any.
    pub options: Option<GameOptions>,
    /// The game in progress, once the host has started it.
    pub game: Option<Box<dyn Running>>,
}

pub struct Seat {
    pub name: String,
    pub token: String,
    /// `None` while the player is disconnected.
    pub tx: Option<Tx>,
    /// When the connection last dropped (only meaningful while `tx` is `None`).
    pub dropped_at: Option<Instant>,
}

/// Which seat a connection controls.
pub struct Ident {
    pub room: String,
    pub token: String,
}

impl Room {
    fn is_host(&self, token: &str) -> bool {
        self.seats.first().is_some_and(|s| s.token == token)
    }

    fn lobby_players(&self) -> Vec<LobbyPlayer> {
        self.seats
            .iter()
            .map(|s| LobbyPlayer {
                name: s.name.clone(),
                connected: s.tx.is_some(),
            })
            .collect()
    }
}

fn random_string(len: usize, alphabet: &[u8]) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| alphabet[rng.gen_range(0..alphabet.len())] as char)
        .collect()
}

fn new_token() -> String {
    random_string(24, b"abcdefghijklmnopqrstuvwxyz0123456789")
}

fn clean_name(name: &str) -> Result<String, String> {
    let name: String = name.trim().chars().take(MAX_NAME_LEN).collect();
    if name.is_empty() {
        Err("Please enter a name.".into())
    } else {
        Ok(name)
    }
}

/// Handles one client message. Returns an error string to send back, if any.
pub fn process(state: &Shared, tx: &Tx, ident: &mut Option<Ident>, msg: ClientMsg) -> Option<String> {
    let mut rooms = state.lock().unwrap();
    match msg {
        ClientMsg::Create { game, name } => {
            if ident.is_some() {
                return Some("You are already in a room.".into());
            }
            let name = match clean_name(&name) {
                Ok(n) => n,
                Err(e) => return Some(e),
            };
            let code = loop {
                let c = random_string(4, b"ABCDEFGHJKLMNPQRSTUVWXYZ");
                if !rooms.map.contains_key(&c) {
                    break c;
                }
            };
            let token = new_token();
            rooms.map.insert(
                code.clone(),
                Room {
                    code: code.clone(),
                    kind: game,
                    seats: vec![Seat {
                        name,
                        token: token.clone(),
                        tx: Some(tx.clone()),
                        dropped_at: None,
                    }],
                    options: games::game(game).default_options(),
                    game: None,
                },
            );
            let _ = tx.send(ServerMsg::Joined { room: code.clone(), token: token.clone() });
            broadcast(&rooms.map[&code]);
            *ident = Some(Ident { room: code, token });
            None
        }
        ClientMsg::Join { room, name, token } => {
            if ident.is_some() {
                return Some("You are already in a room.".into());
            }
            let code = room.trim().to_uppercase();
            let Some(r) = rooms.map.get_mut(&code) else {
                return Some("No such room.".into());
            };
            // Reconnect with a known token.
            if let Some(tok) = token {
                if let Some(seat) = r.seats.iter_mut().find(|s| s.token == tok) {
                    seat.tx = Some(tx.clone());
                    let _ = tx.send(ServerMsg::Joined { room: code.clone(), token: tok.clone() });
                    broadcast(r);
                    *ident = Some(Ident { room: code, token: tok });
                    return None;
                }
            }
            if r.game.is_some() {
                return Some("That game has already started.".into());
            }
            if r.seats.len() >= r.kind.max_players_with(r.options.as_ref()) {
                return Some("That room is full.".into());
            }
            let name = match clean_name(&name) {
                Ok(n) => n,
                Err(e) => return Some(e),
            };
            if r.seats.iter().any(|s| s.name.eq_ignore_ascii_case(&name)) {
                return Some("That name is taken in this room.".into());
            }
            let tok = new_token();
            r.seats.push(Seat {
                name,
                token: tok.clone(),
                tx: Some(tx.clone()),
                dropped_at: None,
            });
            let _ = tx.send(ServerMsg::Joined { room: code.clone(), token: tok.clone() });
            broadcast(r);
            *ident = Some(Ident { room: code, token: tok });
            None
        }
        ClientMsg::SetOptions(options) => {
            let id = ident.as_ref()?;
            let r = rooms.map.get_mut(&id.room)?;
            if r.options.is_none() {
                return Some("This game has no options to set.".into());
            }
            if options.game() != r.kind {
                return Some("Those options are for another game.".into());
            }
            if r.game.is_some() {
                return Some("The game has already started.".into());
            }
            if !r.is_host(&id.token) {
                return Some("Only the host can change the options.".into());
            }
            match games::game(r.kind).check_options(options) {
                Ok(options) if r.seats.len() > r.kind.max_players_with(Some(&options)) => {
                    Some("There are too many players here for that.".into())
                }
                Ok(options) => {
                    r.options = Some(options);
                    broadcast(r);
                    None
                }
                Err(e) => Some(e),
            }
        }
        ClientMsg::Start => {
            let id = ident.as_ref()?;
            let r = rooms.map.get_mut(&id.room)?;
            if r.game.is_some() {
                return Some("The game has already started.".into());
            }
            if !r.is_host(&id.token) {
                return Some("Only the host can start the game.".into());
            }
            if r.seats.len() < r.kind.min_players() {
                return Some("You need at least two players.".into());
            }
            let game = games::game(r.kind);
            let mut rng = rand::thread_rng();
            if game.shuffles_seats() {
                r.seats.shuffle(&mut rng);
            }
            let names = r.seats.iter().map(|s| s.name.clone()).collect();
            r.game = Some(game.start(names, r.options.as_ref(), &mut rng));
            broadcast(r);
            None
        }
        ClientMsg::Leave => {
            let id = ident.take()?;
            leave(&mut rooms, &id, tx, true);
            None
        }
        ClientMsg::Move(mv) => {
            let id = ident.as_ref()?;
            let r = rooms.map.get_mut(&id.room)?;
            // The seat comes from the connection's token, never from the message.
            let seat = r.seats.iter().position(|s| s.token == id.token)?;
            if mv.game() != r.kind {
                return Some(WRONG_GAME.into());
            }
            let Some(game) = r.game.as_mut() else {
                return Some("The game hasn't started yet.".into());
            };
            match game.play(seat, mv) {
                Ok(()) => {
                    broadcast(r);
                    None
                }
                Err(e) => Some(e),
            }
        }
    }
}

/// Sends every connected seat its own view of the room: the lobby, or the
/// running game as that seat may see it.
pub fn broadcast(room: &Room) {
    let connected: Vec<bool> = room.seats.iter().map(|s| s.tx.is_some()).collect();
    let names: Vec<String> = room.seats.iter().map(|s| s.name.clone()).collect();
    for (i, seat) in room.seats.iter().enumerate() {
        let Some(tx) = &seat.tx else { continue };
        let msg = match &room.game {
            Some(game) => ServerMsg::Game {
                view: game.view(i),
                names: names.clone(),
                connected: connected.clone(),
            },
            None => ServerMsg::Lobby {
                room: room.code.clone(),
                game: room.kind,
                players: room.lobby_players(),
                you: i,
                is_host: i == 0,
                options: room.options.clone(),
            },
        };
        let _ = tx.send(msg);
    }
}

fn leave(rooms: &mut Rooms, id: &Ident, tx: &Tx, explicit: bool) {
    let Some(r) = rooms.map.get_mut(&id.room) else { return };
    if explicit && r.game.is_none() {
        free_seat(rooms, id);
        return;
    }
    if let Some(seat) = r.seats.iter_mut().find(|s| s.token == id.token) {
        // Otherwise the seat is kept so the player can come back (to a lobby
        // only for a while, see `free_dropped_seat`), unless the connection
        // was already replaced by a reconnect.
        if explicit || seat.tx.as_ref().is_some_and(|t| t.same_channel(tx)) {
            seat.tx = None;
            seat.dropped_at = Some(Instant::now());
        }
    }
    broadcast(r);
}

/// Gives a lobby seat up (the next player in line becomes host), closing
/// the room if it was the last one.
fn free_seat(rooms: &mut Rooms, id: &Ident) {
    let Some(r) = rooms.map.get_mut(&id.room) else { return };
    r.seats.retain(|s| s.token != id.token);
    if r.seats.is_empty() {
        rooms.map.remove(&id.room);
    } else {
        broadcast(r);
    }
}

/// Frees a lobby seat whose player dropped at least `grace` ago and hasn't
/// come back, so a reload or a phone switching apps doesn't cost the seat
/// but a player who is really gone doesn't stay in the way. Seats in a
/// running game are kept for good.
pub fn free_dropped_seat(state: &Shared, id: &Ident, grace: Duration) {
    let mut rooms = state.lock().unwrap();
    let Some(r) = rooms.map.get(&id.room) else { return };
    let gone = r.game.is_none()
        && r.seats.iter().any(|s| {
            s.token == id.token && s.tx.is_none() && s.dropped_at.is_some_and(|t| t.elapsed() >= grace)
        });
    if gone {
        free_seat(&mut rooms, id);
    }
}

/// A connection has closed. Returns the seat it held, to free or to check
/// the room for abandonment later, if there is one.
pub fn disconnect(state: &Shared, tx: &Tx, ident: Option<Ident>) -> Option<Ident> {
    let id = ident?;
    let mut rooms = state.lock().unwrap();
    // Ignore stale disconnects from a connection that was replaced.
    if let Some(r) = rooms.map.get(&id.room) {
        let current = r
            .seats
            .iter()
            .find(|s| s.token == id.token)
            .and_then(|s| s.tx.as_ref())
            .is_some_and(|t| t.same_channel(tx));
        if !current {
            return None;
        }
    }
    leave(&mut rooms, &id, tx, false);
    Some(id)
}

/// Drops the room if everybody has left it.
pub fn remove_if_abandoned(state: &Shared, code: &str) {
    let mut rooms = state.lock().unwrap();
    let abandoned = rooms
        .map
        .get(code)
        .is_some_and(|r| r.seats.iter().all(|s| s.tx.is_none()));
    if abandoned {
        rooms.map.remove(code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::*;
    use protocol::GameMove;

    // ----- the shared room system ------------------------------------

    #[test]
    fn creating_a_room_gives_a_code_a_token_and_a_lobby() {
        let state = new_state();
        let (mut host, code) = create(&state, GameKind::Catan, "Ann");
        assert_eq!(code.len(), 4);
        assert!(code.chars().all(|c| c.is_ascii_uppercase()));
        let msgs = host.drain();
        assert!(matches!(&msgs[0], ServerMsg::Joined { room, token } if *room == code && !token.is_empty()));
        match &msgs[1] {
            ServerMsg::Lobby { room, game, players, you, is_host, options } => {
                assert_eq!(room, &code);
                assert_eq!(*game, GameKind::Catan);
                assert_eq!(players.len(), 1);
                assert_eq!((*you, *is_host), (0, true));
                assert!(options.is_none(), "Catan has no lobby options");
            }
            other => panic!("expected the lobby, got {other:?}"),
        }
    }

    #[test]
    fn the_room_code_decides_the_game() {
        let state = new_state();
        let (_a, hanabi_code) = create(&state, GameKind::Hanabi, "Ann");
        let (_b, catan_code) = create(&state, GameKind::Catan, "Bob");

        let mut c = join(&state, &hanabi_code.to_lowercase(), "Cy").unwrap();
        let mut d = join(&state, &catan_code, "Di").unwrap();
        let game_of = |c: &mut Client| {
            c.drain().into_iter().find_map(|m| match m {
                ServerMsg::Lobby { game, .. } => Some(game),
                _ => None,
            })
        };
        assert_eq!(game_of(&mut c), Some(GameKind::Hanabi));
        assert_eq!(game_of(&mut d), Some(GameKind::Catan));
    }

    #[test]
    fn unknown_rooms_names_and_double_joins_are_refused() {
        let state = new_state();
        let (mut host, code) = create(&state, GameKind::Hanabi, "Ann");
        assert_eq!(join(&state, "ZZZZ", "Bob").err().as_deref(), Some("No such room."));
        assert_eq!(join(&state, &code, "  ").err().as_deref(), Some("Please enter a name."));
        assert_eq!(join(&state, &code, "ann").err().as_deref(), Some("That name is taken in this room."));
        // Already seated: no second room, no second seat.
        assert_eq!(
            host.send(&state, ClientMsg::Create { game: GameKind::Hanabi, name: "Ann".into() }).as_deref(),
            Some("You are already in a room.")
        );
        assert_eq!(
            host.send(&state, ClientMsg::Join { room: code, name: "Ann".into(), token: None }).as_deref(),
            Some("You are already in a room.")
        );
    }

    #[test]
    fn long_names_are_cut_to_fit() {
        let state = new_state();
        let (_host, code) = create(&state, GameKind::Hanabi, "Ann");
        let guest = join(&state, &code, &"x".repeat(50)).unwrap();
        let rooms = state.lock().unwrap();
        let seat = rooms.map[&code].seats.iter().find(|s| s.token == guest.token()).unwrap();
        assert_eq!(seat.name.chars().count(), MAX_NAME_LEN);
    }

    #[test]
    fn rooms_fill_up_to_each_games_limit() {
        let state = new_state();
        let (code, _hanabi) = lobby(&state, GameKind::Hanabi, 5);
        assert_eq!(join(&state, &code, "P6").err().as_deref(), Some("That room is full."));

        let (code, _catan) = lobby(&state, GameKind::Catan, 4);
        assert_eq!(join(&state, &code, "P5").err().as_deref(), Some("That room is full."));

        let (code, _wonderful) = lobby(&state, GameKind::Wonderful, 5);
        assert_eq!(join(&state, &code, "P6").err().as_deref(), Some("That room is full."));

        let (code, _small_world) = lobby(&state, GameKind::SmallWorld, 5);
        assert_eq!(join(&state, &code, "P6").err().as_deref(), Some("That room is full."));
    }

    #[test]
    fn only_the_host_can_start_and_only_with_two_players() {
        for game in GameKind::ALL {
            let state = new_state();
            let (mut host, code) = create(&state, game, "Ann");
            assert_eq!(host.send(&state, ClientMsg::Start).as_deref(), Some("You need at least two players."));

            let mut guest = join(&state, &code, "Bob").unwrap();
            assert_eq!(guest.send(&state, ClientMsg::Start).as_deref(), Some("Only the host can start the game."));
            assert!(state.lock().unwrap().map[&code].game.is_none());

            assert_eq!(host.send(&state, ClientMsg::Start), None);
            assert!(state.lock().unwrap().map[&code].game.is_some());
            assert_eq!(host.send(&state, ClientMsg::Start).as_deref(), Some("The game has already started."));
        }
    }

    #[test]
    fn nobody_new_can_join_once_the_game_has_started() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Hanabi, 2);
        assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
        assert_eq!(join(&state, &code, "Late").err().as_deref(), Some("That game has already started."));
    }

    #[test]
    fn leaving_a_lobby_frees_the_seat_and_passes_the_host_role_on() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Hanabi, 3);
        assert_eq!(clients[0].send(&state, ClientMsg::Leave), None);

        let msgs = clients[1].drain();
        match msgs.last().unwrap() {
            ServerMsg::Lobby { players, you, is_host, .. } => {
                let names: Vec<_> = players.iter().map(|p| p.name.as_str()).collect();
                assert_eq!(names, ["P1", "P2"]);
                assert_eq!((*you, *is_host), (0, true), "the next seat in line hosts");
            }
            other => panic!("expected the lobby, got {other:?}"),
        }
        // The freed seat can be taken again.
        join(&state, &code, "P0").unwrap();
        assert_eq!(state.lock().unwrap().map[&code].seats.len(), 3);
    }

    #[test]
    fn an_emptied_lobby_closes_the_room() {
        let state = new_state();
        let (mut host, code) = create(&state, GameKind::Catan, "Ann");
        host.send(&state, ClientMsg::Leave);
        assert!(!state.lock().unwrap().map.contains_key(&code));
    }

    /// The names in the latest lobby message in `msgs`, offline ones marked.
    fn lobby_names(msgs: &[ServerMsg]) -> Vec<String> {
        msgs.iter()
            .rev()
            .find_map(|m| match m {
                ServerMsg::Lobby { players, .. } => Some(
                    players
                        .iter()
                        .map(|p| if p.connected { p.name.clone() } else { format!("{} (offline)", p.name) })
                        .collect(),
                ),
                _ => None,
            })
            .expect("a lobby message")
    }

    #[test]
    fn a_dropped_lobby_seat_waits_for_its_player() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Hanabi, 2);
        let mut bob = clients.remove(1);
        let token = bob.token();

        // A reload: the old socket closes first, the seat stays, offline.
        let id = disconnect(&state, &bob.tx, bob.ident.take()).unwrap();
        assert_eq!(lobby_names(&clients[0].drain()), ["P0", "P1 (offline)"]);
        free_dropped_seat(&state, &id, Duration::from_secs(120));
        assert_eq!(state.lock().unwrap().map[&code].seats.len(), 2, "still within the grace period");

        // The new page reclaims it with the token, under whatever name.
        let mut back = Client::new();
        let r = back.send(&state, ClientMsg::Join { room: code.clone(), name: "P0".into(), token: Some(token) });
        assert_eq!(r, None);
        assert_eq!(lobby_names(&back.drain()), ["P0", "P1"]);

        // The first timer firing later doesn't free a seat that came back.
        free_dropped_seat(&state, &id, Duration::ZERO);
        assert_eq!(state.lock().unwrap().map[&code].seats.len(), 2);
    }

    #[test]
    fn a_lone_host_who_drops_keeps_the_room_open_until_the_grace_runs_out() {
        let state = new_state();
        let (mut host, code) = create(&state, GameKind::Hanabi, "Ann");
        let id = disconnect(&state, &host.tx, host.ident.take()).unwrap();

        // The room survives the host switching apps to send the code...
        let mut bob = join(&state, &code, "Bob").unwrap();
        assert_eq!(lobby_names(&bob.drain()), ["Ann (offline)", "Bob"]);

        // ...and once Ann's seat is given up, Bob hosts.
        free_dropped_seat(&state, &id, Duration::ZERO);
        match bob.drain().last().unwrap() {
            ServerMsg::Lobby { players, is_host, .. } => {
                assert_eq!(players.len(), 1);
                assert!(*is_host);
            }
            other => panic!("expected the lobby, got {other:?}"),
        }

        // Alone, a dropped host's room closes when the grace runs out.
        let (mut cy, code) = create(&state, GameKind::Catan, "Cy");
        let id = disconnect(&state, &cy.tx, cy.ident.take()).unwrap();
        free_dropped_seat(&state, &id, Duration::ZERO);
        assert!(!state.lock().unwrap().map.contains_key(&code));
    }

    #[test]
    fn a_running_game_keeps_dropped_seats_past_the_grace() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Hanabi, 2);
        clients[0].send(&state, ClientMsg::Start);
        let mut bob = clients.remove(1);
        let id = disconnect(&state, &bob.tx, bob.ident.take()).unwrap();
        free_dropped_seat(&state, &id, Duration::ZERO);
        assert_eq!(state.lock().unwrap().map[&code].seats.len(), 2);
    }

    #[test]
    fn a_dropped_player_gets_their_seat_back_with_the_token() {
        for game in GameKind::ALL {
            let state = new_state();
            let (code, mut clients) = lobby(&state, game, 2);
            clients[0].send(&state, ClientMsg::Start);
            let mut bob = clients.remove(1);
            let token = bob.token();
            let bob_name = "P1".to_string();
            bob.drain();
            clients[0].drain();

            // The connection drops: the seat stays, marked offline.
            let to_check = disconnect(&state, &bob.tx, bob.ident.take());
            assert_eq!(to_check.map(|id| id.room).as_deref(), Some(code.as_str()));
            let offline = clients[0].drain().pop().expect("the host is told");
            let ServerMsg::Game { connected, view, .. } = offline else {
                panic!("expected a state message, got {offline:?}");
            };
            assert_eq!(view.game(), game);
            assert_eq!(connected.iter().filter(|c| !**c).count(), 1);

            // A stranger can't take it, the token holder can.
            assert_eq!(join(&state, &code, "Mallory").err().as_deref(), Some("That game has already started."));
            let mut back = Client::new();
            let r = back.send(
                &state,
                ClientMsg::Join { room: code.clone(), name: bob_name.clone(), token: Some(token.clone()) },
            );
            assert_eq!(r, None);
            let msgs = back.drain();
            assert!(matches!(&msgs[0], ServerMsg::Joined { token: t, .. } if *t == token));
            assert!(matches!(msgs.last().unwrap(), ServerMsg::Game { view, .. } if view.game() == game));

            // Nobody left in the room: it is dropped once the timer fires.
            clients[0].send(&state, ClientMsg::Leave);
            back.send(&state, ClientMsg::Leave);
            remove_if_abandoned(&state, &code);
            assert!(!state.lock().unwrap().map.contains_key(&code));
        }
    }

    #[test]
    fn a_stale_disconnect_does_not_kick_a_reconnected_player() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Hanabi, 2);
        clients[0].send(&state, ClientMsg::Start);
        let old = clients.remove(1);
        let token = old.token();

        // Reconnects (new tab) before the old socket is noticed as closed.
        let mut fresh = Client::new();
        fresh.send(&state, ClientMsg::Join { room: code.clone(), name: "P1".into(), token: Some(token) });
        let stale = disconnect(&state, &old.tx, old.ident);
        assert!(stale.is_none());
        let rooms = state.lock().unwrap();
        assert!(rooms.map[&code].seats.iter().all(|s| s.tx.is_some()));
    }

    #[test]
    fn a_move_for_the_wrong_game_or_before_the_start_is_refused() {
        // One move of each game.
        let moves = [
            GameMove::Catan(catan_core::Action::EndTurn),
            GameMove::Hanabi(hanabi_core::Action::Play { card_id: hanabi_core::CardId(0) }),
            GameMove::Wonderful(wonderful_core::Action::Ready),
            GameMove::SmallWorld(smallworld_core::Action::EndConquests),
        ];
        for game in GameKind::ALL {
            let state = new_state();
            let (_code, mut clients) = lobby(&state, game, 2);
            for started in [false, true] {
                if started {
                    clients[0].send(&state, ClientMsg::Start);
                }
                for mv in &moves {
                    let r = clients[0].send(&state, ClientMsg::Move(mv.clone()));
                    let expected = if mv.game() != game {
                        Some(WRONG_GAME)
                    } else if !started {
                        Some("The game hasn't started yet.")
                    } else {
                        continue; // a move of the right game: up to the rules
                    };
                    assert_eq!(r.as_deref(), expected, "{mv:?} in a {game:?} room, started: {started}");
                }
            }
        }
    }

    #[test]
    fn options_are_the_hosts_and_only_for_a_game_that_has_them() {
        let hanabi_rules = GameOptions::Hanabi(hanabi_core::GameRules::default());
        for game in GameKind::ALL {
            let state = new_state();
            let (code, mut clients) = lobby(&state, game, 2);
            let defaults = games::game(game).default_options();
            assert_eq!(state.lock().unwrap().map[&code].options, defaults);
            // The game's own options, or some for a game that has none.
            let options = defaults.clone().unwrap_or(hanabi_rules.clone());
            let guest = clients[1].send(&state, ClientMsg::SetOptions(options.clone()));
            let host = clients[0].send(&state, ClientMsg::SetOptions(options));
            if defaults.is_some() {
                assert_eq!(guest.as_deref(), Some("Only the host can change the options."));
                assert_eq!(host, None);
            } else {
                assert_eq!(guest.as_deref(), Some("This game has no options to set."));
                assert_eq!(host.as_deref(), Some("This game has no options to set."));
            }
            // Another game's options are refused.
            if defaults.is_some() {
                let other = GameKind::ALL
                    .into_iter()
                    .filter(|&g| g != game)
                    .find_map(|g| games::game(g).default_options())
                    .unwrap();
                let r = clients[0].send(&state, ClientMsg::SetOptions(other));
                assert_eq!(r.as_deref(), Some("Those options are for another game."));
            }
        }
    }

    #[test]
    fn every_game_starts_and_shows_every_seat_its_view() {
        for game in GameKind::ALL {
            // A full table under the lobby's first options.
            let seats = game.max_players_with(games::game(game).default_options().as_ref());
            let state = new_state();
            let (code, mut clients) = lobby(&state, game, seats);
            assert_eq!(join(&state, &code, "Late").err().as_deref(), Some("That room is full."));
            assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
            for c in &mut clients {
                let (view, names, connected) = c.latest_game().expect("a view for every seat");
                assert_eq!(view.game(), game);
                assert_eq!(names.len(), seats);
                assert!(connected.iter().all(|&c| c));
            }
        }
    }

    #[test]
    fn an_empty_name_never_creates_a_room() {
        let state = new_state();
        let mut c = Client::new();
        assert_eq!(
            c.send(&state, ClientMsg::Create { game: GameKind::Hanabi, name: "   ".into() }).as_deref(),
            Some("Please enter a name.")
        );
        assert!(state.lock().unwrap().map.is_empty());
    }
}
