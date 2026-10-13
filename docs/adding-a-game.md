# Adding a game

Every game lives in the same four places, each a folder or a file named after the game, and
plugs into one short registry per crate. Nothing shared (the rooms, the connection, the menu,
the routing between screens) names a game, so a new one leaves all of that alone. The steps
use a made-up game, Nim, as the example; Catan, Hanabi, It's a Wonderful World, Small World and
Barrage are the real ones to copy from.

## 1. Rules: `games/nim/`

A library crate called `nim-core`, with no I/O and no UI: the state, the moves and what one
seat may see.

- `State` holds the authoritative game. It applies a seat's move, or refuses it with a message
  the player will read.
- `Action` (the moves) and `View` (one seat's view of the game) derive `Serialize` and
  `Deserialize`. The view is all the browser ever gets, so leave out what that seat must not
  see. It travels as JSON, where map keys must be strings: prefer vectors indexed by seat.
- Test the rules in the crate itself; those tests are the cheapest to run and to read.

The workspace takes every crate in `games/` as a member. Declare the new one once in the root
`Cargo.toml`, under `[workspace.dependencies]`:

```toml
nim-core = { path = "games/nim" }
```

and add `nim-core = { workspace = true }` to the dependencies of `protocol`, `server` and
`client`.

## 2. Protocol: `protocol/src/lib.rs`

One variant in each per-game enum:

- `GameKind::Nim`, with its `title()`, `min_players()` and `max_players()`;
- `GameMove::Nim(nim_core::Action)` and `GameView::Nim(Box<nim_core::View>)`, with their arms
  in `game()`;
- `GameOptions::Nim(..)` only if the host picks options in the lobby (Hanabi's variant rules
  are the example to follow).

From there the compiler points at every `match` that needs the new game, on both sides.

## 3. Server: `server/src/games/nim.rs`

The game implements the two traits of `server/src/games/mod.rs`:

```rust
pub struct Nim;

impl Game for Nim {
    /// Shuffled seats, as in every game so far; `false` keeps the join order, host first.
    fn shuffles_seats(&self) -> bool {
        true
    }

    fn start(&self, names: Vec<String>, _options: Option<&GameOptions>, rng: &mut dyn RngCore) -> Box<dyn Running> {
        Box::new(nim_core::State::new(names.len(), rng.gen()))
    }

    // default_options() and check_options() only for a game with lobby options.
}

impl Running for nim_core::State {
    fn play(&mut self, seat: usize, mv: GameMove) -> Result<(), String> {
        let GameMove::Nim(action) = mv else {
            return Err(WRONG_GAME.into());
        };
        self.apply(seat, action) // an `Err` is shown to that player
    }

    fn view(&self, seat: usize) -> GameView {
        GameView::Nim(Box::new(self.view_for(seat)))
    }
}
```

Then `pub mod nim;` and its arm in `game()`, both in `server/src/games/mod.rs`. The seat comes
from the player's connection, never from the message, so `play` can trust it.

The game's server tests go in its module, with the helpers of `server/src/testing.rs`: fake
connections, `lobby()` for a room with seated players, `with_game()` to look inside (or set up)
the running game. The room tests in `rooms.rs` already run for every `GameKind`: starting a
game with as many players as it seats and checking everyone gets a view, refusing other games'
moves, lobby options, reconnecting.

## 4. Client: `client/src/games/nim/`

The same files as the other games:

- `mod.rs`: the game's `Signals` (what it keeps between server messages: the view, the names,
  who is connected, and interface state that must survive a redraw), and `struct Nim`
  implementing the `Game` trait of `client/src/games/mod.rs` (its `snapshot` returns those
  signals as JSON, which a bug report sent from the game carries);
- `screens.rs`: the frame (the element carrying the game's class, e.g. `<div class="nim">`);
  the home screen (your name, "Create a room", a room code and "Join room", "← All games");
  the waiting room (who is here, "Start game" for the host, "Leave room"); and `icon()`, the
  menu card's picture;
- `board.rs`: the game screen, redrawn from the signals by `board(app)`, and the moves it sends
  (`send(app, &ClientMsg::Move(GameMove::Nim(..)))`);
- `look.rs`: the pure functions behind the screens (texts, what a click means), with native
  tests;
- `nim.css`: every rule scoped under the frame's class, so it can't restyle another game.

Then register it:

- `pub mod nim;`, its arm in `game()` and a `nim: nim::Signals` field in `Games`, all in
  `client/src/games/mod.rs`;
- its stylesheet in `client/index.html`:
  `<link data-trunk rel="css" href="src/games/nim/nim.css" />`;
- its frame's class and a first view in the shell tests at the bottom of `client/src/main.rs`
  (the compiler asks for both).

The shell tests then check, for the new game like the others, that the menu shows its card,
that its home screen and waiting room are drawn in its own frame with the buttons above, that
the page title follows it, and that a state from the server puts its board on screen until
you leave.

## 5. Docs

A section in `README.md` (the rules as played, where the game lives) and, when decisions were
made without a rulebook, notes in `docs/` like those for It's a Wonderful World.

## Checklist

- [ ] `games/nim/` (crate `nim-core`), declared in `[workspace.dependencies]`, used by
      `protocol`, `server` and `client`
- [ ] protocol: `GameKind`, `GameMove` and `GameView` variants (and `GameOptions`)
- [ ] server: `server/src/games/nim.rs` with `Game` and `Running`, its arm in `game()`
- [ ] client: `client/src/games/nim/` with the `Game` impl, screens, board and stylesheet; its
      arm in `game()`, its field in `Games`, its stylesheet in `index.html`, its arms in the
      shell tests
- [ ] `cargo test --workspace`, `cargo clippy --workspace --all-targets`, `trunk build`
- [ ] a round in the browser with two players
- [ ] a README section
