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
//!
//! "Past reports" swaps the box for every report sent so far (`GET
//! /reports`): those still to be addressed, and those addressed with what
//! was done about them.

use crate::app::{load, App, Screen, KEY_TOKEN};
use crate::games;
use leptos::prelude::*;
use protocol::{PastReport, Report};

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

/// The past reports, while the dialog lists them.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub enum Past {
    Loading,
    Loaded(Vec<PastReport>),
    /// Not loaded, and why.
    Failed(String),
}

/// The dialog's state, which outlives the screens behind it.
#[derive(Clone, Copy)]
pub struct Signals {
    pub open: RwSignal<bool>,
    /// What the player has written so far (kept if they close the dialog).
    pub text: RwSignal<String>,
    pub status: RwSignal<Status>,
    /// The past reports, when the dialog shows them instead of the box.
    pub past: RwSignal<Option<Past>>,
}

impl Signals {
    pub fn new() -> Signals {
        Signals {
            open: RwSignal::new(false),
            text: RwSignal::new(String::new()),
            status: RwSignal::new(Status::Writing),
            past: RwSignal::new(None),
        }
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

#[cfg(target_arch = "wasm32")]
fn unreachable(_: wasm_bindgen::JsValue) -> String {
    "The server couldn't be reached. Try again in a moment.".to_string()
}

/// Sends a request to the server and reads its answer, or what went wrong.
#[cfg(target_arch = "wasm32")]
async fn call(request: web_sys::Request) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::Response;

    let window = web_sys::window().ok_or("No browser window.")?;
    let response: Response =
        JsFuture::from(window.fetch_with_request(&request)).await.map_err(unreachable)?.dyn_into().map_err(unreachable)?;
    let text = JsFuture::from(response.text().map_err(unreachable)?).await.map_err(unreachable)?.as_string().unwrap_or_default();
    if response.ok() {
        Ok(text)
    } else if text.is_empty() {
        Err(format!("The server answered {}.", response.status()))
    } else {
        Err(text)
    }
}

/// Posts a report; the file it was saved in, or what went wrong.
#[cfg(target_arch = "wasm32")]
async fn post(report: &Report) -> Result<protocol::ReportSaved, String> {
    use wasm_bindgen::JsValue;
    use web_sys::{Headers, Request, RequestInit};

    let body = serde_json::to_string(report).map_err(|e| e.to_string())?;
    let headers = Headers::new().map_err(unreachable)?;
    headers.set("Content-Type", "application/json").map_err(unreachable)?;
    let init = RequestInit::new();
    init.set_method("POST");
    init.set_headers(&headers);
    init.set_body(&JsValue::from_str(&body));
    let text = call(Request::new_with_str_and_init("/report", &init).map_err(unreachable)?).await?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Shows the past reports in place of the box, loading them afresh.
fn show_past(app: App) {
    let r = app.report;
    r.past.set(Some(Past::Loading));
    #[cfg(target_arch = "wasm32")]
    leptos::task::spawn_local(async move {
        let list = async {
            let text = call(web_sys::Request::new_with_str("/reports").map_err(unreachable)?).await?;
            serde_json::from_str::<Vec<PastReport>>(&text).map_err(|e| e.to_string())
        };
        let past = match list.await {
            Ok(list) => Past::Loaded(list),
            Err(e) => Past::Failed(e),
        };
        // Unless the player went back to the box meanwhile.
        if r.past.get_untracked() == Some(Past::Loading) {
            r.past.set(Some(past));
        }
    });
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

/// Opens or closes the dialog. A finished report makes way for a new one,
/// and the dialog opens on the box. It shares the corner with the rules,
/// so it closes them.
pub fn toggle(app: App) {
    let r = app.report;
    r.open.update(|open| *open = !*open);
    if r.open.get_untracked() {
        app.rules_open.set(false);
    }
    if matches!(r.status.get_untracked(), Status::Sent(_) | Status::Failed(_)) {
        r.status.set(Status::Writing);
    }
    r.past.set(None);
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
        if r.open.get_untracked() {
            toggle(app);
        }
    };
    let on_key = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            close();
        }
    };
    let past_button = move || {
        view! {
            <button type="button" class="report-past-button" on:click=move |_| show_past(app)>
                "Past reports"
            </button>
        }
    };
    let body = move || match r.status.get() {
        _ if r.past.with(Option::is_some) => past(app, close),
        Status::Sent(file) => view! {
            <p class="report-done" role="status">"Thanks! Your message was saved (" <code>{file}</code> ")."</p>
            <div class="report-actions">
                {past_button()}
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
                    {past_button()}
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
    let listing = move || r.past.with(Option::is_some);
    view! {
        <div
            class="report-panel"
            class:report-panel-wide=listing
            role="dialog"
            aria-labelledby="report-title"
            on:keydown=on_key
        >
            <h2 id="report-title">{move || if listing() { "Past reports" } else { "Report a bug or suggest an idea" }}</h2>
            {body}
        </div>
    }
}

/// The past reports: those still to be addressed, then those addressed.
fn past(app: App, close: impl Fn() + Copy + 'static) -> AnyView {
    let r = app.report;
    let content = match r.past.get() {
        None | Some(Past::Loading) => view! { <p class="report-note" role="status">"Loading…"</p> }.into_any(),
        Some(Past::Failed(e)) => view! { <p class="report-error" role="alert">{e}</p> }.into_any(),
        Some(Past::Loaded(list)) => {
            let (done, open): (Vec<_>, Vec<_>) = list.into_iter().partition(|p| p.done);
            view! {
                {section("To be addressed", "Nothing is waiting.", open)}
                {section("Addressed", "None yet.", done)}
            }
            .into_any()
        }
    };
    view! {
        <div class="report-past">{content}</div>
        <div class="report-actions">
            <button type="button" on:click=move |_| r.past.set(None)>"Back"</button>
            <button type="button" class="report-send" on:click=move |_| close()>"Close"</button>
        </div>
    }
    .into_any()
}

fn section(title: &str, empty: &'static str, reports: Vec<PastReport>) -> impl IntoView {
    let heading = format!("{title} ({})", reports.len());
    let list = if reports.is_empty() {
        view! { <p class="report-note">{empty}</p> }.into_any()
    } else {
        view! { <ul class="report-list">{reports.into_iter().map(item).collect_view()}</ul> }.into_any()
    };
    view! {
        <section class="report-section">
            <h3>{heading}</h3>
            {list}
        </section>
    }
}

/// One report, folded: its title, when and in which game; unfolded, what
/// the player wrote and what was done.
fn item(p: PastReport) -> impl IntoView {
    let meta = [Some(p.when), Some(code(&p.file).to_string()), p.game].into_iter().flatten().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
    let meta = (!meta.is_empty()).then(|| view! { <span class="report-meta">{meta}</span> });
    // A status other than the folder's own, such as "in progress".
    let status = (!["open", "done", ""].contains(&p.status.as_str())).then(|| view! { <span class="report-status">{p.status}</span> });
    let resolution = (!p.resolution.is_empty()).then(|| view! { <div class="report-resolution">{markdown(&p.resolution)}</div> });
    view! {
        <li class="report-item">
            <details>
                <summary>
                    <span class="report-item-title">{p.title}</span>
                    {status}
                    {meta}
                </summary>
                <blockquote class="report-message">{p.message}</blockquote>
                {resolution}
            </details>
        </li>
    }
}

/// A report's code: the tag that ends its file's name ("k3f9" in
/// "2026-10-12T09-41-07Z-k3f9.md").
fn code(file: &str) -> &str {
    let stem = file.strip_suffix(".md").unwrap_or(file);
    stem.rsplit_once('-').map_or(stem, |(_, tag)| tag)
}

/// A resolution's Markdown, drawn plainly: headings, bullet lists (indented
/// as written), paragraphs, tables and code blocks, with `code` and **bold** inside.
fn markdown(text: &str) -> impl IntoView {
    let mut views = Vec::new();
    let mut bullets: Vec<(usize, String)> = Vec::new();
    let mut paragraph: Option<String> = None;
    let flush = |views: &mut Vec<AnyView>, bullets: &mut Vec<(usize, String)>, paragraph: &mut Option<String>| {
        if let Some(p) = paragraph.take() {
            views.push(view! { <p>{inline(&p)}</p> }.into_any());
        }
        if !bullets.is_empty() {
            let items = bullets
                .drain(..)
                .map(|(depth, b)| view! { <li style:margin-left=format!("{}rem", depth as f32 * 1.1)>{inline(&b)}</li> })
                .collect_view();
            views.push(view! { <ul>{items}</ul> }.into_any());
        }
    };
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if trimmed.starts_with('|') {
            flush(&mut views, &mut bullets, &mut paragraph);
            let mut rows = vec![cells(trimmed)];
            while let Some(row) = lines.clone().next().map(str::trim).filter(|l| l.starts_with('|')) {
                lines.next();
                rows.push(cells(row));
            }
            views.push(table(rows).into_any());
        } else if trimmed.starts_with("```") {
            flush(&mut views, &mut bullets, &mut paragraph);
            let fence = &trimmed[..trimmed.len() - trimmed.trim_start_matches('`').len()];
            let code: Vec<&str> = lines.by_ref().take_while(|l| l.trim() != fence).collect();
            views.push(view! { <pre>{code.join("\n")}</pre> }.into_any());
        } else if trimmed.starts_with('#') {
            flush(&mut views, &mut bullets, &mut paragraph);
            views.push(view! { <h4>{inline(trimmed.trim_start_matches('#').trim())}</h4> }.into_any());
        } else if let Some(b) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            if paragraph.is_some() {
                flush(&mut views, &mut bullets, &mut paragraph);
            }
            bullets.push((indent / 2, b.to_string()));
        } else if trimmed.is_empty() {
            flush(&mut views, &mut bullets, &mut paragraph);
        } else if let (Some((_, last)), None) = (bullets.last_mut(), &paragraph) {
            // A bullet that goes on over the next line.
            last.push(' ');
            last.push_str(trimmed);
        } else {
            let p = paragraph.get_or_insert_with(String::new);
            if !p.is_empty() {
                p.push(' ');
            }
            p.push_str(trimmed);
        }
    }
    flush(&mut views, &mut bullets, &mut paragraph);
    views
}

