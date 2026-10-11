# Small World: design notes

What the README doesn't say: how the boards were made, the rule calls the rulebook leaves
open, why the code is shaped the way it is, and what is left. The README's "Small World"
section has the rules as played and where the code lives.

Status (2026-10-11): the base game runs end to end in tests for 2 to 5 players, through the
rules crate, the server and the client's render tests, and a whole two-player game plus the
first turns with four and five players were played in headless Firefox by script. The rules
follow the 2015 English rulebook
([sw_rules_2015_en.pdf](https://cdn1.daysofwonder.com/smallworld/en/img/sw_rules_2015_en.pdf)).

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

## Why the code is shaped this way

The shape is every game's (see `docs/adding-a-game.md`), with It's a Wonderful World as
the nearest model: a pure crate with its own small seeded generator (SplitMix64) instead of
`rand`, so the whole game, die rolls included, replays from the seed the server passes.

**The view carries the legal moves.** `View::moves` lists what the seat can do right now
(regions to conquer with their cost, regions within the die's reach, where tokens can go,
whether the turn can end and why not). The client highlights and labels exactly those, so
it never has to work out the rules, and a rule fixed in the crate is fixed on screen too.
`State::moves` tries the redeploy actions on a clone of the state, which is simple and
cheap enough for a few dozen regions.

**One phase at a time, one actor at a time.** `State::actor` names who must act: the current
player, or during Reinforce the first player with tokens to put back. The server just
applies a move for the seat of the connection that sent it, and broadcasts.

**The boards are data.** `maps.rs` holds topology only (terrain, features, neighbours), so
the rules crate stays small; the client's `shapes.rs` holds the outlines in the same order.

## Left to do

* A browser pass with people, especially the rarer powers (Heroes, Dragon, Bivouacking,
  Fortified, Diplomat, Sorcerers), which were tested in the rules and on the server only.
* Compare each board region by region with a physical copy, if one is at hand.
* Dialog accessibility: the die's confirmation is a plain box in the action panel.
* The expansions, if wanted: the 6-player board and the extra races and powers would be
  more variants in `races.rs` and another table in `maps.rs`.
