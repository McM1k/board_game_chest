//! The four boards of the box, one per number of players, as regions and
//! who borders whom. Transcribed from photographs of the printed boards
//! (see `docs/small-world.md`); the outlines the client draws are in its
//! own `shapes.rs`, with the regions in the same order.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Terrain {
    Farmland,
    Forest,
    Hill,
    Swamp,
    Mountain,
    Sea,
    Lake,
}

impl Terrain {
    pub fn name(self) -> &'static str {
        match self {
            Terrain::Farmland => "Farmland",
            Terrain::Forest => "Forest",
            Terrain::Hill => "Hill",
            Terrain::Swamp => "Swamp",
            Terrain::Mountain => "Mountain",
            Terrain::Sea => "Sea",
            Terrain::Lake => "Lake",
        }
    }

    /// Seas and the lake: only Seafaring races go there.
    pub fn is_water(self) -> bool {
        matches!(self, Terrain::Sea | Terrain::Lake)
    }
}

const MINE: u8 = 1;
const MAGIC: u8 = 2;
const CAVERN: u8 = 4;
const TRIBE: u8 = 8;
const EDGE: u8 = 16;

/// One region of a board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub terrain: Terrain,
    flags: u8,
    /// The regions it shares a border with.
    pub adjacent: &'static [u8],
}

impl Region {
    const fn new(terrain: Terrain, flags: u8, adjacent: &'static [u8]) -> Region {
        Region { terrain, flags, adjacent }
    }

    pub fn mine(&self) -> bool {
        self.flags & MINE != 0
    }

    pub fn magic(&self) -> bool {
        self.flags & MAGIC != 0
    }

    pub fn cavern(&self) -> bool {
        self.flags & CAVERN != 0
    }

    /// A Lost Tribe starts here.
    pub fn lost_tribe(&self) -> bool {
        self.flags & TRIBE != 0
    }

    /// Touches the edge of the board.
    pub fn edge(&self) -> bool {
        self.flags & EDGE != 0
    }

    /// A Mountain token sits here for the whole game.
    pub fn mountain(&self) -> bool {
        self.terrain == Terrain::Mountain
    }
}

/// A board: its regions and how many turns a game on it lasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Map {
    pub players: usize,
    pub turns: u8,
    pub regions: &'static [Region],
}

impl Map {
    /// The board for `players` players (2 to 5).
    pub fn for_players(players: usize) -> &'static Map {
        MAPS.iter().find(|m| m.players == players).expect("the boards are for 2 to 5 players")
    }

    pub fn adjacent(&self, a: usize, b: usize) -> bool {
        self.regions[a].adjacent.contains(&(b as u8))
    }

    /// Next to a sea or the lake.
    pub fn coastal(&self, region: usize) -> bool {
        self.regions[region].adjacent.iter().any(|&n| self.regions[n as usize].terrain.is_water())
    }

    /// Where a race may enter the map: a region on the edge of the board,
    /// or one whose shore is on a sea that reaches the edge.
    pub fn border(&self, region: usize) -> bool {
        let r = &self.regions[region];
        r.edge()
            || r.adjacent.iter().any(|&n| {
                let n = &self.regions[n as usize];
                n.terrain.is_water() && n.edge()
            })
    }
}

pub static MAPS: [Map; 4] = [
    Map { players: 2, turns: 10, regions: MAP_2 },
    Map { players: 3, turns: 10, regions: MAP_3 },
    Map { players: 4, turns: 9, regions: MAP_4 },
    Map { players: 5, turns: 8, regions: MAP_5 },
];

use Terrain::*;

