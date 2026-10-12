//! The report button: round, red, with an exclamation mark, in the bottom
//! left corner of every screen. It asks for a bug or an idea and sends the
//! answer to the server's `POST /report` with where the player was: the
//! screen, the game, the room, the page, the browser, the last error shown
//! and what the screen held (the waiting room, or the game as their seat
//! sees it). The server saves it in the reports folder for an agent to pick
//! up (see `reports/README.md`).
//!
//! The report goes over HTTP rather than the game's socket, which carries
//! small messages only and may be the very thing that broke.

use crate::app::{load, App, Screen, KEY_TOKEN};
use crate::games;
use leptos::prelude::*;
use protocol::Report;

/// The longest message the box takes (the server keeps as much).
pub const MAX_MESSAGE: usize = 5_000;

/// Where the dialog is at. The server's answer (`Sent`, `Failed`) only
/// arrives in the browser; native builds just draw them.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub enum Status {
    Writing,
    Sending,
    /// Saved, in this file.
    Sent(String),
    /// Not saved, and why.
    Failed(String),
}

/// The dialog's state, which outlives the screens behind it.
#[derive(Clone, Copy)]
pub struct Signals {
    pub open: RwSignal<bool>,
    /// What the player has written so far (kept if they close the dialog).
    pub text: RwSignal<String>,
    pub status: RwSignal<Status>,
}

impl Signals {
    pub fn new() -> Signals {
        Signals { open: RwSignal::new(false), text: RwSignal::new(String::new()), status: RwSignal::new(Status::Writing) }
    }
}

impl Default for Signals {
    fn default() -> Self {
        Signals::new()
    }
}

/// The page's address, the browser, and the window's size.
#[cfg(target_arch = "wasm32")]
fn page() -> (String, String, String) {
    let Some(window) = web_sys::window() else { return Default::default() };
    let url = window.location().href().unwrap_or_default();
    let browser = window.navigator().user_agent().unwrap_or_default();
    let size = |v: Result<wasm_bindgen::JsValue, _>| v.ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
    let viewport = format!("{}x{}", size(window.inner_width()), size(window.inner_height()));
    (url, browser, viewport)
}

// Native builds only exist for `cargo check` and render tests.
#[cfg(not(target_arch = "wasm32"))]
fn page() -> (String, String, String) {
    Default::default()
}

/// The report for `message`, with where the player is and what their
/// screen holds.
pub fn build(app: App, message: String) -> Report {
    let (screen, game, state) = app.screen.with_untracked(|s| match s {
        Screen::Menu => ("Menu", None, None),
        Screen::Home(kind) => (
            "Home",
            Some(*kind),
            Some(serde_json::json!({
                "name": app.name.get_untracked(),
                "room_code_typed": app.room_input.get_untracked(),
            })),
        ),
        Screen::Lobby(lobby) => (
            "Lobby",
            Some(lobby.game),
            Some(serde_json::json!({
                "lobby": {
                    "room": lobby.room,
                    "players": lobby.players,
                    "you": lobby.you,
                    "is_host": lobby.is_host,
                },
                "game": games::game(lobby.game).snapshot(app),
            })),
        ),
        Screen::Playing(kind) => ("Game", Some(*kind), Some(games::game(*kind).snapshot(app))),
    });
    let state = state.map(|mut s| {
        s["online"] = app.online.get_untracked().into();
        s.to_string()
    });
    let room = Some(app.room.get_untracked()).filter(|r| !r.is_empty());
    let (url, browser, viewport) = page();
    Report {
        message: message.trim().chars().take(MAX_MESSAGE).collect(),
        screen: screen.to_string(),
        game,
        token: room.as_ref().and_then(|_| load(KEY_TOKEN)),
        room,
        url,
        browser,
        viewport,
        last_error: app.last_error.get_untracked(),
        state,
    }
}

