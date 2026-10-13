# It's a Wonderful World: design notes

What the README doesn't say: the rule calls that were made without the rulebook, why the
code is shaped the way it is, what is known to be wrong, and what is left. The README's
"It's a Wonderful World" section has the rules as played and where the code lives.

Status (2026-10-11): the base game and its Corruption & Ascension expansion run end to end in
tests, from the lobby to the final standings, for 2 to 5 players (2 to 7 with the expansion),
and the start of a game of each has been played in a browser by script. The cards and the
Empires are the published ones (side A of the Empires), taken from Game Park's online version
([gamepark/its-a-wonderful-world](https://github.com/gamepark/its-a-wonderful-world),
`rules/src/material/Developments.ts`, `AscensionDevelopments.ts` and `Empires.ts`).

## Rules: what is confirmed and what was chosen

Confirmed against a third-party rules summary (Ultraboardgames): four rounds of draft,
planning and production; 7 cards per hand, or 10 with two players, where the 3 left over are
discarded without their recycling bonus; hands pass to the next seat in rounds 1 and 3 and
to the previous one in rounds 2 and 4; a recycled card's cube goes on a building or the
Empire; five Empire cubes make one Krystallium, which replaces any resource cube but never a
character; the strictly highest producer takes the character; the score is printed points,
type combos and 1 per character; Krystallium and unfinished cards score nothing; ties go to
most finished cards, then most characters, then are shared.

Confirmed against Game Park's rules code and Board Game Arena's game help: Materials and Gold
give a Financier, Energy and Exploration a General, Science the winner's choice (both sources
agree; the first version of this engine had Materials and Exploration the wrong way round);
a tie for the most gives nothing; production is counted at the start of each step;
construction bonuses are only characters and Krystallium, kept until used; some cost spaces
take Krystallium and nothing else; Krystallium and characters can be placed in Planning and in
any production step; a scrapped building's recycling cube goes on the Empire.

