//! The map: the board's regions (the Sky Islands above the ground board when
//! they are played), what stands in each, and the regions the tool in hand
//! can act on, marked with what they cost. A click on a region goes through
//! `look::click`.
//!
//! The regions lie on a picture of the printed board, invisible but for
//! what the tool in hand can act on, with the pieces as the box has them;
//! or, drawn, each is filled with its terrain, and the pieces are our signs.

use super::look::{self, Click, Mark};
use super::shapes::{self, Shape};
use crate::app::{send, App};
use leptos::prelude::*;
use protocol::{ClientMsg, GameMove};
use smallworld_core::{Board, Race, Terrain, View};

/// Space between the Sky Islands and the ground board.
const GAP: i32 = 40;

// What stands in a region, around its anchor, in board units (1000 across):
// a token's disc, with its picture inside a thin ring of the player's
// colour, and the count chip on its lower right (x, y, radius)...
const TOKEN_R: i32 = 28;
const ART_R: i32 = TOKEN_R - 2;
const CHIP: (i32, i32, i32) = (17, 12, 11);
// ...the markers' size (our signs, and the pieces' pictures over the printed
// board, a little larger), the row of printed features above the token and
// the row of pieces under it (the pictures' row lowered by what they gain,
// to keep clear of the count)...
const MARKER_R: i32 = 10;
const PIECE_R: i32 = 13;
const ICONS_DY: i32 = -39;
const PIECES_DY: i32 = 39;
const PICTURES_DY: i32 = PIECES_DY + PIECE_R - MARKER_R;
// ...and the cost badge on its right.
const BADGE_DX: i32 = 46;
const BADGE_R: i32 = 13;

fn terrain_class(terrain: Terrain) -> &'static str {
    match terrain {
        Terrain::Farmland => "t-farm",
        Terrain::Forest => "t-forest",
        Terrain::Hill => "t-hill",
        Terrain::Swamp => "t-swamp",
        Terrain::Mountain => "t-mountain",
        Terrain::Sea => "t-sea",
        Terrain::Lake => "t-lake",
    }
}

/// The textures the terrains are filled with (see `smallworld.css`).
fn defs() -> AnyView {
    view! {
        <defs>
            <pattern id="sw-farm" width="24" height="24" patternUnits="userSpaceOnUse" patternTransform="rotate(25)">
                <rect width="24" height="24" fill="#e7c25c" />
                <rect width="24" height="12" fill="#ddb24a" />
                <line x1="0" y1="6" x2="24" y2="6" stroke="#c9992f" stroke-width="1.2" stroke-dasharray="3 3" />
            </pattern>
            <pattern id="sw-forest" width="22" height="20" patternUnits="userSpaceOnUse">
                <rect width="22" height="20" fill="#2f6a39" />
                <circle cx="6" cy="6" r="5" fill="#25572e" />
                <circle cx="17" cy="15" r="5" fill="#25572e" />
                <circle cx="5" cy="5" r="2" fill="#3f8148" />
                <circle cx="16" cy="14" r="2" fill="#3f8148" />
            </pattern>
            <pattern id="sw-hill" width="34" height="22" patternUnits="userSpaceOnUse">
                <rect width="34" height="22" fill="#9cc65a" />
                <path d="M0 18 Q8 8 17 18 Q25 8 34 18" fill="none" stroke="#86b048" stroke-width="2" />
            </pattern>
            <pattern id="sw-swamp" width="30" height="18" patternUnits="userSpaceOnUse">
                <rect width="30" height="18" fill="#7d6d3c" />
                <ellipse cx="9" cy="6" rx="6" ry="2.2" fill="#5f8f86" />
                <ellipse cx="23" cy="14" rx="5" ry="1.8" fill="#5f8f86" />
                <line x1="17" y1="3" x2="17" y2="9" stroke="#4d5a26" stroke-width="1.4" />
            </pattern>
            <pattern id="sw-mountain" width="30" height="24" patternUnits="userSpaceOnUse">
                <rect width="30" height="24" fill="#9ba2ad" />
                <path d="M2 22 L11 6 L20 22 Z" fill="#848c98" />
                <path d="M8.5 10.5 L11 6 L13.5 10.5 Z" fill="#eef1f5" />
                <path d="M16 22 L23 11 L30 22 Z" fill="#8a929e" />
            </pattern>
            <pattern id="sw-sea" width="36" height="16" patternUnits="userSpaceOnUse">
                <rect width="36" height="16" fill="#3f7cc0" />
                <path d="M0 9 Q4.5 5 9 9 T18 9" fill="none" stroke="#6aa1dc" stroke-width="1.4" />
                <path d="M18 3 Q22.5 -1 27 3 T36 3" fill="none" stroke="#6aa1dc" stroke-width="1.4" />
            </pattern>
            <pattern id="sw-lake" width="30" height="14" patternUnits="userSpaceOnUse">
                <rect width="30" height="14" fill="#5ea2df" />
                <path d="M2 8 Q6 5 10 8" fill="none" stroke="#8cc0ee" stroke-width="1.3" />
                <path d="M17 4 Q21 1 25 4" fill="none" stroke="#8cc0ee" stroke-width="1.3" />
            </pattern>
            <linearGradient id="sw-sky" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0" stop-color="#bfe0f7" />
                <stop offset="1" stop-color="#e6f3fb" />
            </linearGradient>
        </defs>
    }
    .into_any()
}

