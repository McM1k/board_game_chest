# Small World: design notes

What the README doesn't say: how the boards were made, the rule calls the rule sheets leave
open, why the code is shaped the way it is, and what is left. The README's "Small World"
section has the rules as played and where the code lives.

Status (2026-10-11): the base game and its six expansions run end to end in tests for 2 to
5 players (3 to 6 with the Sky Islands), through the rules crate, the server and the
client's render tests. In headless Firefox, by script: a whole two-player base game, a whole
four-player game with every expansion, and the first turns with three, five and six players.
The base rules follow the 2015 English rulebook
([sw_rules_2015_en.pdf](https://cdn1.daysofwonder.com/smallworld/en/img/sw_rules_2015_en.pdf)),
the expansions Days of Wonder's own rule sheets:
[Cursed!](https://cdn1.daysofwonder.com/smallworld/en/img/sw_cu_rules_en.pdf),
[Grand Dames](https://cdn1.daysofwonder.com/smallworld/en/img/sw_gd_rules_en.pdf),
[Be Not Afraid...](https://cdn1.daysofwonder.com/smallworld/en/img/sw_bna_en.pdf),
[Royal Bonus](https://cdn1.daysofwonder.com/smallworld/en/img/sw_royal_en.pdf),
[A Spider's Web](https://cdn1.daysofwonder.com/smallworld/en/img/sw_spider_en.pdf) and
[Sky Islands](https://cdn1.daysofwonder.com/smallworld/en/img/sw_si_en.pdf).

## The boards

No open data for the printed boards was found (no equivalent of Game Park's code for It's a
Wonderful World), so the four boards were traced from photographs on BoardGameGeek:

| Board | Photograph | Regions |
|---|---|---|
| 2 players | [image 2885924](https://boardgamegeek.com/image/2885924) (angled, straightened) | 23 |
| 3 players | [image 455937](https://boardgamegeek.com/image/455937) (Days of Wonder's, small, straightened), checked against [845685](https://boardgamegeek.com/image/845685) | 30 |
| 4 players | [image 4530241](https://boardgamegeek.com/image/4530241) (flat) | 40 |
| 5 players | [image 455940](https://boardgamegeek.com/image/455940) (Days of Wonder's, small, straightened), checked against [2015727](https://boardgamegeek.com/image/2015727), [464677](https://boardgamegeek.com/image/464677) and [485025](https://boardgamegeek.com/image/485025) | 47 |

How: the white lines that separate regions were picked out of each photograph by colour
(bright, unsaturated, thin), each region was flooded from a seed point placed by hand until
the floods met on those lines, and where the board draws a border without a white line (a
cliff, a dashed line between two fields, a coast) a cut was drawn by hand. Each region's
terrain, mine, magic source, cavern and Lost Tribe were read off the photograph. Two regions
are neighbours when their floods share a border at least a few pixels long; the outlines are
the floods, simplified, with shared borders simplified once so neighbours fit exactly.

The scripts and the hand-placed data are in `docs/small-world-boards/`: `seedsN.json` (each
region's seed points, terrain, features and Lost Tribe, in board order) and `cutsN.json`
(the hand-drawn borders). To redo a board, from a folder holding the photograph:

```sh
python3 lines.py photo.png lines.png 9 70          # the white lines (3 players: 7 40 165; 5: 9 50 160)
python3 shed.py photo.png lines.png.npy seedsN.json shedN.png X0 Y0 X1 Y1 cutsN.json
python3 check.py photo.png shedN.png.npy seedsN.json check.png X0 Y0 x0 y0 x1 y1 1.0
python3 export.py shedN.png.npy seedsN.json mapN.json 50 4   # 2 players: 40 4; 3 and 5: 30 2
python3 gen.py core_maps.rs client_shapes.rs        # needs map2..5.json
```

`rectify.py` straightens an angled photograph from its four corners first (the 2-, 3- and
5-player photographs were). The two generated files become the tables at the bottom of
`games/smallworld/src/maps.rs` and `client/src/games/smallworld/shapes.rs`.

What backs the transcription:

* every board has as many mines as magic sources and caverns (4, 5, 7, 9), which the box's
  symmetric design suggests and the reading gave without forcing it;
* the Lost Tribes add up to 9, 10, 14 and 17 (the box has 18) and the Mountains to 4, 7, 8
  and 8 (the box has 9);
* every border goes both ways and every board is in one piece (tested).

Known weak spots, if a border looks wrong in play:

* the 3- and 5-player photographs are small (about 700 pixels across), so a short border
  between two regions can be missed or invented: the export ignores borders shorter than a
  few pixels;
* where the floods meet on a border with no white line, the hand-drawn cut decides;
* "border regions" (where a race enters the map) are those touching the board's frame, or
  on a sea that does. The printed boards don't mark them; this is the rulebook's definition.

### The Sky Islands

Both sides of the Sky Islands board were traced the same way, from Days of Wonder's photographs
([three islands](https://boardgamegeek.com/image/3740949), [two islands](https://boardgamegeek.com/image/3740951)).
Clouds hide the islands' edges, so their coasts were drawn by hand (`sky_outlines.py`) and only
the borders inside an island come from the white lines (`seeds-sky-three.json`,
`seeds-sky-two.json`). Then `export.py` as above, and `gen_sky.py` writes the tables (it
expects the exports as `map-sky-three.json` and `map-sky-two.json`). The cloud bridges, the
islands and where the Beanstalk and the Stairway arrive were read off the photographs and are
written in `gen_sky.py`:

| Side | Regions | Islands | Cloud bridges | Beanstalk, Stairway |
|---|---|---|---|---|
| Three islands | 9 | 2, 4 and 3 regions (the rulebook's example) | 1–3, 2–8, 5–9 | 9 (Forest), 6 (Hill) |
| Two islands | 10, one a lake | 4 and 6 regions | 2–5, 4–9 | 4 (Swamp), 10 (Forest) |

## The printed boards

The map shows a picture of the printed board by default (a switch under it goes back to the
drawn map). The regions lie on it, invisible but for what the tool in hand can act on, and
the picture shows the mines, magic sources and caverns itself. The pictures, in
`client/src/games/smallworld/art/boards`, come from BoardGameGeek:

| Board | Picture | |
|---|---|---|
| 2 players | [4530242](https://boardgamegeek.com/image/4530242) | flat scan, sharp |
| 3 players | [845685](https://boardgamegeek.com/image/845685), the corner it cuts off from [455937](https://boardgamegeek.com/image/455937) | photo at an angle, upside down |
| 4 players | [4530241](https://boardgamegeek.com/image/4530241) | flat scan, sharp |
| 5 players | [455940](https://boardgamegeek.com/image/455940) | Days of Wonder's small photo: soft |
| Sky Islands | [3740949](https://boardgamegeek.com/image/3740949), [3740951](https://boardgamegeek.com/image/3740951) | Days of Wonder's |

No better 3- or 5-player picture was found; a flat photo of a physical copy would replace
them as they are (`boards.json`).

Each picture was fitted to the outlines (`docs/small-world-art/boards/`). The board's four
corners were read off the picture (an angled photo's from zoomed crops; the 3-player board's
hidden corner from its edges). Then a scale and offset was searched that puts the borders
between regions on the picture's white lines. Only thin white lines count, so a white table
or the clouds don't. A photo at an angle then gets its perspective refined the same way, and
the picture is resampled into board units.

The traced outlines only followed the painted borders to a few units (they were simplified
on a grid, and some borders were drawn by hand). So they were moved onto the white lines: a
smooth field of small shifts, each found near a point of a 90-unit grid. Shared points move
together, so neighbours stay neighbours, and the rules' tables don't change. `shapes.rs`
holds the moved outlines.

To redo it, with the pictures in `<work>/boards/<id>.jpg`:

```sh
cd docs/small-world-art/boards
git show 592d6e4:client/src/games/smallworld/shapes.rs > <work>/shapes.rs   # the traced outlines
python3 -I export_shapes.py <work>/shapes.rs <work>/map/shapes.json
python3 -I boards.py <work>                    # pictures and moved outlines in <work>/map/out
python3 -I write_shapes.py ../../../client/src/games/smallworld/shapes.rs <work>/map/out SHAPES_2 SHAPES_3 SHAPES_4 SHAPES_5 SKY_THREE SKY_TWO
```

The pictures in `art/boards` are `<work>/map/out/<KEY>.png` saved as WebP (quality 80).

## The pictures

The race portraits and power badges (`client/src/games/smallworld/art`, served at
`/smallworld-art/`) come from the rule sheets linked above, where every banner and badge is
printed. They are small (about 120 pixels across for the base game, more for some
expansions), which is plenty for tokens and the column.

How to make them again, from `docs/small-world-art/`:

1. Download the seven PDFs into a folder of their own and export each one's images with
   their positions: `pdftohtml -xml -q <sheet>.pdf <exports>/<sheet>/doc`.
2. `python3 -I make.py <exports> client/src/games/smallworld/art`, then
   `python3 -I banners.py <exports> <scans> client/src/games/smallworld/art`, where `<scans>`
   holds the three BoardGameGeek scans below as `<id>.jpg`.

`picks.json` says which exported image is each race's and each power's. Most were matched by
a script that paired each name in the PDF with the nearest picture; the rest (the Sky
Islands, Royal Bonus's Aquatic and Behemoth, A Spider's Web's races) were picked by eye from
contact sheets. The base races are printed as round portraits on white, and the script finds
each ring where the white starts. The expansion races are printed as whole banners, and the
portrait is the medallion on their right. Its circle is the same on every banner of a sheet
(`medallions.json`): it was fitted across all of a sheet's banners at once, then checked by
eye. Royal Bonus's radius was set by hand, as the fit caught the ring's inner edge. A banner's
medallion keeps the token number printed over its corner.

The banners: the rule sheets have the expansion races' (their white surround made
transparent), but no base race's. Those come from three flatbed scans of the physical
banners on BoardGameGeek: [466842](https://boardgamegeek.com/image/466842),
[466845](https://boardgamegeek.com/image/466845) and
[466846](https://boardgamegeek.com/image/466846). `banners.py` cuts each one out and
straightens it: `banner-scans.json` holds roughly where each banner lies, found by matching
a template of what every banner shares. The rule sheets crop their banners a little tighter
than the scans' outline (the top of the medallion is cut), which only shows side by side.

The Lost Tribe's picture (`art/lost-tribe.webp`) is its token, from the base rule sheet's list
of components (page 1, image `1_38`, next to "18 Lost Tribes"; nothing larger is printed).
The token is square, 97 pixels across, framed in grey with the Lost Tribe symbol and a
column in two corners. `make.py` cuts a circle out of it that keeps the silhouette against
the sunset, head included, and leaves the grey out: centre (51, 47.5) and radius 34, in the
export's pixels (`picks.json`, `tribe`).

On the map a token is its race's portrait on a disc of its player's colour, with the token
count beside it, and a Lost Tribe is its picture on a sand-coloured disc. Tokens are 46 board
units across (1000 across the board), so a phone still shows the faces. The printed features
sit in a row above a token, the pieces in a row under it, and the cost badge on its right;
`map.rs` tests that none of them overlap. A race in decline is drawn in grey, like the
banner's grey side.

In the column and the players' list, a combo is put together as in the box: the power's
badge against the left end of the race's banner. What the race and the power do shows while
the combo is hovered, or focused from the keyboard or by a tap.

## Rules: what was chosen where the rulebook is silent

| Topic | What the engine does |
|---|---|
| Who plays first | Seat 1 after the server shuffles the seats (the rulebook: the most pointed ears). |
| Losses during someone else's turn | Each player who lost regions puts their kept tokens back after the turn, one player after another, before the next turn starts (the "Reinforce" phase). A race with nothing left on the map keeps them in hand and comes back as a new race would; a race in decline loses them. |
| Readying | At the start of a turn every region keeps one token and the rest go in hand. If nothing is conquered or abandoned the player can still go into decline: the lifted tokens are put back first. |
| Going into decline | Only before conquering or abandoning anything (Stout: also at the end of a turn of conquests, after scoring). The decline scores at once and ends the turn. |
| Ghouls in decline | Conquer first, before the active race; their own tokens are readied the same way and put back if they don't conquer. They also redeploy on their own turn. |
| Amazons | Four tokens are set aside after redeploying (fewer if they can't spare them without emptying a region) and come back when readying. |
| Halflings | The Holes-in-the-Ground go in the first two regions they ever conquer; abandoning one loses it for good. |
| Sorcerers | Only a lone active token with nothing else defending it (an encampment protects it; a lair, a fortress or a Mountain don't, as the rulebook says). |
| Diplomat | Peace lasts until the Diplomat's next turn and protects only the active race. |
| The die | Rolled only for a region within 3 tokens; the region is picked first, and a failed roll ends the conquests. Berserk rolls before any conquest instead, one roll per conquest. |
| Tokens from the box | A race's tokens are capped by what the box holds (e.g. 20 Skeletons, 18 Sorcerers). |
| The column | Combos are drawn from a shuffled stack of banners and one of badges; a vanished race's banner goes under the stack, used badges are reshuffled when the stack runs out. |
| Ties | Most coins, then most tokens on the map; still level shares the win. |

## The expansions: what was chosen

The rule sheets are a paragraph per race and power; these are the calls made around them.

| Race or power | What the engine does |
|---|---|
| Token supplies | The rule sheets don't count the tokens in the box. Races that never gain tokens get their banner's number plus five, as in the base game (which matches every count found: Kobolds 16, Goblins 11, Gypsies 11, Priestesses 9, White Ladies 7, Leprechauns 11, and the Sky Islands' races). Estimated from the boxes' totals: Fauns 15 and Igors 15 (Royal Bonus's 41 tokens less the Shrubmen's 11), Homunculi 11 and Pygmies 15 (Be Not Afraid... and A Spider's Web hold 98). |
| Hordes of | The two Hordes tokens are two more tokens of the race, not taken from its supply, gone when it declines. |
| Marauding | The second round starts when the first is declared done; the die is only offered in the second. |
| Priestesses | Going into decline with them asks where the Ivory Tower goes (the "Ivory Tower" tool); the plain decline button puts it on a Fortress, else a Mountain, else the first region. |
| Homunculi | A skipped Homunculi combo gets a token only while the box has one to spare. |
| Leprechauns | 16 Pots of Gold; a pot can be taken off again while redeploying. Pots pay at the start of the owner's next turn, in decline too. |
| Pygmies | Only while active, and for each token lost for good (not for those kept in hand). |
| Catapult | The interface asks only for the target; the Catapult fires from the most exposed region that can reach it (fewest tokens). It stays there, protecting it, until it fires again or the race declines. |
| Mercenary | Paying for the final conquest after the die: done for you when the die falls short by no more than two and you have a coin. |
| Behemoth | Behemoths come back in hand when the troops are readied, attack from hand, and are placed while redeploying. One whose region is taken goes back to its owner's hand (rather than being placed at the end of the attacker's turn). |
| Fireball | At most 4 held. A conquest uses the fewest Fireballs that make it as cheap as possible. |
| Ice Witches | Winter markers are counted when the conquests end and placed while redeploying; they stay on a region whoever holds it, until the Ice Witches decline. |
| Skags | The ten markers are 0, 0, 1, 1, 2, 2, 3 coins and three Skag Attacks (a published review's count). Loot left in an abandoned region goes with whoever takes the region next; a Skag Attack there does nothing. A Skag Attack also stops Dragons, Sorcerers and Zeppelins. |
| Slingmen | The coin comes with any conquest made from two regions away. |
| Copycat | The copy is picked before anything moves, from the six combos; it ends at the start of the next turn or when someone picks that combo, taking the copied power's pieces with it (a copied Seafaring's tokens leave the water). Copied Wealthy, Spirit and Soul-Touch do nothing. |
| Lava | One Lava per Mountain held (four at most), on a region next to one of them; the tokens there go back in hand. |
| Soul-Touch | As Days of Wonder answered: the race in decline comes back with its own power and as many tokens as when it was picked, and plays on the next turn. |
| Escargots | The start-of-turn coins are the regions' own (less Winter); what the power adds is paid at the end. |
| Scavengers | A race in decline keeps scoring the regions where the Scavengers hold its token, and doesn't leave the map while it has one. |
| Storm Giants | The Lightning Bolts take a Mountain within the usual reach. |
| Drakons | Each Dragon costs a token turned into it and one to hold the region. Their Dragons fly away at the start of the next turn. |
| Zeppelined | A crash throws the defenders out as a conquest would (a token lost, the rest in hand) and the region burns until the start of the zeppelin's owner's next turn. |
| Haggling | Pacts are settled at the end of each holder's turn, with the player whose active race is Haggling: a coin per pact to the haggler if the holder took one of that race's regions, else to the holder. |
| Gunner | Both Cannons are placed at the end of every turn (one with a single region); a fired Cannon can't move that turn; all stand down at the start of the next. |
| Racketeering | Paid for skips by anyone, the free pick included. |
| Sky Islands | Played on the board for one player fewer, so with 3 to 6 players; the turn track is the base game's for 3 to 5 players, 8 turns for 6. The Beanstalk and the Stairway go on two ground regions away from the edge and the water, at least three steps apart. An island held whole by one race is worth a coin to that race (the lake doesn't count). |

## Why the code is shaped this way

The shape is every game's (see `docs/adding-a-game.md`), with It's a Wonderful World as
the nearest model: a pure crate with its own small seeded generator (SplitMix64) instead of
`rand`, so the whole game, die rolls included, replays from the seed the server passes.

**The view carries the legal moves.** `View::moves` lists what the seat can do right now: a
list of moves on regions (`RegionMove` and the region, with the tokens it costs), and the
other choices (picks with their cost, copying, peace, Trade Pacts, whether the turn can end
and why not). The client highlights and labels exactly those, so
it never has to work out the rules, and a rule fixed in the crate is fixed on screen too.
`State::check` validates a move on a region without changing anything; `apply` and `moves`
both go through it, so what is offered is exactly what is accepted.

**One phase at a time, one actor at a time.** `State::actor` names who must act: the current
player, or during Reinforce the first player with tokens to put back. The server just
applies a move for the seat of the connection that sent it, and broadcasts.

**The boards are data.** `maps.rs` holds topology only (terrain, features, neighbours), so
the rules crate stays small; the client's `shapes.rs` holds the outlines in the same order. A
game plays on a `Board` built from them: the ground board, then the Sky Islands' regions
numbered after it, with the Beanstalk and the Stairway joining the two. The view carries the
ground board's size, the side and the access regions, so the client builds the same `Board`.

## Left to do

* A browser pass with people, especially the powers with their own tools (Heroes, Dragons,
  Bivouacking, Fortified, Diplomat, Sorcerers, and the expansions' Catapult, Mercenaries,
  Fireballs, Behemoths, Lava, Winter, Pots of Gold, Cannons, Zeppelins, Copycat and Trade
  Pacts), which were tested in the rules, on the server and in render tests, but clicked
  through in a browser only by a script that conquers.
* Compare each board region by region with a physical copy, if one is at hand.
* Dialog accessibility: the die's confirmation is a plain box in the action panel.
* The expansions not in: the 6-player board, Underground, Realms, Necromancer Island, Tales and
  Legends, Leaders, and the blank banners and badges.