/// Posts a report; the file it was saved in, or what went wrong.
#[cfg(target_arch = "wasm32")]
async fn post(report: &Report) -> Result<protocol::ReportSaved, String> {
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Headers, Request, RequestInit, Response};

    let unreachable = |_| "The server couldn't be reached. Try again in a moment.".to_string();
    let body = serde_json::to_string(report).map_err(|e| e.to_string())?;
    let headers = Headers::new().map_err(unreachable)?;
    headers.set("Content-Type", "application/json").map_err(unreachable)?;
    let init = RequestInit::new();
    init.set_method("POST");
    init.set_headers(&headers);
    init.set_body(&JsValue::from_str(&body));
    let request = Request::new_with_str_and_init("/report", &init).map_err(unreachable)?;
    let window = web_sys::window().ok_or("No browser window.")?;
    let response: Response =
        JsFuture::from(window.fetch_with_request(&request)).await.map_err(unreachable)?.dyn_into().map_err(unreachable)?;
    let text = JsFuture::from(response.text().map_err(unreachable)?).await.map_err(unreachable)?.as_string().unwrap_or_default();
    if response.ok() {
        serde_json::from_str::<protocol::ReportSaved>(&text).map_err(|e| e.to_string())
    } else if text.is_empty() {
        Err(format!("The server answered {}.", response.status()))
    } else {
        Err(text)
    }
}

/// Sends what the player wrote, with the context.
pub fn send(app: App) {
    let r = app.report;
    let message = r.text.get_untracked();
    if message.trim().is_empty() || r.status.get_untracked() == Status::Sending {
        return;
    }
    let report = build(app, message);
    r.status.set(Status::Sending);
    #[cfg(target_arch = "wasm32")]
    leptos::task::spawn_local(async move {
        match post(&report).await {
            Ok(saved) => {
                r.text.set(String::new());
                r.status.set(Status::Sent(saved.file));
            }
            Err(e) => r.status.set(Status::Failed(e)),
        }
    });
    #[cfg(not(target_arch = "wasm32"))]
    let _ = report;
}

/// Opens or closes the dialog. A finished report makes way for a new one.
fn toggle(app: App) {
    let r = app.report;
    r.open.update(|open| *open = !*open);
    if matches!(r.status.get_untracked(), Status::Sent(_) | Status::Failed(_)) {
        r.status.set(Status::Writing);
    }
}

/// The button, and the dialog when it is open.
pub fn widget(app: App) -> impl IntoView {
    let r = app.report;
    view! {
        <div class="report">
            {move || r.open.get().then(|| dialog(app))}
            <button
                class="report-button"
                type="button"
                title="Report a bug or suggest an idea"
                aria-label="Report a bug or suggest an idea"
                aria-haspopup="dialog"
                aria-expanded=move || r.open.get().to_string()
                on:click=move |_| toggle(app)
            >
                "!"
            </button>
        </div>
    }
}

