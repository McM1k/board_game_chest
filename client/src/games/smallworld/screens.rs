//! The Small World screens around the board: the page frame, create/join,
//! and the waiting room.

use crate::app::{create_room, join_room, leave, send, App, Screen};
use leptos::prelude::*;
use protocol::{ClientMsg, GameKind, GameOptions, LobbyPlayer};
use smallworld_core::{turns, Options, Pack, Sky};

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
                "Lead fantasy races to conquer a world too small for all of them. When your empire runs out of steam, send it into decline and take up a new race. For 2 to 5 players, or 6 with the Sky Islands."
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

/// What each expansion brings, for the lobby.
fn pack_hint(pack: Pack) -> &'static str {
    match pack {
        Pack::Base => "",
        Pack::Cursed => "Goblins and Kobolds; Cursed, Hordes of, Marauding, Ransacking and Were- powers.",
        Pack::GrandDames => "Gypsies, Priestesses and White Ladies; Historian and Peace-Loving.",
        Pack::BeNotAfraid => "Barbarians, Homunculi, Leprechauns, Pixies and Pygmies; Barricade, Catapult, Corrupt, Imperial and Mercenary.",
        Pack::RoyalBonus => "Fauns, Igors and Shrubmen; Aquatic, Behemoth and Fireball.",
        Pack::SpidersWeb => "Ice Witches, Skags and Slingmen; Copycat, Lava and Soul-Touch.",
        Pack::SkyIslands => "Islands above the board for one player fewer, reached by a beanstalk and a stairway; 7 races and 7 powers; 3 to 6 players.",
    }
}

/// The expansions' switches. The host ticks them, everybody sees them
/// (locked for the others). As with Hanabi's rules, a click only asks the
/// server, whose echo is what moves the checkboxes. With six seated, the
/// Sky Islands can't be switched off.
fn expansion_options(app: App, is_host: bool, seated: usize) -> impl IntoView {
    let options = app.games.smallworld.options;
    let set = move |options: Options| send(app, &ClientMsg::SetOptions(GameOptions::SmallWorld(options)));
    let packs = Pack::EXPANSIONS
        .into_iter()
        .map(|pack| {
            let toggle = move |_| {
                let mut o = options.get_untracked();
                o.set(pack, !o.has(pack));
                set(o);
            };
            let locked = move || {
                !is_host || (pack == Pack::SkyIslands && options.get().has(pack) && seated > Options::default().max_players())
            };
            view! {
                <li class="option">
                    <label class="option-toggle">
                        <input type="checkbox" prop:checked=move || options.get().has(pack) disabled=locked on:change=toggle />
                        <span>{pack.name()}</span>
                    </label>
                    <p class="hint">{pack_hint(pack)}</p>
                </li>
            }
        })
        .collect_view();
    let side = move |sky: Sky| {
        view! {
            <label class="side">
                <input
                    type="radio"
                    name="sky-side"
                    prop:checked=move || options.get().sky == Some(sky)
                    disabled=move || !is_host
                    on:change=move |_| set(Options { sky: Some(sky), ..options.get_untracked() })
                />
                <span>{sky.name()}</span>
            </label>
        }
    };
    view! {
        <div class="expansions">
            <h3>"Expansions"</h3>
            <ul>{packs}</ul>
            {move || options.get().sky.is_some().then(|| view! {
                <div class="sides" role="group" aria-label="Side of the Sky Islands board">
                    {side(Sky::Three)}
                    {side(Sky::Two)}
                </div>
            })}
        </div>
    }
}

/// What will be played, as the start button's hint says it.
fn board_line(options: Options, seated: usize) -> String {
    let sky = options.sky.map_or(String::new(), |s| format!(" and the Sky Islands ({})", s.name().to_lowercase()));
    let ground = if options.sky.is_some() { seated - 1 } else { seated };
    format!("You'll play on the {ground}-player board{sky}, for {} turns. Seats are drawn when you start.", turns(seated))
}

/// The waiting room: who is here, the expansions, and the host's start
/// button.
pub fn lobby_screen(app: App, room: String, players: Vec<LobbyPlayer>, you: usize, is_host: bool) -> impl IntoView {
    let seated = players.len();
    let options = app.games.smallworld.options;
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
            <button
                class="primary"
                disabled=move || seated < options.get().min_players().max(GameKind::SmallWorld.min_players())
                on:click=move |_| send(app, &ClientMsg::Start)
            >
                "Start game"
            </button>
            <p class="hint">
                {move || {
                    let o = options.get();
                    if seated >= o.min_players() {
                        board_line(o, seated)
                    } else if o.sky.is_some() {
                        "Needs at least 3 players with the Sky Islands.".to_string()
                    } else {
                        "Needs at least 2 players.".to_string()
                    }
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
                {expansion_options(app, is_host, seated)}
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
    fn the_host_picks_the_expansions_and_everyone_sees_them() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let seats = players(&["Ann", "Bob"]);
            let host = lobby_screen(app, "WXYZ".into(), seats.clone(), 0, true).to_html();
            assert_eq!(host.matches("type=\"checkbox\"").count(), 6);
            for pack in Pack::EXPANSIONS {
                assert!(shows(&host, pack.name()), "{pack:?}");
            }
            assert!(!host.contains("disabled"), "the host can tick them all");
            assert!(!host.contains("Two islands"), "no side to pick yet");
            let guest = lobby_screen(app, "WXYZ".into(), seats.clone(), 1, false).to_html();
            assert_eq!(guest.matches("disabled").count(), 6, "locked for the others");

            // The Sky Islands: three players at least, a side to pick.
            app.games.smallworld.options.set(Options::all());
            let host = lobby_screen(app, "WXYZ".into(), seats, 0, true).to_html();
            assert!(host.contains("Three islands") && host.contains("Two islands"));
            assert!(host.contains("Needs at least 3 players with the Sky Islands"));
            let six = players(&["A", "B", "C", "D", "E", "F"]);
            let html = lobby_screen(app, "WXYZ".into(), six, 0, true).to_html();
            assert!(shows(&html, "the 5-player board and the Sky Islands (three islands), for 8 turns"), "{html}");
            assert_eq!(html.matches("disabled").count(), 1, "the Sky Islands can't go with six seated");
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
