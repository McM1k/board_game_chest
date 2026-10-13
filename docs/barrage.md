# Barrage: sources and rule calls

Barrage (Cranio Creations, 2019, by Tommaso Battista and Simone Luciani) is played here as the
rulebook's **full game** for 2 to 4 players: Executive Officers, the Patent Office with its
Advanced Technology tiles, random Bonus and Objective tiles, and the picking of Companies,
officers and Starting Contracts. The introductory setup, the solo Automa, the Leeghwater
Project expansion and the promo officers are not in.

## Where the numbers come from

* **Rules**: the 2019 English rulebook (28 pages; a copy is at boardgame.bg). Every rule in
  `games/barrage/src/state.rs` follows it, appendices included (Company boards, officers,
  Advanced tiles, Bonus and Objective tiles).
* **The map**: the rulebook's counts of spaces per basin, BoardGameGeek images 5302816 (the
  twelve basins) and 4936955 (a fan's connection graph of rivers and conduits, with each
  basin's best path), checked against the conduit values printed on the board. The table is
  `games/barrage/src/map.rs`: rivers M1→P1, M2→H1, M3→H2, M4→H3, H1→P2, H2→P3, H3→P3,
  P1→B1, P2→B1, P3→B2; twenty conduits (two 1s, four 2s, seven 3s, four 4s, three 5s).
* **Red (paid) spaces**: in every basin with dams, the upper dam space; one powerhouse space
  of each Hills basin, one of P1 and P2, two of P3 and B1, three of B2 (read off the board).
* **Contracts**: BoardGameGeek photos 4924638/39/40/41 of every green, yellow, red and
  national tile (a fan site's data had a few wrong). The four Starting Contracts and the
  eight Headstream tiles come from the Pantherrochen/Barrage-Random-Setup repository's scans.
* **Company boards**: BoardGameGeek photo 5246940 of the four empty boards (every income).
* **Management board**: the rulebook's pictures and BoardGameGeek photo 5725788.

## Rule calls

Where the rulebook or the photos left something open:

* **The Energy Track's Credits** couldn't be read in any picture. The rulebook's example
  (3 Credits at 4 energy, 4 at 8) and the spacing of the printed coins give: 1 Credit from 1
  energy, 2 from 2, 3 from 4, 4 from 7, 5 from 11, 6 from 16, 7 from 22, 8 from 29; and, as
  the rulebook says, 3 Credits but 3 VP lost for 0.
* **Machinery Shop**: 2 Credits for an Excavator, 4 for a Machinery of your choice (as Board
  Game Arena's help describes it), 5 for one of each. **Workshop**: free for one turn of the
  wheel, 2 Credits for two, 5 for three.
* **Neutral Dams** stand on the lower (free) space of their basin, as the setup tiles show;
  the upper space of a basin, red and 3 Credits dearer, catches the water first.
* **Mahiri Sekibo** copies any of the six other base-game officers, in play or not. Her
  Engineer goes on her tile, the copy lasts the turn, and she then takes her turn's action
  as usual (the copy is pointless otherwise). She starts with 6 Credits like everybody.
* **Free buildings** (Contract rewards) still pay 3 Credits on a red space; they need no
  Engineers, tile or Machineries, and uncover incomes as usual.
* **Drops a reward places** can be fewer than it allows, as the Water Management action
  explicitly lets you.
* **II Powerhouse's production** is a production action: the Company board's bonus, Italy's
  +3 and Germany's second production apply. **II Base's and III Conduit's energy** isn't: no
  bonus, no ability, but it counts for the round and can fulfill a Contract.
* **A Contract is chosen after the drops of the production have flowed on** (it doesn't
  matter for the energy; it matters if the reward places water).
* **The Objective** gives nothing to a count of 0; ties share the VP of their places,
  rounded up.
* **Using somebody's Conduit** needs the Credits for it (1 a drop): the drops you can send are
  capped by your Credits after the action space's own fee.
* **VP never go below 0**; Credits and Machineries are unlimited.
* With **no Engineers left**, a player still gets their turn while they can build with a tile
  that needs none (I Powerhouse, III Base, or Anton Krylov's tile copying one), and passes
  otherwise.

## Testing

`cargo test -p barrage-core` runs the rules (33 tests, random whole games for 2 to 4 players
checked after every move). `cargo test -p barrage-core game_report -- --ignored --nocapture`
prints what random bots do. A whole two-player game was also played through the browser by a
script clicking the interface.
