//! The screens before the game: your name and the room code, then the
//! waiting room.

use crate::app::{create_room, join_room, leave, send, App, Screen};
use catan_core::{Layout, Options};
use leptos::prelude::*;
use protocol::{ClientMsg, GameKind, GameOptions, LobbyPlayer};

pub fn home_screen(app: App) -> impl IntoView {
    let create = move |_| create_room(app, GameKind::Catan);
    let join = move |_| join_room(app);

    view! {
        <div class="home card">
            <h1>"Catan"</h1>
            <p class="muted">"Trade, build and settle the island. 2–4 players, play in your browser."</p>
            <label>"Your name"
                <input type="text" maxlength="20" placeholder="e.g. Ada"
                    prop:value=move || app.name.get()
                    on:input=move |ev| app.name.set(event_target_value(&ev)) />
            </label>
            <button class="primary" on:click=create disabled=move || !app.online.get()>"Create a room"</button>
            <div class="or">"or join a friend"</div>
            <label>"Room code"
                <input type="text" maxlength="4" placeholder="ABCD" class="code-input"
                    prop:value=move || app.room_input.get()
                    on:input=move |ev| app.room_input.set(event_target_value(&ev).to_uppercase()) />
            </label>
            <button on:click=join disabled=move || !app.online.get()>"Join room"</button>
            <button class="link" on:click=move |_| app.screen.set(Screen::Menu)>"← All games"</button>
        </div>
    }
}

/// The island's picker: a random one, or the rulebook's beginner map. The
/// host picks, everybody sees it (locked for the others). As in the other
/// games' lobbies, a pick only asks the server, whose echo is what the
/// picker shows.
fn layout_option(app: App, is_host: bool) -> impl IntoView {
    let options = app.games.catan.options;
    let pick = move |ev: leptos::ev::Event| {
        let layout = if event_target_value(&ev) == "beginner" { Layout::Beginner } else { Layout::Random };
        send(app, &ClientMsg::SetOptions(GameOptions::Catan(Options { layout })));
    };
    let choice = move |layout: Layout, value: &'static str, label: &'static str| {
        view! { <option value=value prop:selected=move || options.get().layout == layout>{label}</option> }
    };
    let hint = move || match options.get().layout {
        Layout::Random => "Tiles, numbers and harbors are shuffled for every game.",
        Layout::Beginner => {
            "The rulebook's starting map for beginners: the same tiles, numbers and harbors every game. You still place your own first settlements and roads."
        }
    };
    view! {
        <div class="option">
            <label>"Board"
                <select disabled=!is_host on:change=pick>
                    {choice(Layout::Random, "random", "Random")}
                    {choice(Layout::Beginner, "beginner", "Beginner (the rulebook's map)")}
                </select>
            </label>
            <p class="muted small-text">{hint}</p>
        </div>
    }
}

pub fn lobby_screen(app: App, room: String, players: Vec<LobbyPlayer>, is_host: bool) -> impl IntoView {
    let can_start = players.len() >= 2;
    let rows = players
        .iter()
        .enumerate()
        .map(|(i, p)| {
            view! {
                <li>
                    <span class="swatch" style=format!("background:{}", super::map::PLAYER_COLORS[i % 4]) />
                    {p.name.clone()}
                    {(i == 0).then(|| view! { <span class="badge">"host"</span> })}
                    {(!p.connected).then(|| view! { <span class="badge off">"offline"</span> })}
                </li>
            }
        })
        .collect_view();

    view! {
        <div class="home card">
            <h2>"Waiting room"</h2>
            <p class="muted">"Share this code with your friends:"</p>
            <div class="room-code">{room}</div>
            <ul class="lobby-list">{rows}</ul>
            {layout_option(app, is_host)}
            {if is_host {
                view! {
                    <button class="primary" disabled=!can_start on:click=move |_| send(app, &ClientMsg::Start)>
                        {if can_start { "Start game" } else { "Need at least 2 players" }}
                    </button>
                }.into_any()
            } else {
                view! { <p class="muted">"Waiting for the host to start…"</p> }.into_any()
            }}
            <button class="link" on:click=move |_| leave(app)>"Leave room"</button>
        </div>
    }
}

fn hex_points(cx: f64, cy: f64, r: f64) -> String {
    let h = r * 0.866;
    [
        (cx, cy - r),
        (cx + h, cy - r / 2.0),
        (cx + h, cy + r / 2.0),
        (cx, cy + r),
        (cx - h, cy + r / 2.0),
        (cx - h, cy - r / 2.0),
    ]
    .iter()
    .map(|(x, y)| format!("{x:.2},{y:.2}"))
    .collect::<Vec<_>>()
    .join(" ")
}

/// The menu card's picture: three terrain hexes (forest, hills, fields).
pub fn icon() -> impl IntoView {
    view! {
        <svg viewBox="0 0 64 64" role="img" aria-label="Catan">
            <polygon points=hex_points(21.0, 24.0, 12.0) fill="#2d6a4f" stroke="#fdf6e3" stroke-width="1.5" />
            <polygon points=hex_points(43.0, 24.0, 12.0) fill="#c1553b" stroke="#fdf6e3" stroke-width="1.5" />
            <polygon points=hex_points(32.0, 43.0, 12.0) fill="#e0b94f" stroke="#fdf6e3" stroke-width="1.5" />
            <circle cx="32" cy="43" r="5" fill="#f6efdd" />
            <text x="32" y="46" text-anchor="middle" font-size="8" font-weight="700" fill="#c1121f">"8"</text>
        </svg>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::players;
    use leptos::tachys::view::RenderHtml;

    #[test]
    fn home_and_lobby_render() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            assert!(home_screen(app).to_html().contains("Create a room"));
            let mut seats = players(&["Ann", "Bob"]);
            seats[1].connected = false;
            let html = lobby_screen(app, "WXYZ".into(), seats, true).to_html();
            assert!(html.contains("WXYZ") && html.contains("Start game"));
            assert!(html.contains("offline"));
            let alone = lobby_screen(app, "WXYZ".into(), players(&["Ann"]), true).to_html();
            assert!(alone.contains("Need at least 2 players"));
        });
    }

    #[test]
    fn the_lobby_shows_the_island_the_host_picked() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let catan = crate::games::game(GameKind::Catan);
            // A random island until the server says otherwise.
            let host = lobby_screen(app, "WXYZ".into(), players(&["Ann", "Bob"]), true).to_html();
            assert!(host.contains("Board") && host.contains("Beginner (the rulebook"));
            assert!(host.contains("shuffled for every game"));
            assert!(!host.contains("disabled"), "the host can pick");
            let guest = lobby_screen(app, "WXYZ".into(), players(&["Ann", "Bob"]), false).to_html();
            assert!(guest.contains("disabled"), "only the host picks");

            catan.options(app, GameOptions::Catan(Options { layout: Layout::Beginner }));
            assert_eq!(app.games.catan.options.get_untracked().layout, Layout::Beginner);
            let html = lobby_screen(app, "WXYZ".into(), players(&["Ann", "Bob"]), false).to_html();
            assert!(html.contains("starting map for beginners"));
            assert_eq!(catan.snapshot(app)["options"]["layout"], "Beginner");
            // Leaving the room forgets the pick.
            catan.reset(app);
            assert_eq!(app.games.catan.options.get_untracked(), Options::default());
        });
    }
}
