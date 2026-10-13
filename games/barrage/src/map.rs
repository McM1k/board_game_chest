//! The map: a stretch of the Alps cut into twelve basins, from the four
//! below the headstreams in the Mountains down to the two at the bottom of
//! the Plains, where the water leaves the map.
//!
//! Each basin is crossed in the same order by a drop of water: first its
//! powerhouses (at the top of the Hills and Plains basins), then its upper
//! dam space (outlined in red on the map: 3 Credits more to build there,
//! but it catches the water first), then its lower dam space, and on along
//! the river to the next basin. A dam holds as many drops as its level.
//!
//! A conduit sits in the basin of the dams it drains and leads to the
//! powerhouses of another basin. Its production value (1 to 5, the colour
//! of its pipe on the map) is the energy each drop sent through it makes.
//!
//! The connections were read off the printed map: rivers M1→P1, M2→H1,
//! M3→H2, M4→H3, H1→P2, H2→P3, H3→P3, P1→B1, P2→B1, P3→B2.

use serde::{Deserialize, Serialize};

/// The three bands of the map, from top to bottom. The two basins at the
/// very bottom belong to the Plains.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Area {
    Mountains,
    Hills,
    Plains,
}

impl Area {
    pub const ALL: [Area; 3] = [Area::Mountains, Area::Hills, Area::Plains];

    pub fn name(self) -> &'static str {
        match self {
            Area::Mountains => "Mountains",
            Area::Hills => "Hills",
            Area::Plains => "Plains",
        }
    }

    /// Excavators for a Base here.
    pub fn base_cost(self) -> u8 {
        match self {
            Area::Mountains => 5,
            Area::Hills => 4,
            Area::Plains => 3,
        }
    }

    /// Concrete Mixers for an Elevation here.
    pub fn elevation_cost(self) -> u8 {
        match self {
            Area::Mountains => 4,
            Area::Hills => 3,
            Area::Plains => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Basin {
    pub name: &'static str,
    pub area: Area,
    /// Where its river goes; `None`: off the map.
    pub flows_to: Option<usize>,
}

pub const M1: usize = 0;
pub const M2: usize = 1;
pub const M3: usize = 2;
pub const M4: usize = 3;
pub const H1: usize = 4;
pub const H2: usize = 5;
pub const H3: usize = 6;
pub const P1: usize = 7;
pub const P2: usize = 8;
pub const P3: usize = 9;
pub const B1: usize = 10;
pub const B2: usize = 11;

const fn basin(name: &'static str, area: Area, flows_to: Option<usize>) -> Basin {
    Basin { name, area, flows_to }
}

pub const BASINS: [Basin; 12] = [
    basin("M1", Area::Mountains, Some(P1)),
    basin("M2", Area::Mountains, Some(H1)),
    basin("M3", Area::Mountains, Some(H2)),
    basin("M4", Area::Mountains, Some(H3)),
    basin("H1", Area::Hills, Some(P2)),
    basin("H2", Area::Hills, Some(P3)),
    basin("H3", Area::Hills, Some(P3)),
    basin("P1", Area::Plains, Some(B1)),
    basin("P2", Area::Plains, Some(B1)),
    basin("P3", Area::Plains, Some(B2)),
    basin("B1", Area::Plains, None),
    basin("B2", Area::Plains, None),
];

/// The basins with dams and conduits: all but the two at the bottom.
pub const DAM_BASINS: usize = 10;

/// Dam spaces: two per basin with dams, the upper (red, paid) one first.
/// Space `s` is in basin `s / 2`.
pub const DAM_SPACES: usize = DAM_BASINS * 2;

pub fn dam_basin(space: usize) -> usize {
    space / 2
}

/// The upper dam space of a basin is outlined in red: 3 Credits more.
pub fn dam_paid(space: usize) -> bool {
    space.is_multiple_of(2)
}

/// A conduit space: in `basin`, leading to the powerhouses of `to`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Conduit {
    pub basin: usize,
    pub to: usize,
    pub value: u8,
}

const fn conduit(basin: usize, to: usize, value: u8) -> Conduit {
    Conduit { basin, to, value }
}

/// Two per basin with dams, the left one first (as drawn on the map).
pub const CONDUITS: [Conduit; 20] = [
    conduit(M1, P1, 5),
    conduit(M1, H1, 4),
    conduit(M2, H2, 3),
    conduit(M2, P3, 5),
    conduit(M3, H1, 4),
    conduit(M3, H2, 3),
    conduit(M4, H3, 3),
    conduit(M4, B2, 5),
    conduit(H1, P1, 3),
    conduit(H1, P3, 4),
    conduit(H2, P2, 4),
    conduit(H2, H3, 2),
    conduit(H3, P3, 2),
    conduit(H3, B2, 3),
    conduit(P1, B1, 3),
    conduit(P1, P2, 2),
    conduit(P2, B1, 1),
    conduit(P2, B2, 3),
    conduit(P3, B1, 2),
    conduit(P3, B2, 1),
];

/// A powerhouse space; the red (paid) ones cost 3 Credits more.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PowerhouseSpace {
    pub basin: usize,
    pub paid: bool,
}

const fn ph(basin: usize, paid: bool) -> PowerhouseSpace {
    PowerhouseSpace { basin, paid }
}

/// Two in each Hills basin, three in each upper Plains basin, four in each
/// of the two at the bottom.
pub const POWERHOUSES: [PowerhouseSpace; 23] = [
    ph(H1, true),
    ph(H1, false),
    ph(H2, true),
    ph(H2, false),
    ph(H3, true),
    ph(H3, false),
    ph(P1, true),
    ph(P1, false),
    ph(P1, false),
    ph(P2, true),
    ph(P2, false),
    ph(P2, false),
    ph(P3, true),
    ph(P3, true),
    ph(P3, false),
    ph(B1, true),
    ph(B1, true),
    ph(B1, false),
    ph(B1, false),
    ph(B2, true),
    ph(B2, true),
    ph(B2, true),
    ph(B2, false),
];

/// The four headstreams feed the four Mountains basins.
pub const HEADSTREAMS: usize = 4;

pub fn headstream_basin(headstream: usize) -> usize {
    headstream
}

/// The basins a drop entering `basin` crosses, `basin` first.
pub fn downstream(basin: usize) -> Vec<usize> {
    let mut path = vec![basin];
    let mut b = basin;
    while let Some(next) = BASINS[b].flows_to {
        path.push(next);
        b = next;
    }
    path
}

/// The conduit spaces of a basin.
pub fn conduits_of(basin: usize) -> impl Iterator<Item = usize> {
    (0..CONDUITS.len()).filter(move |&c| CONDUITS[c].basin == basin)
}

/// The powerhouse spaces of a basin.
pub fn powerhouses_of(basin: usize) -> impl Iterator<Item = usize> {
    (0..POWERHOUSES.len()).filter(move |&p| POWERHOUSES[p].basin == basin)
}

/// The dam spaces of a basin, upper first.
pub fn dams_of(basin: usize) -> impl Iterator<Item = usize> {
    (0..DAM_SPACES).filter(move |&d| dam_basin(d) == basin)
}