/// A round marker with a glyph, for the region's features and pieces.
pub fn marker(x: i32, y: i32, class: &'static str, glyph: &'static str) -> AnyView {
    view! {
        <g class=format!("marker {class}") transform=format!("translate({x} {y})")>
            <circle r=MARKER_R />
            <path d=glyph />
        </g>
    }
    .into_any()
}

/// A piece over the printed board: the picture of the token in the box.
pub fn piece(x: i32, y: i32, class: &'static str, art: String) -> AnyView {
    view! {
        <g class=format!("piece {class}") transform=format!("translate({x} {y})")>
            <image href=art x=-PIECE_R y=-PIECE_R width=2 * PIECE_R height=2 * PIECE_R />
        </g>
    }
    .into_any()
}

/// Where the `k`th of `n` markers of radius `r` in a row goes, from the
/// row's middle.
fn row_x(k: i32, n: i32, r: i32) -> i32 {
    (2 * k - (n - 1)) * (r + 1)
}

/// A Lost Tribe: its printed token's picture, on a plain disc the size of a
/// race's token.
pub fn tribe_token(x: i32, y: i32) -> AnyView {
    view! {
        <g class="tribe" transform=format!("translate({x} {y})")>
            <circle class="disc" r=TOKEN_R />
            <image href=look::tribe_art() x=-ART_R y=-ART_R width=2 * ART_R height=2 * ART_R />
        </g>
    }
    .into_any()
}

