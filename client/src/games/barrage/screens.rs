//! The Barrage screens around the board: the page frame, create/join, and
//! the waiting room.

use crate::app::{create_room, join_room, leave, send, App, Screen};
use leptos::prelude::*;
use protocol::{ClientMsg, GameKind, LobbyPlayer};

/// The frame every screen of the game sits in: its own look (see
/// `barrage.css`, everything in it is scoped under `.barrage`).
pub fn shell(content: impl IntoView + 'static) -> impl IntoView {
    view! {
        <div class="barrage">
            <main class="app">{content}</main>
        </div>
    }
}

/// Name + create a room, or join one with its code.
pub fn home_screen(app: App) -> impl IntoView {
    let create = move |_| create_room(app, GameKind::Barrage);
    let join = move |_| join_room(app);

    view! {
        <div class="entry">
            <div class="mark" aria-hidden="true">{icon()}</div>
            <h1 class="game-title">"Barrage"</h1>
            <p class="lede">
                "Run an energy company in the Alps of the 1920s: build dams, conduits and powerhouses, hold back the water that runs down from the headstreams and turn it into energy to fulfill contracts. Five rounds, 2 to 4 players."
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
    let seated = players.len();
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
        let hint = if seated >= GameKind::Barrage.min_players() {
            "Seats are drawn when you start: the first seat picks its company first, and plays last."
        } else {
            "Needs at least 2 players."
        };
        view! {
            <button
                class="primary"
                disabled={seated < GameKind::Barrage.min_players()}
                on:click=move |_| send(app, &ClientMsg::Start)
            >
                "Start game"
            </button>
            <p class="hint">{hint}</p>
        }
        .into_any()
    } else {
        view! { <p class="hint">"Waiting for the host to start the game…"</p> }.into_any()
    };

    view! {
        <div class="entry">
            <div class="mark" aria-hidden="true">{icon()}</div>
            <h1 class="game-title">"Barrage"</h1>
            <div class="panel">
                <h2>"Waiting for players"</h2>
                <p class="hint">"Share this code with everyone you're playing with:"</p>
                <div class="room-code">{room}</div>
                <ul class="roster">{rows}</ul>
                <p class="hint">"The full game: Executive Officers, the Patent Office and its Advanced Technology tiles, the National Contracts."</p>
                {start}
                <button class="link" on:click=move |_| leave(app)>
                    "Leave room"
                </button>
            </div>
        </div>
    }
}

/// The menu card's picture: a dam holding back a lake, with a conduit down
/// to a powerhouse.
pub fn icon() -> impl IntoView {
    view! {
        <svg viewBox="0 0 64 64" role="img" aria-label="Barrage">
            <rect x="4" y="4" width="56" height="56" rx="12" fill="#2a2f36" />
            <path d="M8 26 Q20 18 32 24 Q44 30 56 22 L56 34 L8 34 Z" fill="#3f86c6" />
            <path d="M14 34 L50 34 L46 42 L18 42 Z" fill="#c9b48a" stroke="#1d1a16" stroke-width="1.2" />
            <path d="M22 42 L22 52" stroke="#e3c429" stroke-width="3" />
            <rect x="15" y="50" width="14" height="9" rx="2" fill="#c8323c" stroke="#1d1a16" stroke-width="1" />
            <path d="M42 14 l-5 8 h4 l-3 7 l7 -10 h-4 l3 -5 Z" fill="#e8c169" />
        </svg>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::players;
    use leptos::tachys::view::RenderHtml;

    #[test]
    fn the_barrage_lobby_lets_only_the_host_start() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let seats = players(&["Ann", "Bob", "Cy"]);
            let host = lobby_screen(app, "WXYZ".into(), seats.clone(), 0, true).to_html();
            assert!(host.contains("WXYZ") && host.contains("Start game") && host.contains("picks its company first"));
            assert!(!host.contains("disabled"));
            let alone = lobby_screen(app, "WXYZ".into(), players(&["Ann"]), 0, true).to_html();
            assert!(alone.contains("Needs at least 2 players") && alone.contains("disabled"));
            let guest = lobby_screen(app, "WXYZ".into(), seats, 2, false).to_html();
            assert!(guest.contains("Waiting for the host to start the game") && !guest.contains("Start game"));
        });
    }

    #[test]
    fn the_home_screen_creates_or_joins() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let html = shell(home_screen(app)).to_html();
            assert!(html.contains("class=\"barrage\""));
            assert!(html.contains("Create a room") && html.contains("Join room") && html.contains("All games"));
        });
    }
}