/// The 2-player board: 23 regions, 10 turns.
const MAP_2: &[Region] = &[
    Region::new(Sea, EDGE, &[1, 2]), // 0
    Region::new(Mountain, MINE | CAVERN | EDGE, &[0, 2, 12, 13]), // 1
    Region::new(Farmland, MAGIC | EDGE, &[0, 1, 3, 12]), // 2
    Region::new(Forest, MINE | EDGE, &[2, 4, 5, 11, 12]), // 3
    Region::new(Mountain, 0, &[3, 5, 6, 10, 11, 15]), // 4
    Region::new(Swamp, CAVERN | TRIBE | EDGE, &[3, 4, 6, 7]), // 5
    Region::new(Farmland, 0, &[4, 5, 7, 8, 10]), // 6
    Region::new(Hill, EDGE, &[5, 6, 8]), // 7
    Region::new(Forest, MAGIC | TRIBE | EDGE, &[6, 7, 9, 10]), // 8
    Region::new(Mountain, MINE | EDGE, &[8, 10, 21, 22]), // 9
    Region::new(Hill, CAVERN | TRIBE, &[4, 6, 8, 9, 15, 20, 21]), // 10
    Region::new(Lake, 0, &[3, 4, 12, 14, 15]), // 11
    Region::new(Hill, TRIBE, &[1, 2, 3, 11, 13, 14]), // 12
    Region::new(Farmland, TRIBE | EDGE, &[1, 12, 14, 16, 17]), // 13
    Region::new(Forest, TRIBE, &[11, 12, 13, 15, 17, 18]), // 14
    Region::new(Farmland, MAGIC | TRIBE, &[4, 10, 11, 14, 18, 19, 20]), // 15
    Region::new(Swamp, MAGIC | TRIBE | EDGE, &[13, 17]), // 16
    Region::new(Hill, CAVERN | EDGE, &[13, 14, 16, 18]), // 17
    Region::new(Swamp, MINE | TRIBE | EDGE, &[14, 15, 17, 19]), // 18
    Region::new(Mountain, EDGE, &[15, 18, 20]), // 19
    Region::new(Swamp, EDGE, &[10, 15, 19, 21, 22]), // 20
    Region::new(Forest, 0, &[9, 10, 20, 22]), // 21
    Region::new(Sea, EDGE, &[9, 20, 21]), // 22
];

/// The 3-player board: 30 regions, 10 turns.
const MAP_3: &[Region] = &[
    Region::new(Sea, EDGE, &[1, 5]), // 0
    Region::new(Farmland, MAGIC | TRIBE | EDGE, &[0, 2, 4, 5]), // 1
    Region::new(Forest, MINE | EDGE, &[1, 3, 4]), // 2
    Region::new(Mountain, EDGE, &[2, 4, 7, 8]), // 3
    Region::new(Swamp, 0, &[1, 2, 3, 5, 6, 7]), // 4
    Region::new(Mountain, MINE | CAVERN | EDGE, &[0, 1, 4, 6, 21]), // 5
    Region::new(Swamp, TRIBE, &[4, 5, 7, 17, 21, 22]), // 6
    Region::new(Hill, MAGIC | TRIBE, &[3, 4, 6, 8, 9, 17]), // 7
    Region::new(Farmland, EDGE, &[3, 7, 9, 10]), // 8
    Region::new(Mountain, MINE, &[7, 8, 10, 11, 16, 17]), // 9
    Region::new(Swamp, CAVERN | EDGE, &[8, 9, 11, 12]), // 10
    Region::new(Farmland, TRIBE, &[9, 10, 12, 13, 15, 16]), // 11
    Region::new(Forest, TRIBE | EDGE, &[10, 11, 13]), // 12
    Region::new(Swamp, MINE | EDGE, &[11, 12, 14, 15]), // 13
    Region::new(Mountain, EDGE, &[13, 15, 19, 29]), // 14
    Region::new(Hill, MAGIC, &[11, 13, 14, 16, 18, 19]), // 15
    Region::new(Mountain, CAVERN, &[9, 11, 15, 17, 18, 20]), // 16
    Region::new(Lake, 0, &[6, 7, 9, 16, 20, 22, 26]), // 17
    Region::new(Farmland, TRIBE, &[15, 16, 19, 20, 28]), // 18
    Region::new(Hill, CAVERN | TRIBE, &[14, 15, 18, 28, 29]), // 19
    Region::new(Swamp, MAGIC | TRIBE, &[16, 17, 18, 26, 27, 28]), // 20
    Region::new(Farmland, MAGIC | EDGE, &[5, 6, 22, 23, 24]), // 21
    Region::new(Forest, CAVERN | TRIBE, &[6, 17, 21, 24, 25, 26]), // 22
    Region::new(Hill, EDGE, &[21, 24]), // 23
    Region::new(Mountain, EDGE, &[21, 22, 23, 25]), // 24
    Region::new(Mountain, EDGE, &[22, 24, 26]), // 25
    Region::new(Hill, EDGE, &[17, 20, 22, 25, 27]), // 26
    Region::new(Forest, EDGE, &[20, 26, 28]), // 27
    Region::new(Forest, MINE | TRIBE | EDGE, &[18, 19, 20, 27, 29]), // 28
    Region::new(Sea, EDGE, &[14, 19, 28]), // 29
];