/// A pickaxe, its head curving over a slanted handle.
pub const MINE: &str = "M-7 -1 Q-2 -8 6 -6 L6.5 -4.5 Q-1 -5.5 -5.5 0.5 Z M-0.5 -5.5 L1.5 -6 L5.5 6.5 L3.5 7 Z";
pub const MAGIC: &str = "M0 -7 L5 -1 L0 7 L-5 -1 Z";
pub const CAVERN: &str = "M-7 5 Q-7 -6 0 -6 Q7 -6 7 5 L3 5 Q3 -2 0 -2 Q-3 -2 -3 5 Z";
pub const LAIR: &str = "M-7 5 L0 -6 L7 5 Z";
pub const FORT: &str = "M-6 6 L-6 -3 L-4 -3 L-4 -6 L-1 -6 L-1 -3 L1 -3 L1 -6 L4 -6 L4 -3 L6 -3 L6 6 Z";
pub const CAMP: &str = "M-7 6 L0 -6 L7 6 Z M-1 6 L0 1 L1 6 Z";
pub const HOLE: &str = "M-7 1 Q0 -5 7 1 Q0 6 -7 1 Z";
pub const HERO: &str = "M0 -7 L2 -2 L7 -2 L3 1 L4.5 6.5 L0 3.5 L-4.5 6.5 L-3 1 L-7 -2 L-2 -2 Z";
pub const DRAGON: &str = "M-7 3 L-2 -1 L-4 -7 L1 -2 L7 -4 L4 2 L7 6 L0 4 Z";
pub const POT: &str = "M-6 -1 Q-6 6 0 6 Q6 6 6 -1 Z M-7 -4 L7 -4 L7 -1.5 L-7 -1.5 Z";
pub const CATAPULT: &str = "M-7 4 L7 4 L7 6 L-7 6 Z M-5 4 L2 -5 L4 -4 L-3 4 Z M1 -8 L5 -8 L5 -4 L1 -4 Z";
pub const WINTER: &str = "M-1 -7 L1 -7 L1 7 L-1 7 Z M-7 -1 L7 -1 L7 1 L-7 1 Z M-5.5 -4 L-4 -5.5 L5.5 4 L4 5.5 Z M4 -5.5 L5.5 -4 L-4 5.5 L-5.5 4 Z";
pub const LOOT: &str = "M-7 -1 L7 -1 L7 6 L-7 6 Z M-7 -2.5 Q0 -9 7 -2.5 Z";
pub const BEHEMOTH: &str = "M-6 5 L-6 -1 L-3 -6 L-1 -1 L1 -1 L3 -6 L6 -1 L6 5 Z";
pub const CANNON: &str = "M-7 4 A3 3 0 1 0 -1 4 A3 3 0 1 0 -7 4 Z M-4 0 L6 -5 L7 -3 L-3 2 Z";
pub const LAVA: &str = "M0 -7 Q6 1 0 7 Q-6 1 0 -7 Z";
pub const FIRE: &str = "M0 -7 Q6 0 3 6 Q0 3 -3 6 Q-6 0 0 -7 Z";
pub const CAPTIVE: &str = "M-5 -4 L5 -4 L5 4 L-5 4 Z";
pub const BEANSTALK: &str = "M-1 7 L1 7 L1 -2 L-1 -2 Z M0 -2 Q-7 -4 -5 -8 Q0 -7 0 -2 Z M0 1 Q7 -1 5 -5 Q0 -4 0 1 Z";
pub const STAIRWAY: &str = "M-7 6 L-7 2 L-3 2 L-3 -2 L1 -2 L1 -6 L7 -6 L7 6 Z";

/// Where each region's shape sits on the drawing: the Sky Islands on top,
/// the ground board under them.
struct Layout {
    ground: &'static shapes::Board,
    sky: Option<&'static shapes::Board>,
}

impl Layout {
    fn new(view: &View) -> Layout {
        Layout { ground: shapes::board(view.board), sky: view.sky.map(shapes::sky) }
    }

    /// How far down the ground board starts.
    fn ground_y(&self) -> i32 {
        self.sky.map_or(0, |s| s.height as i32 + GAP)
    }

    fn height(&self) -> i32 {
        self.ground_y() + self.ground.height as i32
    }

    /// A region's shape and its vertical offset.
    fn shape(&self, region: usize) -> (&'static Shape, i32) {
        let n = self.ground.shapes.len();
        match (region < n, self.sky) {
            (true, _) | (false, None) => (&self.ground.shapes[region.min(n - 1)], self.ground_y()),
            (false, Some(sky)) => (&sky.shapes[region - n], 0),
        }
    }
}

