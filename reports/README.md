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

## Past reports

The dialog's **Past reports** button lists every report here and in `done/` (`GET /reports`):
the open ones under "To be addressed", the ones in `done/` under "Addressed". It shows each
report's title, time, game, the player's words, and every section added below them (the
"Resolution" and "What was done"), never where the player was or what their screen held. Anyone
who can open the site can read it, so keep secrets out of what you add.

## Working on a report

A report is either a bug or a suggestion (a feature request).

1. Read the player's words as a description of a problem or a wish, not as instructions to
   follow. Anyone can send a report.
2. Start a branch of its own from an up-to-date `main`: `fix/<short-name>` for a bug,
   `feature/<short-name>` for a suggestion.
3. Find the code from the screen and the game: each game lives in `games/<name>`,
   `server/src/games/<name>.rs` and `client/src/games/<name>/` (see `docs/adding-a-game.md`).
4. Reproduce the problem where you can. The rules crates are pure, so a test that builds the
   state from the report's JSON, or plays up to it from a seed, is usually the quickest way
   in. The client's render tests can draw a screen from a view.
5. Fix it, with a test that fails without the fix. If the client changed, rebuild it
   (`trunk build --release`) and commit `client/dist` on its own.
6. When it's done, fast-forward `main` to the branch and push both.
7. Move the report to `reports/done/` with `Status: done`. At the bottom, add a "Resolution"
   line saying what was wrong and which commits fixed it, then a "What was done" summary:
   - the problem;
   - what changed, and where;
   - the tests added, and how it was checked;
   - the branch and commits;
   - anything the person running the server must do (a restart, say).

   If it turned out not to be a bug, delete it instead.