fn dialog(app: App) -> impl IntoView {
    let r = app.report;
    let input = NodeRef::<leptos::html::Textarea>::new();
    Effect::new(move |_| {
        if let Some(el) = input.get() {
            let _ = el.focus();
        }
    });
    let close = move || {
        r.open.set(false);
        if matches!(r.status.get_untracked(), Status::Sent(_) | Status::Failed(_)) {
            r.status.set(Status::Writing);
        }
    };
    let on_key = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            close();
        }
    };
    let body = move || match r.status.get() {
        Status::Sent(file) => view! {
            <p class="report-done" role="status">"Thanks! Your message was saved (" <code>{file}</code> ")."</p>
            <div class="report-actions">
                <button type="button" on:click=move |_| r.status.set(Status::Writing)>"Send something else"</button>
                <button type="button" class="report-send" on:click=move |_| close()>"Close"</button>
            </div>
        }
        .into_any(),
        status => {
            let sending = status == Status::Sending;
            let failed = match status {
                Status::Failed(e) => Some(view! { <p class="report-error" role="alert">{e}</p> }),
                _ => None,
            };
            view! {
                <label for="report-text">"Something went wrong, or something you'd like changed or added?"</label>
                <textarea
                    id="report-text"
                    node_ref=input
                    rows="5"
                    maxlength=MAX_MESSAGE
                    placeholder="A bug: what happened, and what did you expect? An idea: what would you like?"
                    prop:value=move || r.text.get()
                    on:input=move |ev| r.text.set(event_target_value(&ev))
                    disabled=sending
                ></textarea>
                <p class="report-note">"Sent with it: the page you are on and what your screen shows, your game as you see it."</p>
                {failed}
                <div class="report-actions">
                    <button type="button" on:click=move |_| close()>"Cancel"</button>
                    <button
                        type="button"
                        class="report-send"
                        disabled=move || sending || r.text.with(|t| t.trim().is_empty())
                        on:click=move |_| send(app)
                    >
                        {if sending { "Sending…" } else { "Send" }}
                    </button>
                </div>
            }
            .into_any()
        }
    };
    view! {
        <div class="report-panel" role="dialog" aria-labelledby="report-title" on:keydown=on_key>
            <h2 id="report-title">"Report a bug or suggest an idea"</h2>
            {body}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{handle, Lobby};
    use crate::testing::{players, shows};
    use leptos::tachys::view::RenderHtml;
    use protocol::{GameKind, ServerMsg};

    #[test]
    fn the_button_opens_a_dialog_for_a_bug_or_a_suggestion() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let html = widget(app).to_html();
            assert!(html.contains("class=\"report-button\"") && html.contains(">!<"));
            assert!(html.contains("aria-label=\"Report a bug or suggest an idea\"") && !html.contains("report-panel"));
            toggle(app);
            let html = widget(app).to_html();
            assert!(html.contains("Report a bug or suggest an idea") && html.contains("<textarea"));
            assert!(shows(&html, "Something went wrong, or something you'd like changed or added?"), "{html}");
            assert!(html.contains("disabled"), "nothing to send yet: {html}");
            app.report.status.set(Status::Failed("The server couldn't be reached.".into()));
            assert!(shows(&widget(app).to_html(), "The server couldn't be reached."));
            app.report.status.set(Status::Sent("2026-10-12T09-41-07Z-k3f9.md".into()));
            let html = widget(app).to_html();
            assert!(html.contains("Thanks! Your message was saved") && html.contains("2026-10-12T09-41-07Z-k3f9.md"));
            assert!(html.contains("Send something else"));
            // Closing and opening again starts a new report.
            toggle(app);
            toggle(app);
            assert_eq!(app.report.status.get_untracked(), Status::Writing);
        });
    }

    #[test]
    fn a_report_says_where_the_player_was() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let r = build(app, "  The menu is empty \n".into());
            assert_eq!((r.message.as_str(), r.screen.as_str(), r.game, r.state), ("The menu is empty", "Menu", None, None));
            assert_eq!((r.room, r.token), (None, None));

            app.screen.set(Screen::Home(GameKind::Catan));
            app.name.set("Ann".into());
            let r = build(app, "x".into());
            assert_eq!((r.screen.as_str(), r.game), ("Home", Some(GameKind::Catan)));
            assert!(r.state.unwrap().contains("\"name\":\"Ann\""));

            let lobby = Lobby { room: "WXYZ".into(), game: GameKind::Hanabi, players: players(&["Ann", "Bob"]), you: 1, is_host: false };
            app.room.set("WXYZ".into());
            app.screen.set(Screen::Lobby(lobby));
            app.last_error.set(Some("That name is taken in this room.".into()));
            let r = build(app, "x".into());
            assert_eq!((r.screen.as_str(), r.room.as_deref()), ("Lobby", Some("WXYZ")));
            assert_eq!(r.last_error.as_deref(), Some("That name is taken in this room."));
            let state: serde_json::Value = serde_json::from_str(&r.state.unwrap()).unwrap();
            assert_eq!(state["lobby"]["players"][1]["name"], "Bob");
            assert_eq!(state["lobby"]["you"], 1);
            assert!(state["game"]["rules"].is_object(), "Hanabi's rules: {state}");
        });
    }

    #[test]
    fn a_report_from_a_game_carries_what_the_screen_held() {
        let owner = Owner::new();
        owner.with(|| {
            for kind in GameKind::ALL {
                let app = App::new();
                handle(app, ServerMsg::Joined { room: "WXYZ".into(), token: "t".into() });
                let names = vec!["Ann".to_string(), "Bob".to_string(), "Cy".to_string()];
                handle(app, ServerMsg::Game { view: crate::render_tests::first_view(kind), names, connected: vec![true; 3] });
                let r = build(app, "Something broke".into());
                assert_eq!((r.screen.as_str(), r.game, r.room.as_deref()), ("Game", Some(kind), Some("WXYZ")));
                let state: serde_json::Value = serde_json::from_str(&r.state.unwrap()).unwrap();
                assert!(state["view"].is_object(), "{kind:?}: {state}");
                assert_eq!(state["online"], false);
            }
        });
    }
}