/// The 4-player board: 40 regions, 9 turns.
const MAP_4: &[Region] = &[
    Region::new(Farmland, 0, &[5, 20]), // 0
    Region::new(Forest, CAVERN | TRIBE | EDGE, &[2, 3, 7, 20]), // 1
    Region::new(Farmland, MINE, &[1, 3, 4, 20]), // 2
    Region::new(Farmland, TRIBE, &[1, 2, 4, 6]), // 3
    Region::new(Hill, CAVERN | TRIBE, &[2, 3, 5, 6, 16, 17, 20]), // 4
    Region::new(Swamp, MAGIC | TRIBE, &[0, 4, 16, 20]), // 5
    Region::new(Farmland, 0, &[3, 4, 7, 9, 13, 17, 18]), // 6
    Region::new(Hill, TRIBE | EDGE, &[1, 6, 8, 9]), // 7
    Region::new(Swamp, MAGIC | EDGE, &[7, 9, 10]), // 8
    Region::new(Mountain, MINE, &[6, 7, 8, 11, 13]), // 9
    Region::new(Forest, CAVERN | EDGE, &[8, 11, 12]), // 10
    Region::new(Forest, MAGIC | TRIBE, &[9, 10, 12, 13, 14]), // 11
    Region::new(Mountain, EDGE, &[10, 11, 14, 15]), // 12
    Region::new(Lake, 0, &[6, 9, 11, 14, 18, 22, 26, 32]), // 13
    Region::new(Swamp, MINE, &[11, 12, 13, 15, 32]), // 14
    Region::new(Farmland, EDGE, &[12, 14, 33, 34]), // 15
    Region::new(Forest, 0, &[4, 5, 17, 19, 20, 21]), // 16
    Region::new(Swamp, TRIBE, &[4, 6, 16, 18, 21, 22]), // 17
    Region::new(Hill, MAGIC, &[6, 13, 17, 22]), // 18
    Region::new(Mountain, EDGE, &[16, 20, 21, 23]), // 19
    Region::new(Sea, EDGE, &[0, 1, 2, 4, 5, 16, 19]), // 20
    Region::new(Hill, 0, &[16, 17, 19, 22, 23, 24]), // 21
    Region::new(Mountain, MINE | CAVERN, &[13, 17, 18, 21, 24, 26]), // 22
    Region::new(Swamp, MINE | TRIBE | EDGE, &[19, 21, 24, 27]), // 23
    Region::new(Forest, MAGIC | TRIBE, &[21, 22, 23, 25, 26, 27, 29]), // 24
    Region::new(Mountain, EDGE, &[24, 27, 28, 29]), // 25
    Region::new(Mountain, 0, &[13, 22, 24, 29, 30, 32]), // 26
    Region::new(Farmland, EDGE, &[23, 24, 25]), // 27
    Region::new(Hill, MINE | EDGE, &[25, 29, 31]), // 28
    Region::new(Swamp, CAVERN | TRIBE, &[24, 25, 26, 28, 30, 31]), // 29
    Region::new(Forest, MAGIC | TRIBE, &[26, 29, 31, 32, 37]), // 30
    Region::new(Farmland, EDGE, &[28, 29, 30, 37]), // 31
    Region::new(Farmland, TRIBE, &[13, 14, 26, 30, 33, 36, 37]), // 32
    Region::new(Hill, TRIBE, &[15, 32, 34, 35, 36]), // 33
    Region::new(Swamp, CAVERN | TRIBE | EDGE, &[15, 33, 35]), // 34
    Region::new(Hill, MAGIC | EDGE, &[33, 34, 36, 38, 39]), // 35
    Region::new(Mountain, CAVERN, &[32, 33, 35, 37, 38]), // 36
    Region::new(Mountain, MINE | EDGE, &[30, 31, 32, 36, 38]), // 37
    Region::new(Forest, EDGE, &[35, 36, 37, 39]), // 38
    Region::new(Sea, EDGE, &[35, 38]), // 39
];