Corruption & Ascension follows its official rulebook
([corruptionascension_rules_EN.pdf](https://www.laboitedejeu.fr/wp-content/uploads/2021/02/corruptionascension_rules_EN.pdf)):
the hand sizes per player count, corruption subtracted from production (never below zero,
for the supremacy too), duo scoring, and the supremacy for six and seven players (two
places; two tied for first share them, a tie for second leaves the second place empty, three
tied for first give nothing; each Science winner chooses). Its two Empires are dealt with the
others: the box's Empires are now dealt at random, where seat *i* used to always play Empire
*i*, which left the last Empires out of smaller games.

Where the engine fills a gap or picks a convenience:

| Topic | What the engine does | Status |
|---|---|---|
| Wrap-up step | A sixth production step after Exploration to place Krystallium and characters still held. Skipped when nobody holds one that fits a free space. | Not in the published game, where these can be placed during any step before you end it. Needed here because a player whose cubes are all placed is done with a step at once. |
| Scrap | Allowed in Planning or any production step. Everything on the building is lost, Krystallium and characters included; its recycling cube goes on the Empire. | Follows the rules summaries (Ultraboardgames, BGA). Game Park gives Krystallium and characters back, and treats a card drafted this round as if it were being recycled. |
| Empires | Side A of the five base Empires (seven with the expansion), each with its production and its end-game bonus, dealt at random. | Side B (the same production for everybody, no bonus) would be another `Options` field beside `ascension`. |
| New cards producing | Production is counted at the start of each step: a card finished in step *k* first produces in step *k+1*; one finished in Planning produces in Materials the same round. | Confirmed (Game Park). |
| Ending a step | A player is done when their pool is empty and no choice is pending, or when they press Done (which drops unplaced cubes). A step where nobody has anything to place is skipped. | Pace: nobody waits on an empty step. |

Edge cases:

* Everyone producing zero of a resource means no supremacy and no log line; a tie for the
  most also gives nothing.
* The Science winner must choose General or Financier before Done is accepted.
* The tie-break counts unspent characters only; one placed on a building no longer counts.
* The deck is never reshuffled: five players use 140 of the 150 cards.
* The round counter stays at 4 when the game is over.

## Why the code is shaped this way

The shape is every game's (see `docs/adding-a-game.md`): a pure rules crate, a variant in
the protocol's per-game enums, a module behind the server's `Game` and `Running` traits, and
a client folder behind the client's `Game` trait. Where nothing was specified, the nearest
existing game was copied: Hanabi for the crate and protocol, Catan for the screen lifecycle,
Hanabi's scoped stylesheet for the CSS.

**Rules crate.** `wonderful-core` depends on serde alone. The only randomness is the opening
shuffle, so the server passes a seed and `apply` takes no RNG (Catan differs because it
rolls dice mid-game). The shuffle is a SplitMix64 in `state.rs` rather than `rand`, which keeps
the crate light for the protocol and client and makes any game replayable from its seed; most
tests rely on that. Only `View` is serializable, not `State`, since rooms live in memory.

**The engine moves the game on, not the server.** Every accepted move ends in `settle()`,
which keeps advancing while everyone is done with the current step (next draft pick, Planning
to Production, next step, next round, the end). The server needs no timers, no "next" message
and no phase logic: it applies a move and broadcasts. The price is that nothing hurries a slow
player.

**Hidden information stays in `State`.** Each client gets only `view_for(seat)`. Other players
appear as counts; a draft step's picks are revealed together when the last one arrives. The
view carries each player's production per resource and current points so the client doesn't
redo production or scoring.

**Server.** The seat comes from the connection's token, never from the message. A refused move
sends an error to the sender only. An accepted one sends every connected seat its full view
(not a diff), so a reconnect is just a rejoin. Seats are shuffled at Start (`shuffles_seats`,
like every other game) because seat order decides passing direction and Empire; after Start, seat 0
need not be the host.

**Protocol.**

* `ClientMsg::Move(GameMove::Wonderful(Action))` wraps the crate's own `Action`, as for the
  other games.
* `ServerMsg::Game { view: GameView::Wonderful(..), names, connected }`: names and connection
  flags travel beside the view, because the rules crate knows nothing about people.
* Cards travel as `CardId`s and each side looks them up in its own copy of the catalogue, so
  server and client must be built from the same one.
* Views contain no maps (vectors and arrays indexed by seat or resource), because JSON keys
  must be strings.
* Lobby options travel as `GameOptions::Wonderful(wonderful_core::Options)`, so far only
  `ascension`. The server starts every lobby with it off (`default_options()` in
  `server/src/games/wonderful.rs`), and the room's capacity follows it
  (`GameOptions::max_players`: 5, or 7 with the expansion). Empire side B would be another
  field of `Options`.

**Client.**

* The game screen is redrawn on every message from the view kept in `Signals`
  (`client/src/games/wonderful/mod.rs`), as Catan's is, rather than built once like Hanabi's
  (which protects animations Wonderful doesn't have). Interface state that must outlive a
  redraw (the piece picked in the tray, a recycled card waiting for a target) lives in its
  `Ui` and is re-checked against each new view: `effective_held` falls back to the first piece held when the chosen one is gone,
  and `CubeFrom::valid` hides the dialog once its card is gone.
* Browser code is kept thin. What a click means is decided by pure functions in `look.rs`
  with native tests (`pick_for_space`, `pick_for_empire`, `cube_targets`); the closures in
  `board.rs` only call `act(action)`.
* Placing: clicking a free space uses the plain piece for it (that resource's cube, a
  General, a Financier, Krystallium on a Krystallium space); picking a piece in the tray first
  overrides that. On a resource's space Krystallium is used only when picked, since it fits
  anything and is easy to waste. Recycle opens a dialog for the cube's target, because the
  target is part of the move; Scrap's dialog only confirms, its cube always goes on the Empire.
* `client/src/games/wonderful/wonderful.css` scopes every rule under `.wonderful`; generic element rules sit in
  `:where()` so the prefix adds no specificity. Checked rendered (not clicked) at 1280, 390,
  360 and 320 px; below 560 px the standings table stacks, because a scrolling table hid the
  Total column. The only change to the other games' look: the menu's max width went from
  760 to 1000 px so three cards fit in a row.

## The pictures

Every card shows its printed illustration, and your Empire its board's (a player asked for
the original game's look, 2026-10-12). They come from Game Park's online version, the same
repository as the numbers (commit `3627484`, 2026-09-27), and are served at `/wonderful-art/`
from `client/src/games/wonderful/art` (`look::card_art`, `empire_art`, `crest_art`,
`token_art`):

| Pictures | Source in `app/src/images` | Ours |
|---|---|---|
| The cards, all 116 designs (78 base, 38 Corruption & Ascension) | `cards/developments/en/<Type>_<n>_EN_<Name>.jpg`: each printed face, 650 x 1000 | `art/cards/<slug>.webp`: a 2:1 band, 480 x 240, about 11 KB |
| The seven Empires | `empires/<name>-artwork.jpg`: the board's illustration alone, 1920 x 1080 | `art/empires/<slug>.webp`: a 3:1 banner from its middle, 1200 x 400, 30 to 46 KB |
| Their crests | `empires/<name>-avatar.png`, 100 x 100 | `art/crests/<slug>.webp`, 96 x 96, 2 to 4 KB |
| The General and Financier tokens | `characters/general.jpg`, `financier.jpg`: the round token on white, 150 x 150 | `art/tokens/general.webp`, `financier.webp`: cut out round, 64 x 64, about 2 KB |
| The round tracker | `round-tracker-recto.png` (green), `round-tracker-verso.png` (pink): a ring of arrows, 200 x 200 | `art/round/next.webp`, `previous.webp`, 112 x 112, about 5 KB |

A slug is the name in lower case, apostrophes dropped and anything else between words a
hyphen ("Pandora's Box" is `pandoras-box`). The pictures weigh 1.6 MB in all, but a page
only loads what it shows (a hand is 80 to 130 KB the first time) and the card pictures are
`loading="lazy"`. A test (`every_card_empire_and_token_has_its_picture`) checks there is one
per design, per Empire and per token, and nothing else.

How to make them again, from `docs/wonderful-world-art/`:

```sh
git clone --depth 1 --filter=blob:none --sparse https://github.com/gamepark/its-a-wonderful-world <gamepark>
git -C <gamepark> sparse-checkout set app/src/images/cards/developments/en app/src/images/empires app/src/images/characters
python3 -I cards.py <gamepark>/app/src/images games/wonderful/src/cards.rs client/src/games/wonderful/art/cards
python3 -I empires.py <gamepark>/app/src/images games/wonderful/src/cards.rs client/src/games/wonderful/art
python3 -I tokens.py <gamepark>/app/src/images client/src/games/wonderful/art
python3 -I round.py <gamepark>/app/src/images client/src/games/wonderful/art
```

The printed face has the game's own icons over its illustration, which `cards.py` keeps out:
the title bar at the top, the column of cost spaces down the left, the construction bonus
and the recycling cube at the foot of the picture. What is left is x 128 to 650 and y 125 to
725. A card with more than nine cost spaces prints a second column beside the first (x 128
to 245, one space every 95 pixels from y 140); its band goes under that column, or beside
both when there is no room under it. `cards.py` counts each design's spaces in `cards.rs`.
The band is as wide as the free area and halfway down it, unless `focus.json` says
otherwise: under `at`, 0 puts it at the top of the free area and 1 at the bottom (31 cards,
mostly to keep faces in); under `beside_second_column`, cards whose band is better beside
both columns (Dark Matter's portal, Giant Robot's head). They were chosen by eye from contact
sheets of every band drawn over its card. Game Park's file names drop the spaces and
apostrophes ("PandoraBox" is the one `cards.py` must be told).

On screen: the band sits under each card's header, with the card's name in white on a dark
fade over its foot; the costs, what the card makes and is worth, and the header's type,
expansion mark and recycling cube are on the pale card as before. The narrow grids ("Kept
so far", your Empire's finished cards) use a taller 8:5 crop of the same picture and a
smaller name. Your Empire's heading lies across its banner, darkened behind the words on the
left and the points on the right; the crest stands by your Empire's title, by each rival's
name, and by each name in the final standings (with the Empire's name as its text). The
columns of your Empire's types no longer stretch to fill the panel (`auto-fill`), so that a
lone finished card's picture stays the size of a card. Every General and Financier (on a card's cost and
the spaces of your buildings, in your hand of things to place, in each player's tally, as a
card's reward or what it scores per) is the printed round token (a player asked, 2026-10-12),
laid over our coloured sign and its letter: if the picture fails to load it removes itself and
the sign shows again (the CSS keys on `:has(> .token-art)`). An empty space shows it faded in a
dashed ring. The round number sits in the round tracker's ring, green when the hands go to the
next seat (rounds 1 and 3), pink when they go to the previous one, as Game Park shows it. In
production, each resource's step shows the character it gives the top producer (both tokens for
Science), then who took it, or "nobody" after a tie (a player asked, 2026-10-13); the engine keeps
this round's winners per step in `awards`, since the log only holds the last 24 events. The
pictures belong to the
publisher, La Boîte de Jeu; Game Park's licence covers its code, not the art.

## Known issues

* **"Leave game" strands the seat.** The client forgets its token, but the server keeps the
  seat, so nobody can take it back and everyone waits at the next step. The room is cleared
  30 minutes after the last player disconnects. The other games stall the same way, but only
  on that player's turn.
* **An offline player blocks everyone.** Nothing plays for a dropped seat and the server has
  no timer or kick. A host-only "skip this seat" or a timeout would both need the engine to
  play a step on a seat's behalf, which it can't do today. Ask before building either.
* **Only bots have played it.** The ignored `economy_report` test has two simple bots finish
  about 8 cards and 15 to 30 points with the real cards; people score far more.
* **Dialog accessibility.** The recycle/scrap dialog has `role="dialog"` and `aria-modal`
  but no Escape key and no focus handling.

## Left to do, in order

1. **Play it in a browser** with people (checklist below), with and without the expansion,
   and with six or seven players.
2. **Dialog:** Escape closes it, focus moves in on open and back on close.
3. **Absent players:** decide what should happen (see Known issues).
4. **Empire side B:** another lobby option. The campaign expansions (War or Peace, Leisure &
   Decadence) are scenarios with rules of their own, a much bigger job. Solo mode isn't
   planned either.

### The browser pass

* Menu → It's a Wonderful World. Create a room in one tab, join from a second tab or a phone,
  start. Two players is quickest.
* Draft: click a card; the heading becomes "Your pick is in". The next hand arrives when
  everyone has picked, passing to the next seat in rounds 1 and 3, the previous in 2 and 4.
* Planning: Build and "Recycle for" on each card. Recycling opens a dialog listing buildings
  that still need that cube, plus the Empire. "Done planning" stays disabled until every card
  is decided. Try Scrap on a building (its dialog only confirms: the cube goes to the Empire).
* Production: the step bar and race list. Click a free space (it lights up when what you hold
  fits). Pick another piece in the tray first to override. Try "Put on your Empire" and
  "Done: drop N cubes". A Science win shows "Take a General" / "Take a Financier" and blocks
  Done until you choose. Wrap-up appears only when someone can use it.
* Reload mid-game: you return to your seat with your own cards.
* A real phone: the sticky tray, tap targets, stacked standings, fonts.
* Other players' moves while you scroll or have the dialog open: does the page jump, does the
  dialog stay?

## Traps

* **Four bots drive the whole-game tests**: `bot` and `smart_bot` in
  `games/wonderful/src/tests.rs`, `next_move` in `server/src/games/wonderful.rs`, `next_move` in
  `client/src/games/wonderful/board.rs`. After a rules change a bot may stall or loop instead of
  failing clearly. Suspect the bots first.
* **Every new kind of move must go through `apply`**, which ends in `settle()`. Anything else
  can leave the game with everyone done and nobody advancing.
* **Anything added to `PlayerView` goes to every seat.** The hidden-info tests
  (`wonderful_hides_picks_until_everybody_has_picked`, `nothing_of_the_others_hands_is_drawn`)
  only catch what they know to look for.
* **Render tests are substring checks** on Leptos's server-side markup (attribute order, `<!>`
  between text nodes). After a markup tweak a failure is more often a stale assertion than a
  bug. Scope assertions to an element: the race line says "takes a General", so a bare
  `contains("General")` passes for the wrong reason.
* **A page-level overflow check misses inner scrollers**: the phone standings table hid its
  Total column inside a scroll box while the page never scrolled sideways.
