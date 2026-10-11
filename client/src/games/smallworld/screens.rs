//! The Small World screens around the board: the page frame, create/join,
//! and the waiting room.

use crate::app::{create_room, join_room, leave, send, App, Screen};
use leptos::prelude::*;
use protocol::{ClientMsg, GameKind, LobbyPlayer};

/// The frame every screen of the game sits in: its own look (see
/// `smallworld.css`, everything in it is scoped under `.smallworld`).
pub fn shell(content: impl IntoView + 'static) -> impl IntoView {
    view! {
        <div class="smallworld">
            <main class="app">{content}</main>
        </div>
    }
}

/// Name + create a room, or join one with its code.
pub fn home_screen(app: App) -> impl IntoView {
    let create = move |_| create_room(app, GameKind::SmallWorld);
    let join = move |_| join_room(app);

    view! {
        <div class="entry">
            <div class="mark" aria-hidden="true">{icon()}</div>
            <h1 class="game-title">"Small World"</h1>
            <p class="lede">
                "Lead fantasy races to conquer a world too small for all of them. When your empire runs out of steam, send it into decline and take up a new race. For 2 to 5 players."
            </p>
            <div class="panel join-form">
                <label>
                    "Your name"
                    <input
                        type="text"
                        maxlength="20"
                        prop:value=move || app.name.get()
                        on:input=move |ev| app.name.set(event_target_value(&ev))
                    />
                </label>
                <button class="primary" on:click=create disabled=move || !app.online.get()>
                    "Create a room"
                </button>
                <p class="hint">"or join a friend's room with its code"</p>
                <label>
                    "Room code"
                    <input
                        type="text"
                        maxlength="4"
                        placeholder="ABCD"
                        class="code-input"
                        prop:value=move || app.room_input.get()
                        on:input=move |ev| app.room_input.set(event_target_value(&ev).to_uppercase())
                    />
                </label>
                <button on:click=join disabled=move || !app.online.get()>
                    "Join room"
                </button>
                <button class="link" on:click=move |_| app.screen.set(Screen::Menu)>
                    "← All games"
                </button>
            </div>
        </div>
    }
}

/// The waiting room: who is here, and the host's start button.
pub fn lobby_screen(app: App, room: String, players: Vec<LobbyPlayer>, you: usize, is_host: bool) -> impl IntoView {
    let can_start = players.len() >= GameKind::SmallWorld.min_players();
    let board = players.len().clamp(GameKind::SmallWorld.min_players(), GameKind::SmallWorld.max_players());
    let rows = players
        .iter()
        .enumerate()
        .map(|(i, p)| {
            view! {
                <li>
                    <span class="seat-name">{p.name.clone()}{(i == you).then_some(" (you)")}</span>
                    {(i == 0).then(|| view! { <span class="tag">"host"</span> })}
                    {(!p.connected).then(|| view! { <span class="tag tag-off">"offline"</span> })}
                </li>
            }
        })
        .collect_view();

    let start = if is_host {
        view! {
            <button class="primary" disabled=!can_start on:click=move |_| send(app, &ClientMsg::Start)>
                "Start game"
            </button>
            <p class="hint">
                {if can_start {
                    format!(
                        "You'll play on the {board}-player board, for {} turns. Seats are drawn when you start.",
                        smallworld_core::Map::for_players(board).turns,
                    )
                } else {
                    "Needs at least 2 players.".to_string()
                }}
            </p>
        }
        .into_any()
    } else {
        view! { <p class="hint">"Waiting for the host to start the game…"</p> }.into_any()
    };

    view! {
        <div class="entry">
            <div class="mark" aria-hidden="true">{icon()}</div>
            <h1 class="game-title">"Small World"</h1>
            <div class="panel">
                <h2>"Waiting for players"</h2>
                <p class="hint">"Share this code with everyone you're playing with:"</p>
                <div class="room-code">{room}</div>
                <ul class="roster">{rows}</ul>
                {start}
                <button class="link" on:click=move |_| leave(app)>
                    "Leave room"
                </button>
            </div>
        </div>
    }
}

/// The menu card's picture: a little world of four regions, with a
/// mountain and two armies' tokens.
pub fn icon() -> impl IntoView {
    view! {
        <svg viewBox="0 0 64 64" role="img" aria-label="Small World">
            <circle cx="32" cy="32" r="27" fill="#3f7cc0" stroke="#2b2116" stroke-width="1.5" />
            <path d="M14 38 Q12 24 24 17 Q33 12 40 17 L34 30 Z" fill="#9cc65a" stroke="#2b2116" stroke-width="1.2" />
            <path d="M40 17 Q52 22 51 34 L34 30 Z" fill="#e7c25c" stroke="#2b2116" stroke-width="1.2" />
            <path d="M51 34 Q49 46 37 50 L34 30 Z" fill="#2f6a39" stroke="#2b2116" stroke-width="1.2" />
            <path d="M37 50 Q22 52 14 38 L34 30 Z" fill="#7d6d3c" stroke="#2b2116" stroke-width="1.2" />
            <path d="M38 27 L43 19 L48 27 Z" fill="#eef1f5" stroke="#2b2116" stroke-width="1" />
            <rect x="17" y="24" width="12" height="8" rx="2.5" fill="#d6453a" stroke="#2b2116" stroke-width="1" />
            <rect x="35" y="38" width="12" height="8" rx="2.5" fill="#2f78d4" stroke="#2b2116" stroke-width="1" />
        </svg>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{players, shows};
    use leptos::tachys::view::RenderHtml;

    #[test]
    fn the_small_world_lobby_names_the_board_and_lets_only_the_host_start() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let seats = players(&["Ann", "Bob", "Cy"]);
            let host = lobby_screen(app, "WXYZ".into(), seats.clone(), 0, true).to_html();
            assert!(host.contains("WXYZ") && host.contains("Start game"));
            assert!(host.contains("Ann") && host.contains("(you)") && host.contains("host"));
            assert!(shows(&host, "You'll play on the 3-player board, for 10 turns."), "{host}");
            assert!(!host.contains("disabled"));

            let alone = lobby_screen(app, "WXYZ".into(), players(&["Ann"]), 0, true).to_html();
            assert!(alone.contains("Needs at least 2 players") && alone.contains("disabled"));

            let guest = lobby_screen(app, "WXYZ".into(), seats, 2, false).to_html();
            assert!(guest.contains("Cy") && guest.contains("Waiting for the host to start the game"));
            assert!(!guest.contains("Start game"));

            let five = players(&["A", "B", "C", "D", "E"]);
            let html = lobby_screen(app, "WXYZ".into(), five, 0, true).to_html();
            assert!(shows(&html, "the 5-player board, for 8 turns"));
        });
    }

    #[test]
    fn the_home_screen_creates_or_joins() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let html = shell(home_screen(app)).to_html();
            assert!(html.contains("class=\"smallworld\""));
            assert!(html.contains("Create a room") && html.contains("Join room") && html.contains("All games"));
        });
    }
}
