//! The It's a Wonderful World screens around the board: the page frame,
//! create/join, and the waiting room.

use super::look;
use crate::app::{create_room, join_room, leave, send, App, Screen};
use leptos::prelude::*;
use protocol::{ClientMsg, GameKind, GameOptions, LobbyPlayer};
use wonderful_core::{Options, Res};

/// The frame every screen of the game sits in: its own look (see
/// `wonderful.css`, everything in it is scoped under `.wonderful`).
pub fn shell(content: impl IntoView + 'static) -> impl IntoView {
    view! {
        <div class="wonderful">
            <main class="app">{content}</main>
        </div>
    }
}

/// The five resources as cubes: the game's mark.
fn mark() -> impl IntoView {
    let cubes = Res::ALL
        .iter()
        .map(|&res| {
            view! { <span class=format!("cube {}", look::res_class(res))>{super::board::kind_icon(look::res_kind(res))}</span> }
        })
        .collect_view();
    view! { <div class="mark" aria-hidden="true">{cubes}</div> }
}

/// Name + create a room, or join one with its code.
pub fn home_screen(app: App) -> impl IntoView {
    let create = move |_| create_room(app, GameKind::Wonderful);
    let join = move |_| join_room(app);

    view! {
        <div class="entry">
            {mark()}
            <h1 class="game-title">"It's a Wonderful World"</h1>
            <p class="lede">
                "Draft cards, build an empire and out-produce your rivals over four rounds. For 2 to 5 players (7 with the Corruption & Ascension expansion), and everybody plays at once."
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

/// The expansion's switch. The host ticks it, everybody sees it (locked for
/// the others). As with Hanabi's rules, a click only asks the server, whose
/// echo is what moves the checkbox. With more than five seated it can't be
/// turned off.
fn expansion_option(app: App, is_host: bool, seated: usize) -> impl IntoView {
    let options = app.games.wonderful.options;
    let toggle = move |_| {
        let ascension = !options.get_untracked().ascension;
        send(app, &ClientMsg::SetOptions(GameOptions::Wonderful(Options { ascension })));
    };
    let locked = move || !is_host || (options.get().ascension && seated > Options::default().max_players());
    view! {
        <div class="option">
            <label class="option-toggle">
                <input
                    type="checkbox"
                    prop:checked=move || options.get().ascension
                    disabled=locked
                    on:change=toggle
                />
                <span>"Corruption & Ascension"</span>
            </label>
            <p class="hint">
                "The expansion: a second deck of 56 cards dealt beside the first, with corruption that eats into your production and points for duos, and two more Empires, for up to 7 players."
            </p>
        </div>
    }
}

/// The waiting room: who is here, the expansion's switch, and the host's
/// start button.
pub fn lobby_screen(
    app: App,
    room: String,
    players: Vec<LobbyPlayer>,
    you: usize,
    is_host: bool,
) -> impl IntoView {
    let can_start = players.len() >= GameKind::Wonderful.min_players();
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
                    "Everyone who's here is in. Seats are drawn when you start."
                } else {
                    "Needs at least 2 players."
                }}
            </p>
        }
        .into_any()
    } else {
        view! { <p class="hint">"Waiting for the host to start the game…"</p> }.into_any()
    };

    view! {
        <div class="entry">
            {mark()}
            <h1 class="game-title">"It's a Wonderful World"</h1>
            <div class="panel">
                <h2>"Waiting for players"</h2>
                <p class="hint">"Share this code with everyone you're playing with:"</p>
                <div class="room-code">{room}</div>
                <ul class="roster">{rows}</ul>
                {expansion_option(app, is_host, players.len())}
                {start}
                <button class="link" on:click=move |_| leave(app)>
                    "Leave room"
                </button>
            </div>
        </div>
    }
}

/// The menu card's picture: a card with a planet on it and the five
/// resource cubes along its foot.
pub fn icon() -> impl IntoView {
    let cube = |x: f64, fill: &'static str| {
        view! { <rect x=x y="42" width="5" height="5" rx="1.2" fill=fill stroke="#0b2a2a" stroke-width="0.6" /> }
    };
    view! {
        <svg viewBox="0 0 64 64" role="img" aria-label="It's a Wonderful World">
            <g transform="rotate(-6 32 32)">
                <rect x="14" y="8" width="36" height="48" rx="4" fill="#eef1ea" stroke="#0b2a2a" stroke-width="1.5" />
                <path d="M14 12a4 4 0 0 1 4-4h28a4 4 0 0 1 4 4v6H14z" fill="#1f9d8f" />
                <circle cx="32" cy="31" r="8" fill="#ff6b8b" />
                <ellipse cx="32" cy="31" rx="13" ry="3.4" fill="none" stroke="#0b2a2a" stroke-width="1.4" transform="rotate(-18 32 31)" />
                {cube(16.5, "#9aa5b1")}
                {cube(23.0, "#2b2f33")}
                {cube(29.5, "#3fb86e")}
                {cube(36.0, "#f2c230")}
                {cube(42.5, "#3b82e0")}
            </g>
        </svg>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{players, shows};
    use leptos::tachys::view::RenderHtml;

    #[test]
    fn wonderful_lobby_lets_only_the_host_start() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let seats = players(&["Ann", "Bob", "Cy"]);
            let host = lobby_screen(app, "WXYZ".into(), seats.clone(), 0, true).to_html();
            assert!(host.contains("WXYZ") && host.contains("Start game"));
            assert!(host.contains("Ann") && host.contains("(you)") && host.contains("host"));
            assert!(host.contains("Leave room"));
            // With three players the start button is enabled.
            assert!(shows(&host, "Everyone who's here is in"));
            assert!(!host.contains("Needs at least 2 players") && !host.contains("disabled"));

            // Alone, the host can't start yet.
            let alone = lobby_screen(app, "WXYZ".into(), players(&["Ann"]), 0, true).to_html();
            assert!(alone.contains("Needs at least 2 players") && alone.contains("disabled"));

            // A guest waits for the host.
            let guest = lobby_screen(app, "WXYZ".into(), seats, 2, false).to_html();
            assert!(guest.contains("Cy") && guest.contains("(you)"));
            assert!(guest.contains("Waiting for the host to start the game"));
            assert!(!guest.contains("Start game"));

            // A seat whose connection dropped says so.
            let mut seats = players(&["Ann", "Bob"]);
            seats[1].connected = false;
            let html = lobby_screen(app, "WXYZ".into(), seats, 0, true).to_html();
            assert!(html.contains("offline"));
        });
    }

    #[test]
    fn the_host_switches_the_expansion_on_and_everyone_sees_it() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let seats = players(&["Ann", "Bob", "Cy"]);
            let host = lobby_screen(app, "WXYZ".into(), seats.clone(), 0, true).to_html();
            assert!(host.contains("Corruption &amp; Ascension"), "{host}");
            assert!(host.contains("type=\"checkbox\"") && !host.contains("disabled"), "the host can tick it");
            // The others see it, locked.
            let guest = lobby_screen(app, "WXYZ".into(), seats, 1, false).to_html();
            assert!(guest.contains("type=\"checkbox\"") && guest.contains("disabled"), "{guest}");
            // Once on with six seated, it can't be turned off.
            app.games.wonderful.options.set(Options { ascension: true });
            let six = players(&["A", "B", "C", "D", "E", "F"]);
            let full = lobby_screen(app, "WXYZ".into(), six, 0, true).to_html();
            assert_eq!(full.matches("disabled").count(), 1, "the switch, not the start button: {full}");
        });
    }
}