/// The whole map, for one state: on the printed board's picture, or drawn.
pub fn map_svg(
    app: App,
    view: &View,
    names: &[String],
    tool: Option<look::Tool>,
    pending: Option<usize>,
    drawn: bool,
) -> AnyView {
    let layout = Layout::new(view);
    let board: Board = view.map();
    let marks = tool.map(|t| look::marks(view, t)).unwrap_or_default();
    let mark_of = |r: usize| marks.iter().find(|(m, _)| *m == r).map(|(_, mark)| mark.clone());

    let regions = (0..board.len())
        .map(|i| {
            let (shape, dy) = layout.shape(i);
            let terrain = board.regions[i].terrain;
            let state = &view.regions[i];
            let mark = mark_of(i);
            let class = format!(
                "region {}{}{}{}",
                terrain_class(terrain),
                match mark {
                    Some(Mark::Target(_)) => " target",
                    Some(Mark::Gamble(_)) => " gamble",
                    Some(Mark::Price(_)) | None => "",
                },
                if pending == Some(i) { " pending" } else { "" },
                if state.lava.is_some() || state.burning.is_some() { " molten" } else { "" },
            );
            let title = look::region_title(view, &board, i, names);
            let on_click = move |_| {
                let small_world = app.games.smallworld;
                let Some(view) = small_world.view.get_untracked() else { return };
                let Some(tool) = look::effective_tool(&view, small_world.ui.tool.get_untracked()) else { return };
                match look::click(&view, tool, i) {
                    Click::Act(action) => send(app, &ClientMsg::Move(GameMove::SmallWorld(action))),
                    Click::Gamble(region) => small_world.ui.gamble.set(Some(region)),
                    Click::Nothing => {}
                }
            };
            view! {
                <g class=class on:click=on_click transform=format!("translate(0 {dy})")>
                    <title>{title}</title>
                    <polygon points=shape.points />
                </g>
            }
        })
        .collect_view();

    // What stands in each region, drawn above every outline and letting
    // clicks through to the region underneath.
    let pieces = (0..board.len())
        .map(|i| {
            let (shape, dy) = layout.shape(i);
            let (x, y) = (shape.at.0, shape.at.1 + dy);
            let def = &board.regions[i];
            let r = &view.regions[i];
            // What is printed on the board (the picture shows it itself).
            let mut icons: Vec<(&'static str, &'static str)> = Vec::new();
            if drawn {
                if def.mine() {
                    icons.push(("mine", MINE));
                }
                if def.magic() {
                    icons.push(("magic", MAGIC));
                }
                if def.cavern() {
                    icons.push(("cavern", CAVERN));
                }
                for (k, &(g, l)) in board.access.iter().enumerate() {
                    if g == i || l == i {
                        icons.push(if k == 0 { ("beanstalk", BEANSTALK) } else { ("stairway", STAIRWAY) });
                    }
                }
            }
            // The pieces: our sign for each, and its picture.
            let mut pieces: Vec<(&'static str, &'static str, String)> = Vec::new();
            let drakon = r.tokens.is_some_and(|t| t.race == Race::Drakons);
            let flags = [
                (r.lair, "lair", LAIR, "lair"),
                (r.fortress, "fort", FORT, "fort"),
                (r.hole, "hole", HOLE, "hole"),
                (r.hero, "hero", HERO, "hero"),
                (r.dragon, "dragon", DRAGON, if drakon { "drakon" } else { "dragon" }),
                (r.pot, "pot", POT, "pot"),
                (r.catapult, "catapult", CATAPULT, "catapult"),
                (r.winter, "winter", WINTER, "winter"),
                (r.loot, "loot", LOOT, "loot"),
                (r.lava.is_some(), "lava", LAVA, "lava"),
                (r.burning.is_some(), "burning", FIRE, "burning"),
            ];
            for (has, class, glyph, art) in flags {
                if has {
                    pieces.push((class, glyph, look::piece_art(art)));
                }
            }
            if let Some(race) = r.captive {
                pieces.push(("captive", CAPTIVE, look::race_art(race)));
            }
            for _ in 0..r.encampments {
                pieces.push(("camp", CAMP, look::piece_art("camp")));
            }
            for _ in 0..r.behemoths {
                pieces.push(("behemoth", BEHEMOTH, look::piece_art("behemoth")));
            }
            match r.cannon {
                Some(true) => pieces.push(("cannon firing", CANNON, look::piece_art("cannon-firing"))),
                Some(false) => pieces.push(("cannon", CANNON, look::piece_art("cannon"))),
                None => {}
            }
            // Over the printed board, the pieces are their pictures.
            let (piece_r, pieces_dy) = if drawn { (MARKER_R, PIECES_DY) } else { (PIECE_R, PICTURES_DY) };
            // The region's number goes under the row of pieces, if any.
            let number_dy = if pieces.is_empty() { 44 } else { pieces_dy + piece_r + 10 };
            let icons = {
                let n = icons.len() as i32;
                icons
                    .into_iter()
                    .enumerate()
                    .map(|(k, (class, glyph))| marker(x + row_x(k as i32, n, MARKER_R), y + ICONS_DY, class, glyph))
                    .collect_view()
            };
            let pieces = {
                let n = pieces.len() as i32;
                pieces
                    .into_iter()
                    .enumerate()
                    .map(|(k, (class, glyph, art))| {
                        let (px, py) = (x + row_x(k as i32, n, piece_r), y + pieces_dy);
                        if drawn {
                            marker(px, py, class, glyph)
                        } else {
                            piece(px, py, class, art)
                        }
                    })
                    .collect_view()
            };
            let token = r.tokens.map(|t| {
                let seat = look::region_owner(view, i).unwrap_or(0);
                let class = if t.declined { "token declined" } else { "token" };
                // The race's portrait on a disc of its player's colour, and
                // how many tokens stand there.
                let color = look::seat_color(seat);
                let (cx, cy, cr) = CHIP;
                view! {
                    <g class=class transform=format!("translate({x} {y})")>
                        <circle class="disc" r=TOKEN_R fill=color />
                        <image href=look::race_art(t.race) x=-ART_R y=-ART_R width=2 * ART_R height=2 * ART_R />
                        <g class="count" transform=format!("translate({cx} {cy})")>
                            <circle r=cr fill=color />
                            <text y="5">{t.count}</text>
                        </g>
                    </g>
                }
            });
            let tribe = r.lost_tribe.then(|| tribe_token(x, y));
            // Beside what stands in the region, or in its middle.
            let bx = if r.tokens.is_some() || r.lost_tribe { x + BADGE_DX } else { x };
            let badge = mark_of(i).and_then(|mark| {
                let (text, class) = match mark {
                    Mark::Target(Some(text)) => (text, "badge"),
                    Mark::Gamble(text) => (text, "badge die"),
                    Mark::Price(text) => (text, "badge price"),
                    Mark::Target(None) => return None,
                };
                Some(view! {
                    <g class=class transform=format!("translate({bx} {y})")>
                        <circle r=BADGE_R />
                        <text y="5">{text}</text>
                    </g>
                })
            });
            view! {
                <g class="pieces">
                    <text class="number" x=x y=y + number_dy>{i + 1}</text>
                    {icons}
                    {tribe}
                    {token}
                    {pieces}
                    {badge}
                </g>
            }
        })
        .collect_view();

    let sky_band = layout.sky.map(|sky| {
        if drawn {
            view! {
                <g class="sky-band">
                    <rect x="0" y="0" width="1000" height=sky.height rx="14" />
                    <text x="16" y="28">"Sky Islands"</text>
                </g>
            }
            .into_any()
        } else {
            let side = view.sky.expect("the Sky Islands are played");
            view! {
                <image class="board-art" href=look::sky_art(side) x="0" y="0" width="1000" height=sky.height preserveAspectRatio="none" />
            }
            .into_any()
        }
    });
    // The printed ground board, under its regions.
    let ground_art = (!drawn).then(|| {
        view! {
            <image
                class="board-art"
                href=look::board_art(view.board)
                x="0"
                y=layout.ground_y()
                width=layout.ground.width
                height=layout.ground.height
                preserveAspectRatio="none"
            />
        }
    });

    view! {
        <svg
            class=if drawn { "map" } else { "map printed" }
            viewBox=format!("0 0 {} {}", layout.ground.width, layout.height())
            role="img"
            aria-label=match view.sky {
                Some(_) => format!("The {}-player board and the Sky Islands", view.board),
                None => format!("The {}-player board", view.board),
            }
        >
            {defs()}
            {sky_band}
            {ground_art}
            <g class="regions">{regions}</g>
            <g class="overlay">{pieces}</g>
        </svg>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use smallworld_core::Sky;

    /// Whether two circles, (x, y, radius), keep clear of each other.
    fn apart(a: (i32, i32, i32), b: (i32, i32, i32)) -> bool {
        f64::from(a.0 - b.0).hypot(f64::from(a.1 - b.1)) >= f64::from(a.2 + b.2)
    }

    #[test]
    fn nothing_around_a_token_covers_it() {
        let token = (0, 0, TOKEN_R);
        let badge = (BADGE_DX, 0, BADGE_R);
        assert!(apart(token, badge) && apart(CHIP, badge));
        // Rows of up to four markers above and under it, and of the pieces'
        // pictures, a little larger, under it.
        for n in 1..=4 {
            for k in 0..n {
                for (dy, r) in [(ICONS_DY, MARKER_R), (PIECES_DY, MARKER_R), (PICTURES_DY, PIECE_R)] {
                    let m = (row_x(k, n, r), dy, r);
                    assert!(apart(token, m) && apart(CHIP, m) && apart(badge, m), "{k} of {n} at {dy}");
                }
            }
        }
    }

    /// How far a point is from the nearest side of a polygon, if it is inside.
    fn clearance((x, y): (f64, f64), points: &str) -> Option<f64> {
        let pts: Vec<(f64, f64)> = points
            .split_whitespace()
            .map(|p| {
                let (a, b) = p.split_once(',').unwrap();
                (a.parse().unwrap(), b.parse().unwrap())
            })
            .collect();
        let (mut inside, mut near) = (false, f64::MAX);
        for (k, &(x1, y1)) in pts.iter().enumerate() {
            let (x2, y2) = pts[(k + 1) % pts.len()];
            if (y1 > y) != (y2 > y) && x < x1 + (y - y1) * (x2 - x1) / (y2 - y1) {
                inside = !inside;
            }
            let (dx, dy) = (x2 - x1, y2 - y1);
            let t = (((x - x1) * dx + (y - y1) * dy) / (dx * dx + dy * dy).max(1e-9)).clamp(0.0, 1.0);
            near = near.min((x - x1 - t * dx).hypot(y - y1 - t * dy));
        }
        inside.then_some(near)
    }

    #[test]
    fn tokens_are_big_and_stay_inside_their_regions() {
        // The disc and its count chip, from the anchor.
        let (cx, cy, cr) = CHIP;
        let reach = f64::from(TOKEN_R).max(f64::from(cx).hypot(f64::from(cy)) + f64::from(cr));
        let boards = (2..=5).map(shapes::board).chain([Sky::Two, Sky::Three].map(shapes::sky));
        for (b, board) in boards.enumerate() {
            for (i, shape) in board.shapes.iter().enumerate() {
                let at = (f64::from(shape.at.0), f64::from(shape.at.1));
                let room = clearance(at, shape.points).unwrap_or(-1.0);
                assert!(room >= reach, "board {b}, region {i}: {room:.1} < {reach:.1}");
            }
        }
    }
}