/// The 5-player board: 47 regions, 8 turns.
const MAP_5: &[Region] = &[
    Region::new(Sea, EDGE, &[1, 4, 6, 8, 9, 23, 24, 25]), // 0
    Region::new(Mountain, EDGE, &[0, 2, 4]), // 1
    Region::new(Swamp, MAGIC | EDGE, &[1, 3, 4, 5]), // 2
    Region::new(Mountain, CAVERN | EDGE, &[2, 5, 12, 13]), // 3
    Region::new(Swamp, 0, &[0, 1, 2, 5, 6]), // 4
    Region::new(Farmland, MINE | TRIBE, &[2, 3, 4, 6, 7, 13]), // 5
    Region::new(Hill, TRIBE, &[0, 4, 5, 7, 9, 10]), // 6
    Region::new(Farmland, CAVERN | TRIBE, &[5, 6, 10, 11, 13, 15, 36]), // 7
    Region::new(Hill, TRIBE, &[0, 9]), // 8
    Region::new(Forest, MAGIC, &[0, 6, 8, 10, 25]), // 9
    Region::new(Swamp, TRIBE, &[6, 7, 9, 11, 25, 26]), // 10
    Region::new(Hill, MAGIC, &[7, 10, 26, 36]), // 11
    Region::new(Forest, MAGIC | EDGE, &[3, 13, 14, 15]), // 12
    Region::new(Hill, 0, &[3, 5, 7, 12, 15]), // 13
    Region::new(Farmland, CAVERN | TRIBE | EDGE, &[12, 15, 16]), // 14
    Region::new(Mountain, MINE, &[7, 12, 13, 14, 16, 17, 19, 36]), // 15
    Region::new(Farmland, EDGE, &[14, 15, 17, 18]), // 16
    Region::new(Swamp, CAVERN | TRIBE, &[15, 16, 18, 19, 20]), // 17
    Region::new(Hill, MINE | EDGE, &[16, 17, 20]), // 18
    Region::new(Forest, 0, &[15, 17, 20, 21, 36]), // 19
    Region::new(Mountain, MAGIC | EDGE, &[17, 18, 19, 21, 22]), // 20
    Region::new(Swamp, MINE, &[19, 20, 22, 36, 37, 38]), // 21
    Region::new(Farmland, EDGE, &[20, 21, 38, 39]), // 22
    Region::new(Forest, MINE | EDGE, &[0, 24, 28]), // 23
    Region::new(Swamp, 0, &[0, 23, 25, 27, 28, 29]), // 24
    Region::new(Farmland, TRIBE, &[0, 9, 10, 24, 26, 27]), // 25
    Region::new(Mountain, MINE | CAVERN, &[10, 11, 25, 27, 36]), // 26
    Region::new(Hill, TRIBE, &[24, 25, 26, 29, 30, 31, 36]), // 27
    Region::new(Farmland, CAVERN | TRIBE | EDGE, &[23, 24, 29, 32]), // 28
    Region::new(Forest, TRIBE, &[24, 27, 28, 30, 32, 33]), // 29
    Region::new(Farmland, 0, &[27, 29, 31, 33, 34]), // 30
    Region::new(Swamp, CAVERN | TRIBE, &[27, 30, 34, 35, 36, 40, 43]), // 31
    Region::new(Swamp, MINE | TRIBE | EDGE, &[28, 29, 33]), // 32
    Region::new(Forest, MAGIC | TRIBE | EDGE, &[29, 30, 32, 34]), // 33
    Region::new(Hill, MINE | EDGE, &[30, 31, 33, 35]), // 34
    Region::new(Mountain, MAGIC | EDGE, &[31, 34, 41, 43]), // 35
    Region::new(Lake, 0, &[7, 11, 15, 19, 21, 26, 27, 31, 37, 40]), // 36
    Region::new(Forest, TRIBE, &[21, 36, 38, 40, 42]), // 37
    Region::new(Hill, 0, &[21, 22, 37, 39, 42, 45]), // 38
    Region::new(Swamp, CAVERN | TRIBE | EDGE, &[22, 38, 45]), // 39
    Region::new(Farmland, MAGIC, &[31, 36, 37, 41, 42, 43]), // 40
    Region::new(Mountain, EDGE, &[35, 40, 42, 43, 44]), // 41
    Region::new(Mountain, MINE, &[37, 38, 40, 41, 44, 45]), // 42
    Region::new(Forest, 0, &[31, 35, 40, 41]), // 43
    Region::new(Forest, CAVERN | EDGE, &[41, 42, 45, 46]), // 44
    Region::new(Hill, MAGIC | TRIBE | EDGE, &[38, 39, 42, 44, 46]), // 45
    Region::new(Sea, EDGE, &[44, 45]), // 46
];
