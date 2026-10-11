# Catan + Hanabi + It's a Wonderful World

Three multiplayer, browser-based games in Rust, served by one server and played from
one page: Catan, the Hanabi clone from
[McM1k/hanabii](https://github.com/McM1k/hanabii) (merged from commit `c2d5fa6`), and a
card-drafting empire builder ("It's a Wonderful World", base game).
All use the same rooms: one player creates a room and gets a 4-letter code, the
others join with it, the host starts the game.

| Where | What it is |
|-------|------------|
| `games/catan` (crate `catan-core`) | Catan rules: board, game, actions, per-player views. No I/O. |
| `games/hanabi` (crate `hanabi-core`) | Hanabi rules, imported from the Hanabi repo's `game-core` (variants, "hanabii" mode, redacted per-player view). No I/O. |
| `games/wonderful` (crate `wonderful-core`) | It's a Wonderful World rules: the card table, drafting, planning, production, scoring, redacted per-player view. No I/O. |
| `protocol` | The WebSocket messages: the room flow every game shares (create, join, lobby options, start, leave), and one enum per thing that differs by game (`GameKind`, `GameOptions`, `GameMove`, `GameView`). |
| `server` | Axum server: the room system (`rooms.rs`, which names no game), each game's server side in `src/games/`, one WebSocket per player; serves the client. |
| `client` | Leptos (CSR/WASM) front end: a shell (`app.rs`, `menu.rs`, `main.rs`: the menu, the connection, the routing between screens) and each game's screens and stylesheet in `src/games/<name>/`. |

Each game is the same set of folders: its rules crate in `games/`, a module in
`server/src/games/` behind two traits (`Game` to start it, `Running` to play it), and a folder
in `client/src/games/` behind one trait (menu card, home, lobby, board, page title). Each crate
has one registry listing the games. `docs/adding-a-game.md` walks through adding one.

The server is authoritative. Clients send moves; the server validates them with the
game's rules crate and sends every seat its own filtered view (other players' hands
and development cards in Catan, your own cards in Hanabi, the hand you are drafting
from, your pick and your drafted cards in It's a Wonderful World, are never sent to the
other seats).

## Run it

You need Rust, the WASM target and Trunk:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked

