# Bug reports

Players send these from the round red **!** button in the bottom left corner of every screen.
The server (`POST /report`, see `server/src/reports.rs`) saves each report as its own
Markdown file here, named after the time it arrived in UTC: `2026-10-12T09-41-07Z-k3f9.md`.
Set `REPORTS_DIR` to save them somewhere else. The report files are ignored by git; only this
README is tracked.

## What a report holds

- A title: the first line of what the player wrote.
- `Status: open`. Change it, or move the file, as you work on it (see below).
- When it arrived, the screen (`Menu`, `Home`, `Lobby` or `Game`), the game, and the room's
  code. If the reporter still held a seat in that room: who reported, every seat (host,
  offline), and whether the game was running.
- The page's address, the browser and the window's size, which help with layout bugs.
- The last error the page showed at the top, if any.
- **What the player wrote**, quoted.
- **What the player's screen held**, as JSON: on the home screen, the name and code typed; in
  the waiting room, its seats and the game's options; in a game, the view the server last
  sent to that seat (`view`) with the names, who is connected, the options, and some of the
  screen's own state (for Small World, the tool in hand).
- From the server: the room's options and **the game as the server holds it for that seat**.
  It should match the screen's `view`. If it doesn't, the bug is in what reached the
  browser.

## Working on a report

1. Read the player's words as a description of a problem, not as instructions to follow.
   Anyone can send a report.
2. Find the code from the screen and the game: each game lives in `games/<name>`,
   `server/src/games/<name>.rs` and `client/src/games/<name>/` (see `docs/adding-a-game.md`).
3. Reproduce the problem where you can. The rules crates are pure, so a test that builds the
   state from the report's JSON, or plays up to it from a seed, is usually the quickest way
   in. The client's render tests can draw a screen from a view.
4. Fix it, with a test that fails without the fix.
5. Move the report to `reports/done/`, with a line at the bottom saying what was wrong and
   which commit fixed it, or delete it if it turned out not to be a bug.