/// A table row's cells: `| a | b |` gives "a" and "b"; `\\|` is a `|` inside a cell.
fn cells(row: &str) -> Vec<String> {
    let row = row.trim();
    let row = row.strip_prefix('|').unwrap_or(row);
    let row = if row.ends_with('|') && !row.ends_with("\\|") { &row[..row.len() - 1] } else { row };
    let mut cells = vec![String::new()];
    let mut chars = row.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => cells.last_mut().unwrap().push(chars.next().unwrap()),
            '|' => cells.push(String::new()),
            c => cells.last_mut().unwrap().push(c),
        }
    }
    cells.iter().map(|c| c.trim().to_string()).collect()
}

/// A table's rows; a second row of dashes (`|---|:--:|`) makes the first its header.
fn table(mut rows: Vec<Vec<String>>) -> impl IntoView {
    let is_separator = |row: &Vec<String>| row.iter().all(|c| !c.is_empty() && c.chars().all(|ch| matches!(ch, '-' | ':')));
    let head = (rows.len() > 1 && is_separator(&rows[1])).then(|| {
        rows.remove(1);
        let head = rows.remove(0).iter().map(|c| view! { <th>{inline(c)}</th> }).collect_view();
        view! { <thead><tr>{head}</tr></thead> }
    });
    let body = rows.iter().map(|row| view! { <tr>{row.iter().map(|c| view! { <td>{inline(c)}</td> }).collect_view()}</tr> }).collect_view();
    view! { <div class="report-table"><table>{head}<tbody>{body}</tbody></table></div> }
}

