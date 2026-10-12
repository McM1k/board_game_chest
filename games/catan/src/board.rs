use crate::Resource;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Terrain {
    Forest,
    Hills,
    Pasture,
    Fields,
    Mountains,
    Desert,
}

impl Terrain {
    pub fn resource(self) -> Option<Resource> {
        match self {
            Terrain::Forest => Some(Resource::Wood),
            Terrain::Hills => Some(Resource::Brick),
            Terrain::Pasture => Some(Resource::Sheep),
            Terrain::Fields => Some(Resource::Wheat),
            Terrain::Mountains => Some(Resource::Ore),
            Terrain::Desert => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tile {
    pub q: i32,
    pub r: i32,
    /// Center in board units (hex size 1.0, pointy-top).
    pub center: (f64, f64),
    pub terrain: Terrain,
    pub number: Option<u8>,
    pub vertices: [usize; 6],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vertex {
    pub pos: (f64, f64),
    pub tiles: Vec<usize>,
    pub neighbors: Vec<usize>,
    pub edges: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub a: usize,
    pub b: usize,
    pub coastal: bool,
}

/// `None` kind is a generic 3:1 port, `Some(r)` is a 2:1 port for `r`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Port {
    pub edge: usize,
    pub kind: Option<Resource>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Board {
    pub tiles: Vec<Tile>,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub ports: Vec<Port>,
}

/// Which island a game is played on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layout {
    /// Tiles, numbers and harbors shuffled for every game.
    #[default]
    Random,
    /// The rulebook's "Starting Set-up for Beginners": the same island
    /// every game.
    Beginner,
}

const HEX_RADIUS: i32 = 2;

/// The rulebook's "Starting Set-up for Beginners" (Illustration A of the
/// base game's rules): each tile's terrain and number, in rows of 3, 4, 5,
/// 4 and 3 from the top, left to right (the order of [`Board::tiles`]).
const BEGINNER_TILES: [(Terrain, Option<u8>); 19] = [
    (Terrain::Mountains, Some(10)),
    (Terrain::Pasture, Some(2)),
    (Terrain::Forest, Some(9)),
    (Terrain::Fields, Some(12)),
    (Terrain::Hills, Some(6)),
    (Terrain::Pasture, Some(4)),
    (Terrain::Hills, Some(10)),
    (Terrain::Fields, Some(9)),
    (Terrain::Forest, Some(11)),
    (Terrain::Desert, None),
    (Terrain::Forest, Some(3)),
    (Terrain::Mountains, Some(8)),
    (Terrain::Forest, Some(8)),
    (Terrain::Mountains, Some(3)),
    (Terrain::Fields, Some(4)),
    (Terrain::Pasture, Some(5)),
    (Terrain::Hills, Some(5)),
    (Terrain::Fields, Some(6)),
    (Terrain::Pasture, Some(11)),
];

/// The same island's 9 harbors, clockwise from the top left: the tile each
/// one touches, which side of it (0 east, then clockwise: 1 south-east,
/// 2 south-west, 3 west, 4 north-west, 5 north-east) and what it trades.
const BEGINNER_PORTS: [(usize, usize, Option<Resource>); 9] = [
    (0, 4, None),
    (1, 5, Some(Resource::Wheat)),
    (6, 5, Some(Resource::Ore)),
    (11, 0, None),
    (15, 1, Some(Resource::Sheep)),
    (17, 1, None),
    (16, 2, None),
    (12, 3, Some(Resource::Brick)),
    (3, 3, Some(Resource::Wood)),
];

impl Board {
    /// The island `layout` names: a random one, or the rulebook's.
    pub fn new(layout: Layout, rng: &mut impl Rng) -> Board {
        match layout {
            Layout::Random => Board::generate(rng),
            Layout::Beginner => Board::beginner(),
        }
    }

    /// The rulebook's island for a first game (see [`BEGINNER_TILES`]).
    pub fn beginner() -> Board {
        let terrains: Vec<Terrain> = BEGINNER_TILES.iter().map(|&(t, _)| t).collect();
        let mut board = Board::island(&terrains);
        for (tile, &(_, number)) in board.tiles.iter_mut().zip(&BEGINNER_TILES) {
            tile.number = number;
        }
        board.ports = BEGINNER_PORTS
            .iter()
            .map(|&(tile, side, kind)| {
                let v = board.tiles[tile].vertices;
                let edge = board.vertices[v[side]]
                    .edges
                    .iter()
                    .copied()
                    .find(|&e| {
                        let e = &board.edges[e];
                        e.a == v[(side + 1) % 6] || e.b == v[(side + 1) % 6]
                    })
                    .expect("a tile's side is an edge");
                Port { edge, kind }
            })
            .collect();
        board
    }

    /// Generates a random standard-sized board (19 land hexes, 9 ports).
    pub fn generate(rng: &mut impl Rng) -> Board {
        let mut terrains = Vec::new();
        for (t, n) in [
            (Terrain::Forest, 4),
            (Terrain::Pasture, 4),
            (Terrain::Fields, 4),
            (Terrain::Hills, 3),
            (Terrain::Mountains, 3),
            (Terrain::Desert, 1),
        ] {
            for _ in 0..n {
                terrains.push(t);
            }
        }
        terrains.shuffle(rng);
        let mut board = Board::island(&terrains);
        assign_numbers(&mut board.tiles, rng);
        let Board { edges, vertices, .. } = &board;

        // Ports: spread 9 around the 30 coastal edges, ordered by angle.
        let mut coastal: Vec<usize> = (0..edges.len()).filter(|&i| edges[i].coastal).collect();
        coastal.sort_by(|&x, &y| {
            let ang = |i: usize| {
                let e = &edges[i];
                let (pa, pb) = (vertices[e.a].pos, vertices[e.b].pos);
                ((pa.1 + pb.1) / 2.0).atan2((pa.0 + pb.0) / 2.0)
            };
            ang(x).partial_cmp(&ang(y)).unwrap()
        });
        let mut kinds: Vec<Option<Resource>> = vec![None, None, None, None];
        kinds.extend(Resource::ALL.iter().map(|&r| Some(r)));
        kinds.shuffle(rng);
        let n = coastal.len();
        board.ports = kinds
            .into_iter()
            .enumerate()
            .map(|(i, kind)| Port {
                edge: coastal[(i * n / 9) % n],
                kind,
            })
            .collect();
        board
    }

    /// The island's shape with these terrains, row by row from the top:
    /// its tiles (without numbers), corners and sides, and no harbors yet.
    fn island(terrains: &[Terrain]) -> Board {
        let mut coords = Vec::new();
        for r in -HEX_RADIUS..=HEX_RADIUS {
            for q in -HEX_RADIUS..=HEX_RADIUS {
                if (q + r).abs() <= HEX_RADIUS {
                    coords.push((q, r));
                }
            }
        }

        let sqrt3 = 3f64.sqrt();
        let mut tiles: Vec<Tile> = coords
            .iter()
            .zip(terrains)
            .map(|(&(q, r), &terrain)| Tile {
                q,
                r,
                center: (sqrt3 * (q as f64 + r as f64 / 2.0), 1.5 * r as f64),
                terrain,
                number: None,
                vertices: [0; 6],
            })
            .collect();

        // Vertices and edges, deduplicated by rounded position.
        let mut vkeys: HashMap<(i64, i64), usize> = HashMap::new();
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut ekeys: HashMap<(usize, usize), usize> = HashMap::new();
        let mut edges: Vec<Edge> = Vec::new();
        let mut edge_tile_count: Vec<u8> = Vec::new();

        for (ti, tile) in tiles.iter_mut().enumerate() {
            let mut ids = [0usize; 6];
            for (i, id) in ids.iter_mut().enumerate() {
                let ang = (60.0 * i as f64 - 30.0).to_radians();
                let x = tile.center.0 + ang.cos();
                let y = tile.center.1 + ang.sin();
                let key = ((x * 100.0).round() as i64, (y * 100.0).round() as i64);
                let vi = *vkeys.entry(key).or_insert_with(|| {
                    vertices.push(Vertex {
                        pos: (x, y),
                        tiles: Vec::new(),
                        neighbors: Vec::new(),
                        edges: Vec::new(),
                    });
                    vertices.len() - 1
                });
                if !vertices[vi].tiles.contains(&ti) {
                    vertices[vi].tiles.push(ti);
                }
                *id = vi;
            }
            tile.vertices = ids;
            for i in 0..6 {
                let (a, b) = (ids[i], ids[(i + 1) % 6]);
                let key = (a.min(b), a.max(b));
                let ei = *ekeys.entry(key).or_insert_with(|| {
                    edges.push(Edge {
                        a: key.0,
                        b: key.1,
                        coastal: false,
                    });
                    edge_tile_count.push(0);
                    edges.len() - 1
                });
                edge_tile_count[ei] += 1;
            }
        }

        for (ei, e) in edges.iter_mut().enumerate() {
            e.coastal = edge_tile_count[ei] == 1;
        }
        for (ei, e) in edges.iter().enumerate() {
            vertices[e.a].neighbors.push(e.b);
            vertices[e.b].neighbors.push(e.a);
            vertices[e.a].edges.push(ei);
            vertices[e.b].edges.push(ei);
        }

        Board {
            tiles,
            vertices,
            edges,
            ports: Vec::new(),
        }
    }

    pub fn desert(&self) -> usize {
        self.tiles
            .iter()
            .position(|t| t.terrain == Terrain::Desert)
            .unwrap()
    }

    /// Port available at each vertex (None = no port; Some(None) = 3:1; Some(Some(r)) = 2:1).
    pub fn vertex_ports(&self) -> Vec<Option<Option<Resource>>> {
        let mut out = vec![None; self.vertices.len()];
        for p in &self.ports {
            let e = &self.edges[p.edge];
            out[e.a] = Some(p.kind);
            out[e.b] = Some(p.kind);
        }
        out
    }

    pub fn tile_neighbors(&self, ti: usize) -> Vec<usize> {
        let t = &self.tiles[ti];
        self.tiles
            .iter()
            .enumerate()
            .filter(|(i, o)| {
                *i != ti && {
                    let (dq, dr) = (o.q - t.q, o.r - t.r);
                    matches!((dq, dr), (1, 0) | (-1, 0) | (0, 1) | (0, -1) | (1, -1) | (-1, 1))
                }
            })
            .map(|(i, _)| i)
            .collect()
    }
}

fn assign_numbers(tiles: &mut [Tile], rng: &mut impl Rng) {
    let mut pool: Vec<u8> = vec![2, 3, 3, 4, 4, 5, 5, 6, 6, 8, 8, 9, 9, 10, 10, 11, 11, 12];
    let land: Vec<usize> = (0..tiles.len())
        .filter(|&i| tiles[i].terrain != Terrain::Desert)
        .collect();

    // Retry until no two "hot" numbers (6/8) touch.
    for _ in 0..2000 {
        pool.shuffle(rng);
        for (k, &ti) in land.iter().enumerate() {
            tiles[ti].number = Some(pool[k]);
        }
        let hot = |t: &Tile| matches!(t.number, Some(6) | Some(8));
        let ok = land.iter().all(|&i| {
            !hot(&tiles[i])
                || land.iter().all(|&j| {
                    i == j || !hot(&tiles[j]) || {
                        let (dq, dr) = (tiles[j].q - tiles[i].q, tiles[j].r - tiles[i].r);
                        !matches!((dq, dr), (1, 0) | (-1, 0) | (0, 1) | (0, -1) | (1, -1) | (-1, 1))
                    }
                })
        });
        if ok {
            return;
        }
    }
}
