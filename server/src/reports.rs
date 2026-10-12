//! Bug reports: what players send from the button in the corner of every
//! screen, saved one Markdown file per report in the reports folder (see
//! `reports/README.md`), for an agent or a person to pick up.
//!
//! The client sends the message and where the player was (the screen, the
//! game's state as their seat saw it); the server adds the time and what it
//! knows about their room, never the seat's token.

use crate::rooms::Rooms;
use protocol::{Report, ReportSaved};
use std::{
    fs,
    io::{ErrorKind, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

/// The longest message kept, in characters.
pub const MAX_MESSAGE: usize = 5_000;
/// The largest game state kept, in bytes.
pub const MAX_STATE: usize = 1_000_000;
/// The largest request accepted, in bytes.
pub const MAX_BODY: usize = 2 * 1024 * 1024;

/// Why a report wasn't saved.
#[derive(Debug, PartialEq, Eq)]
pub enum ReportError {
    /// Nothing was written.
    Empty,
    /// The file couldn't be written.
    Io(String),
}

impl ReportError {
    pub fn message(&self) -> String {
        match self {
            ReportError::Empty => "Write what went wrong first.".to_string(),
            ReportError::Io(e) => format!("The report couldn't be saved: {e}"),
        }
    }
}

/// What the server knows about the reporter's room.
#[derive(Debug, PartialEq)]
pub struct RoomInfo {
    pub code: String,
    pub game: String,
    pub started: bool,
    /// Every seat: its name, whether it is connected.
    pub seats: Vec<(String, bool)>,
    /// The reporter's seat.
    pub seat: usize,
    /// The lobby's options, as JSON.
    pub options: Option<String>,
    /// The running game as the server would send it to the reporter's seat,
    /// as JSON, to compare with what their screen showed.
    pub view: Option<String>,
}

/// The room `code`, if `token` holds one of its seats (a report can't look
/// into someone else's room).
pub fn room_info(rooms: &Rooms, code: &str, token: Option<&str>) -> Option<RoomInfo> {
    let room = rooms.map.get(&code.trim().to_uppercase())?;
    let token = token?;
    let seat = room.seats.iter().position(|s| s.token == token)?;
    Some(RoomInfo {
        code: room.code.clone(),
        game: room.kind.title().to_string(),
        started: room.game.is_some(),
        seats: room.seats.iter().map(|s| (s.name.clone(), s.tx.is_some())).collect(),
        seat,
        options: room.options.as_ref().and_then(|o| serde_json::to_string(o).ok()),
        view: room.game.as_ref().and_then(|g| serde_json::to_string(&g.view(seat)).ok()),
    })
}

/// A moment as UTC date and time: (year, month, day, hour, minute, second).
fn utc(when: SystemTime) -> (i64, u32, u32, u32, u32, u32) {
    let secs = when.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rest) = ((secs / 86_400) as i64, secs % 86_400);
    // Days to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + (month <= 2) as i64;
    (year, month, day, (rest / 3_600) as u32, (rest / 60 % 60) as u32, (rest % 60) as u32)
}

/// A fenced block, with a fence longer than any run of backticks inside.
fn fenced(lang: &str, body: &str) -> String {
    let mut longest = 0;
    let mut run = 0;
    for c in body.chars() {
        run = if c == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    let fence = "`".repeat(longest.max(2) + 1);
    format!("{fence}{lang}\n{body}\n{fence}\n")
}

/// JSON as a readable block (left as it came if it isn't JSON).
fn json_block(text: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(text).and_then(|v| serde_json::to_string_pretty(&v)) {
        Ok(pretty) => fenced("json", &pretty),
        Err(_) => fenced("text", text),
    }
}

/// The report as Markdown.
pub fn render(report: &Report, room: Option<&RoomInfo>, when: SystemTime) -> String {
    let (y, mo, d, h, mi, s) = utc(when);
    let message = report.message.trim();
    let first: String = message.lines().next().unwrap_or("").chars().take(72).collect();
    let mut out = format!("# Bug report: {first}\n\n");
    let game = report.game.map_or("none", |g| g.title());
    let line = |out: &mut String, label: &str, value: &str| {
        if !value.is_empty() {
            out.push_str(&format!("- **{label}:** {value}\n"));
        }
    };
    line(&mut out, "Status", "open");
    line(&mut out, "When", &format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02} UTC"));
    line(&mut out, "Screen", &report.screen);
    line(&mut out, "Game", game);
    if let Some(r) = room {
        let (name, _) = &r.seats[r.seat];
        line(&mut out, "Room", &format!("{} ({})", r.code, if r.started { "game running" } else { "waiting room" }));
        line(&mut out, "Reported by", &format!("{name}, seat {}", r.seat + 1));
        let seats: Vec<String> = r
            .seats
            .iter()
            .enumerate()
            .map(|(i, (n, on))| {
                // The seats are shuffled when a game starts: the first one is
                // the host only in the waiting room.
                let host = if i == 0 && !r.started { ", host" } else { "" };
                let off = if *on { "" } else { ", offline" };
                format!("{}. {n}{host}{off}", i + 1)
            })
            .collect();
        line(&mut out, "Seats", &seats.join("; "));
    } else if let Some(code) = &report.room {
        line(&mut out, "Room", &format!("{code} (unknown to the server now)"));
    }
    line(&mut out, "Page", &report.url);
    line(&mut out, "Browser", &report.browser);
    line(&mut out, "Window", &report.viewport);
    if let Some(e) = &report.last_error {
        line(&mut out, "Last error shown", e);
    }
    out.push_str("\n## What the player wrote\n\n");
    let message: String = message.chars().take(MAX_MESSAGE).collect();
    for l in message.lines() {
        out.push_str(&format!("> {l}\n"));
    }
    if report.message.trim().chars().count() > MAX_MESSAGE {
        out.push_str(&format!("\n(cut at {MAX_MESSAGE} characters)\n"));
    }
    if let Some(state) = &report.state {
        out.push_str("\n## What the player's screen held\n\n");
        if state.len() > MAX_STATE {
            out.push_str(&format!("(left out: {} bytes, over the {MAX_STATE}-byte limit)\n", state.len()));
        } else {
            out.push_str(&json_block(state));
        }
    }
    if let Some(r) = room {
        if let Some(options) = &r.options {
            out.push_str("\n## The room's options (from the server)\n\n");
            out.push_str(&json_block(options));
        }
        if let Some(view) = &r.view {
            out.push_str("\n## The game as the server holds it for that seat\n\n");
            out.push_str(&json_block(view));
        }
    }
    out
}

/// Saves a report in `dir` as a new file named after the time, and says
/// which file.
pub fn save(dir: &Path, report: &Report, room: Option<&RoomInfo>, when: SystemTime) -> Result<ReportSaved, ReportError> {
    if report.message.trim().is_empty() {
        return Err(ReportError::Empty);
    }
    let io = |e: std::io::Error| ReportError::Io(e.to_string());
    fs::create_dir_all(dir).map_err(io)?;
    let (y, mo, d, h, mi, s) = utc(when);
    let text = render(report, room, when);
    for _ in 0..16 {
        let tag: String = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            (0..4).map(|_| b"abcdefghijkmnpqrstuvwxyz23456789"[rng.gen_range(0..32)] as char).collect()
        };
        let file = format!("{y:04}-{mo:02}-{d:02}T{h:02}-{mi:02}-{s:02}Z-{tag}.md");
        match fs::OpenOptions::new().write(true).create_new(true).open(dir.join(&file)) {
            Ok(mut f) => {
                f.write_all(text.as_bytes()).map_err(io)?;
                return Ok(ReportSaved { file });
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(io(e)),
        }
    }
    Err(ReportError::Io("no free file name".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::*;
    use protocol::{ClientMsg, GameKind};
    use std::time::Duration;

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    /// A folder of its own under the system's temporary folder.
    fn scratch(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("board-game-chest-{name}-{nanos}"))
    }

    fn report(message: &str) -> Report {
        Report {
            message: message.into(),
            screen: "Game".into(),
            game: Some(GameKind::Hanabi),
            room: Some("wxyz".into()),
            token: Some("secret-token".into()),
            url: "http://localhost:3000/".into(),
            browser: "Firefox".into(),
            viewport: "1400x1000".into(),
            last_error: Some("It's not your turn.".into()),
            state: Some(r#"{"view":{"name":"has ``` in it"}}"#.into()),
        }
    }

    #[test]
    fn times_are_written_in_utc() {
        assert_eq!(utc(at(0)), (1970, 1, 1, 0, 0, 0));
        assert_eq!(utc(at(951_782_400)), (2000, 2, 29, 0, 0, 0));
        assert_eq!(utc(at(1_791_734_196)), (2026, 10, 11, 15, 56, 36));
        assert_eq!(utc(at(4_102_444_800)), (2100, 1, 1, 0, 0, 0));
    }

    #[test]
    fn a_report_is_saved_in_a_file_of_its_own() {
        let dir = scratch("reports");
        let r = report("The clue buttons vanished.\nI clicked Red and nothing happened.");
        let saved = save(&dir, &r, None, at(1_791_734_196)).unwrap();
        assert!(saved.file.starts_with("2026-10-11T15-56-36Z-") && saved.file.ends_with(".md"), "{}", saved.file);
        let again = save(&dir, &r, None, at(1_791_734_196)).unwrap();
        assert_ne!(saved.file, again.file, "two reports in the same second");
        let text = fs::read_to_string(dir.join(&saved.file)).unwrap();
        assert!(text.starts_with("# Bug report: The clue buttons vanished.\n"));
        assert!(text.contains("- **When:** 2026-10-11 15:56:36 UTC"));
        assert!(text.contains("- **Screen:** Game") && text.contains("- **Game:** Hanabi"));
        assert!(text.contains("- **Room:** wxyz (unknown to the server now)"));
        assert!(text.contains("> The clue buttons vanished.\n> I clicked Red and nothing happened.\n"));
        assert!(text.contains("- **Last error shown:** It's not your turn."));
        // The state, pretty-printed, in a fence its backticks can't close.
        assert!(text.contains("````json\n{\n  \"view\": {"), "{text}");
        assert!(!text.contains("secret-token"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn empty_long_and_huge_reports() {
        let dir = scratch("limits");
        assert_eq!(save(&dir, &report("   "), None, at(0)), Err(ReportError::Empty));
        let mut long = report(&"x".repeat(MAX_MESSAGE + 10));
        long.state = Some("y".repeat(MAX_STATE + 1));
        let text = render(&long, None, at(0));
        assert!(text.contains(&format!("(cut at {MAX_MESSAGE} characters)")));
        assert!(text.contains("(left out:"));
        assert!(!text.contains(&"y".repeat(100)));
        let not_json = Report { state: Some("not json".into()), ..report("x") };
        assert!(render(&not_json, None, at(0)).contains("```text\nnot json\n```"));
    }

    #[test]
    fn the_reporters_room_is_described_by_the_server() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::Hanabi, 3);
        assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
        let token = clients[2].token();
        let rooms = state.lock().unwrap();
        // Someone else's room stays out of reach.
        assert_eq!(room_info(&rooms, &code, Some("not-a-token")), None);
        assert_eq!(room_info(&rooms, &code, None), None);
        let info = room_info(&rooms, &code.to_lowercase(), Some(&token)).unwrap();
        assert_eq!((info.code.as_str(), info.game.as_str(), info.started, info.seat), (code.as_str(), "Hanabi", true, 2));
        assert_eq!(info.seats.len(), 3);
        assert!(info.options.is_some() && info.view.is_some());
        let text = render(&report("Stuck"), Some(&info), at(0));
        assert!(text.contains(&format!("- **Room:** {code} (game running)")));
        assert!(text.contains("- **Reported by:** P2, seat 3"));
        assert!(text.contains("- **Seats:** 1. P0; 2. P1; 3. P2"), "no host once seats are shuffled");
        assert!(text.contains("## The game as the server holds it for that seat"));
        drop(rooms);

        // In the waiting room, the first seat is the host.
        let (code, clients) = lobby(&state, GameKind::Catan, 2);
        let rooms = state.lock().unwrap();
        let info = room_info(&rooms, &code, Some(&clients[1].token())).unwrap();
        let text = render(&report("x"), Some(&info), at(0));
        assert!(text.contains(&format!("- **Room:** {code} (waiting room)")));
        assert!(text.contains("- **Seats:** 1. P0, host; 2. P1"));
        assert!(!text.contains("## The game as the server holds it"));
    }
}