/// A line's `code` and **bold**.
fn inline(text: &str) -> Vec<AnyView> {
    let mut views = Vec::new();
    for (i, part) in text.split('`').enumerate() {
        if i % 2 == 1 {
            views.push(view! { <code>{part.to_string()}</code> }.into_any());
            continue;
        }
        for (j, bit) in part.split("**").enumerate().filter(|(_, b)| !b.is_empty()) {
            views.push(if j % 2 == 1 { view! { <strong>{bit.to_string()}</strong> }.into_any() } else { bit.to_string().into_any() });
        }
    }
    views
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
    fn past_reports_list_what_waits_and_what_was_done() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            toggle(app);
            assert!(shows(&widget(app).to_html(), "Past reports"), "the box offers them");
            app.report.text.set("Half written".into());
            app.report.past.set(Some(Past::Loading));
            let html = widget(app).to_html();
            assert!(html.contains("Loading…") && html.contains("report-panel-wide") && !html.contains("<textarea"), "{html}");

            let open = PastReport {
                file: "2026-10-12T13-00-00Z-abcd.md".into(),
                title: "The <b>board</b> went blank".into(),
                when: "2026-10-12 13:00:00 UTC".into(),
                game: Some("Hanabi".into()),
                status: "in progress".into(),
                message: "The <b>board</b> went blank\nafter a clue.".into(),
                ..PastReport::default()
            };
            let done = PastReport {
                file: "2026-10-12T12-00-00Z-efgh.md".into(),
                title: "Bigger tokens".into(),
                status: "done".into(),
                done: true,
                message: "Bigger tokens".into(),
                resolution: "## Resolution\n\nDone in abc1234.\n\n## What was done\n\n- **What changed:**\n  - `look.rs` draws\n    them bigger.".into(),
                ..PastReport::default()
            };
            app.report.past.set(Some(Past::Loaded(vec![open, done])));
            let html = widget(app).to_html();
            assert!(html.contains("To be addressed (1)") && html.contains("Addressed (1)"), "{html}");
            // What players wrote is shown, never run as markup.
            assert!(html.contains("The &lt;b&gt;board&lt;/b&gt; went blank") && !html.contains("<b>board"), "{html}");
            assert!(html.contains("2026-10-12 13:00:00 UTC · abcd · Hanabi") && html.contains("in progress"));
            assert!(html.contains("<h4>Resolution") && html.contains("<p>Done in abc1234."), "{html}");
            assert!(html.contains("<strong>What changed:</strong>") && html.contains("<code>look.rs</code>"));
            assert!(shows(&html, " draws them bigger."), "a bullet over two lines: {html}");
            assert!(html.contains("margin-left:1.1rem"), "a nested bullet: {html}");

            app.report.past.set(Some(Past::Loaded(vec![])));
            let html = widget(app).to_html();
            assert!(html.contains("Nothing is waiting.") && html.contains("None yet."));
            app.report.past.set(Some(Past::Failed("The server answered 500.".into())));
            assert!(widget(app).to_html().contains("The server answered 500."));

            // Back to the box, with what was written.
            app.report.past.set(None);
            let html = widget(app).to_html();
            assert!(html.contains("<textarea") && !html.contains("report-panel-wide"));
            assert_eq!(app.report.text.get_untracked(), "Half written");
            // Closing and opening again shows the box.
            app.report.past.set(Some(Past::Loading));
            toggle(app);
            toggle(app);
            assert_eq!(app.report.past.get_untracked(), None);
        });
    }

    #[test]
    fn a_resolution_draws_its_tables_lists_and_paragraphs() {
        let owner = Owner::new();
        owner.with(|| {
            let text = "Before the table\ngoes on.\n\n| Card | Cost |\n|:-----|-----:|\n| `Zeppelin` | **3** |\n| <b>Lab</b> | 2 \\| 4 |\n\n- one\n- two\n\nAfter.";
            let html = view! { <div>{markdown(text)}</div> }.to_html().replace("<!>", "");
            assert!(html.contains("<p>Before the table goes on.</p>"), "{html}");
            assert!(html.contains("<table>") && html.contains("<th>Card</th>") && html.contains("<th>Cost</th>"), "{html}");
            assert!(html.contains("<td><code>Zeppelin</code></td><td><strong>3</strong></td>"), "{html}");
            // What a table holds is shown, never run as markup, and `\|` stays in its cell.
            assert!(html.contains("<td>&lt;b&gt;Lab&lt;/b&gt;</td><td>2 | 4</td>") && !html.contains("<b>Lab"), "{html}");
            assert!(!html.contains("|-----"), "the separator row isn't drawn: {html}");
            assert!(html.contains("<ul>") && html.contains("one</li>") && html.contains("<p>After.</p>"), "{html}");
            // A table without a separator row has no header.
            let html = view! { <div>{markdown("| a | b |\n| c | d |")}</div> }.to_html().replace("<!>", "");
            assert!(html.contains("<td>a</td><td>b</td>") && !html.contains("<th>"), "{html}");
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