cd client && trunk build --release && cd ..   # outputs client/dist
cargo run -p server --release                  # http://localhost:3000
```

Open the page in several browser windows (or on several machines), pick a game, create
a room in one and join with the 4-letter code in the others. Refreshing the page puts
you back in your seat automatically (a token is kept in the tab's `sessionStorage`, so
two tabs of one browser are two players).

Env vars: `PORT` (default 3000), `STATIC_DIR` (default `client/dist`).

Development with live reload: run `cargo run -p server` and, in `client/`,
`trunk serve` (its `Trunk.toml` proxies `/ws` to the server on port 3000).

### Make sure you are serving a fresh build

`client/dist` holds a committed release build. `trunk serve` builds into that same folder (as
a development build), so run `trunk build --release` again before committing it. If
`trunk build` fails it leaves the previous `client/dist` in place, so an old page can keep
being served: delete `client/dist` before building, and a failed build shows up as a missing
page instead.

## Rooms (the part that changed for Hanabi)

Hanabi used to have its own server with a "type a name and a room code" join screen,
where anyone typing the same code landed in the same room and any seat could start
the game. It now runs on the room system Catan already had:

* **Create → code.** The game is picked in the menu; the server makes a free 4-letter
  code (no `I` or `O`). **Join** takes the code, and the code decides which game you
  end up in, whatever you picked in the menu.
* **Host.** The first seat is the host. Only the host starts the game, and in Hanabi
  only the host picks the variant rules (everyone sees them, locked, as they change).
  It's a Wonderful World has no options: the host just starts.
* **Seats and reconnecting.** Every seat has a token kept in the browser tab. If the
  connection drops during a game the seat stays yours, shown as "offline" to the
  others, and a refresh or a reconnect takes it back. In a lobby, leaving frees the seat;
  a dropped connection (a reload, a phone switching apps) keeps it for 2 minutes first.
* **Names** are trimmed, at most 20 characters, and unique in a room (ignoring case).
* **Limits.** Catan 2–4 players, Hanabi 2–5, It's a Wonderful World 2–5. Joining a running game is refused
  unless you hold its token. A room nobody has connected to for 30 minutes is removed.
* **Seat order.** Catan shuffles the seats when the game starts. Hanabi keeps join
  order, so the host plays first and the order round the table is who joined when.
  It's a Wonderful World shuffles the seats like Catan; there are no turns, so the
  seat only decides who passes cards to whom, and which Empire you play.

Rooms live in memory only; a server restart ends every game.

## Tests

```sh
cargo test --workspace
```

* `catan-core` (17): board shape, setup order, production, robber/discard, trading, ports,
  longest road (incl. cut roads), largest army, dev cards, winning, hidden info.
* `hanabi-core` (133): the original rules tests, unchanged apart from one assertion
  rewritten the way clippy prefers.
* `wonderful-core` (56, plus one ignored): the base-game card table, the draft (hand
  sizes, which way the hands go in each round, secret picks, the discards with two
  players), planning (build, recycle, scrap, cubes that must fit where they go),
  production (the races and their ties, the characters, the Science choice, Krystallium,
  cubes that are lost, the wrap-up step), scoring and the tie-breaks, what a view hides,
  JSON round trips, and whole games from a seed for 2 to 5 players. The ignored
  `economy_report` prints how a simple bot fares with the cards (see "The cards" below).
* `protocol` (8): every message survives JSON, including each game's view (the Hanabi
  maps are keyed by seat and color), every move, view and option names its game, and rule
  payloads from before "hanabii" existed still parse.
* `server` (37): in `rooms.rs`, the room flows run for every game: starting with a full
  table and a view for every seat, host-only start and options, seat order, fullness,
  reconnecting with a token, stale disconnects, lobby seats kept through a short drop, moves
  for the wrong game or before the start. In `src/games/`, each game's own: Catan's turns,
  Hanabi's rules options (hanabii locks the others, rules freeze once the game starts, …) and
  a whole game, It's a Wonderful World's hidden picks, seat checks and whole games.
* `client` (86): native render tests that build real games and render every screen to
  HTML. In the shell: every game is registered under its own kind and drawn in its own
  frame, and a state from the server puts its board on screen until you leave. In each
  game's folder: all Catan phases, the Hanabi board for every seat, player count and rule
  set, whole games rendered move by move, the clue buttons, rings, drop zones, game-over
  line, flashes; It's a Wonderful World's waiting room and board for every seat, phase and
  player count of a whole game; the lobbies; and the pure helpers behind what a click
  means. Animations, drag and drop and the CSS need a browser (see below).

## What has been run in a browser

Scripted runs in headless Firefox with two players: the menu; creating, joining and starting
each game; a first move in each (a Catan settlement and road, a Hanabi clue, an It's a
Wonderful World pick); reloading mid-game, which lands back on the board; leaving, which
goes back to the menu; the page title throughout. It's a Wonderful World was also taken
through its draft, planning and an Energy step (its kept cards also at phone width), and
joining a room from a second tab was tried by hand. Still to look at with people:

* a whole game of each;
* Hanabi's hand-slide animation, drag and drop onto the play/discard zones, and the
  flashes and spinning rings (these live in effects that only run in a browser);
* phones: It's a Wonderful World's tray that sticks to the bottom of the screen, tap
  targets, and the other two games' layouts.

## Game menu

The first screen is a picker (`client/src/menu.rs`). A card leads to that game's
create/join screen; "← All games" and leaving a game bring you back.

## Catan

### Look and feel

* Tiles are original SVG illustrations (`client/src/games/catan/art.rs`): pine forest, clay hills
  with brick stacks, sheep meadow, wheat fields, snowy mountains, desert with cactus.
  Alternate tiles are mirrored so neighbours of one terrain differ. No image files.
* 2:1 harbours are drawn in the colour of their resource, 3:1 harbours in neutral.
* Anything that just changed on the board (a new road, settlement, city upgrade, or the
  robber's new tile) gets an ember contour that flickers and fades over 2.4 s
  (`EMBER_MS` in `client/src/games/catan/look.rs`, animation in `catan.css` beside it). Changes are
  found by diffing each new game state against the previous one on the client, so every
  player sees them. The first state after joining or reconnecting isn't highlighted.

### Rules implemented

Standard 19-hex island with 9 ports, snake-order setup, dice production (bank
shortages handled), robber + stealing + discard on 7, roads/settlements/cities with
piece limits and the distance rule, bank/port trades, player-to-player offers
(proposer confirms with whoever accepted), development cards (Knight, Road Building,
Year of Plenty, Monopoly, Victory Point; not playable the turn bought, one per turn),
Longest Road, Largest Army, first to 10 points wins.

### Adding your tweaks

Numbers live in `catan_core::Rules` (`victory_points`, `discard_above`, `bank_ratio`,
`longest_road_min`, `largest_army_min`) and are passed to `Game::new` in
`server/src/games/catan.rs`. Structural tweaks go in `games/catan/src/game.rs`:

* new costs / pieces: `build_*` functions and the `MAX_*` constants
* new action: add a variant to `Action`, handle it in `apply_current`
* new board layout: `Board::generate` in `games/catan/src/board.rs`
* anything the UI must show: add it to `GameView` (`view_for`) and render it in
  `client/src/games/catan/board.rs` (the game screen) or `map.rs` (the island)

Because the rules crate has no I/O, new rules are easy to unit-test (see the tests at the
bottom of `game.rs`).

Where it lives: rules in `games/catan` (`catan-core`), server side in
`server/src/games/catan.rs`, screens in `client/src/games/catan/` (`screens.rs` for
create/join and the waiting room, `board.rs` for the game screen, `map.rs` and `art.rs` for
the island, `look.rs` for the ember highlights), styles in `catan.css` beside them, all scoped
under `.theme-catan`.

## Hanabi

A cooperative card game for 2–5 players: build the fireworks in order, but you can't
see your own hand. Everything of the original is kept: the variant rules the host
picks in the lobby (multicolor, black powder, extra colors, six-card suits, the
"short" 1-of-each versions, and the "hanabii" preset), the clue buttons with their
touch preview, drag a card onto the play or discard zone, the received-clue badge, the
highlights for new cards and touched cards, the turn-ordered hands that slide to their
new place, and the fireworks that fill in as a suit is built.

Where it lives: rules in `games/hanabi` (`hanabi-core`), server side in
`server/src/games/hanabi.rs`, screens in `client/src/games/hanabi/` (`screens.rs` for the
page, create/join and the lobby with its rules picker, `board.rs` for the game, `look.rs`
for the pure display helpers), styles in `hanabi.css` beside them, scoped under `.hanabi`.

What differs from the stand-alone app:

* the room system above (host, generated codes, tokens), in place of a free-form code
  where any seat could change rules and start;
* a seat that disconnects is kept and marked "offline" instead of leaving the game;
* a "Room ABCD / Leave game" bar above the board;
* a rejoin mid-game doesn't flash every firework and the discard pile as if they had
  just changed;
* moves the rules refuse come back as a short message in a toast at the top of the page.

The wire protocol is serde's default JSON for `protocol::ClientMsg` / `ServerMsg`;
Hanabi moves are `ClientMsg::Move(GameMove::Hanabi(..))`, the lobby's rules travel as
`GameOptions::Hanabi`, and the game arrives as a `ServerMsg::Game` holding a
`GameView::Hanabi`.

## It's a Wonderful World

A card-drafting empire builder for 2–5 players, played over four rounds, where
**everybody acts at the same time**: there are no turns, so the game only ever waits for the
slowest player. Each round has three phases.

1. **Draft.** You are dealt 7 cards (10 with two players, where the 3 cards left at the end
   are discarded). Each step you keep one and pass the rest on: to the next seat in rounds 1
   and 3, to the previous one in rounds 2 and 4. Picks are secret until everybody has picked.
2. **Planning.** Every card you kept is either **built** (it goes under construction) or
   **recycled** for the cube it gives, which goes on a building that still needs that
   resource or on your Empire. A building under construction can be **scrapped**: what was
   placed on it is lost, and its recycling cube goes on your Empire.
3. **Production**, one resource after the other: Materials, Energy, Science, Gold, Exploration.
   Your Empire and your finished cards make cubes of that resource; you place them on
   buildings or on the Empire, and **cubes you don't place are lost**. Whoever makes strictly
   the most of a resource takes a character: a Financier for Materials or Gold, a General for
   Energy or Exploration, and for Science the winner chooses. A tie gives nothing. Characters
   fill the character spaces of buildings. Five cubes on the Empire make a Krystallium, a
   wildcard cube that stands in for any resource (not for a character), and the only thing
   that fills a card's Krystallium spaces. A last "wrap-up" step is there to place Krystallium
   and characters; it is skipped when nobody can.

A building with every space filled is finished: from then on it produces every round, and some
cards hand over a one-off bonus (Krystallium or characters, kept until used). After round 4 you
score the printed points of your finished cards, the points that depend on the cards of a type
or the characters you hold (on your cards and on your Empire), and 1 for each General and each
Financier. Ties go to whoever finished most cards, then to whoever holds most characters, and
are shared beyond that.

### The cards

The catalogue is the published base game: 78 designs, 150 cards (many designs come in several
copies), and the five Empires on their side A (Noram States, Republic of Europe, Federation of
Asia, Panafrican Union, Aztec Empire), which each produce a few cubes and score a bonus. The
numbers come from Game Park's online version of the game
([gamepark/its-a-wonderful-world](https://github.com/gamepark/its-a-wonderful-world)). Seats are
shuffled when the game starts and seat *i* plays Empire *i*.

The whole catalogue is one table, `designs()` in `games/wonderful/src/cards.rs`, with each
design's number of copies, and the `EMPIRES` table below it. `cargo test -p wonderful-core
economy_report -- --ignored --nocapture` prints how two simple bots fare with it.

### What isn't in it

Only the base game on side A of the Empires: no side B (the same Empire for everybody), no
Corruption & Ascension or War or Peace option in the lobby, no solo play, and no turn timer.

### Where it lives

Rules in `games/wonderful` (`wonderful-core`, pure and deterministic: the shuffle is a small
generator seeded by the server, so there is no `rand` and a game can be replayed from its seed);
server side in `server/src/games/wonderful.rs`; screens in `client/src/games/wonderful/`
(`screens.rs` for the page, create/join and the waiting room, `board.rs` for the game,
`look.rs` for the pure display helpers and what a click means, `mod.rs` for the state kept
between redraws); styles in `wonderful.css` beside them, all scoped under `.wonderful`.

Moves are `ClientMsg::Move(GameMove::Wonderful(..))` and the game arrives as a
`ServerMsg::Game` holding a `GameView::Wonderful`. Because everybody moves at once, the server checks each move
against the rules and the seat of the connection that sent it, never against a turn, and
sends every seat its own view after every move.

## Known gaps

* Rooms live in memory only; a server restart ends all games.
* No chat, spectators or turn timers; a finished game ends the room's play (leave and
  create a new room for another round).
* Catan always uses the random standard layout.
* It's a Wonderful World is the base game only, on side A of the Empires (see its section
  above).
