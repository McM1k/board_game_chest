//! The map: the four Headstreams, the twelve basins with their building
//! spaces, the rivers between them and the coloured pipes of the conduits,
//! and the pieces built. The spaces a click can act on now are marked with
//! what they cost; a click goes through `look::map_click`.
//!
//! Each basin is drawn as water crosses it: its powerhouses at the top (in
//! the Hills and Plains), the lake, the upper (red) dam space, the two
//! conduit spaces, the lower dam space, and the river on.

use std::sync::Arc;

use super::look;
use crate::app::{send, App};
use barrage_core::{
    dam_basin, Area, Target, Tile, View, BASINS, CONDUITS, DAM_SPACES, HEADSTREAMS, HEADSTREAM_TILES, POWERHOUSES,
};
use leptos::prelude::*;
use protocol::{ClientMsg, GameMove};

pub const WIDTH: i32 = 1000;
pub const HEIGHT: i32 = 1290;
/// A dam space, a conduit space's radius, a powerhouse space.
const DAM: (i32, i32) = (78, 24);
const CONDUIT_R: i32 = 17;
const PH: i32 = 40;

/// A basin's box: its centre across, its top, its width and height.
pub fn basin_box(basin: usize) -> (i32, i32, i32, i32) {
    match basin {
        0..=3 => (130 + 247 * basin as i32, 95, 200, 220),
        4..=6 => ([255, 500, 745][basin - 4], 395, 210, 275),
        7..=9 => ([175, 480, 790][basin - 7], 745, 220, 275),
        _ => ([330, 720][basin - 10], 1110, 200, 140),
    }
}

/// How far down a basin's lake starts: below its powerhouses.
fn offset(basin: usize) -> i32 {
    match BASINS[basin].area {
        Area::Mountains => 0,
        _ => 55,
    }
}

/// The centre of a dam space.
pub fn dam_pos(space: usize) -> (i32, i32) {
    let b = dam_basin(space);
    let (cx, top, _, _) = basin_box(b);
    let dy = if space.is_multiple_of(2) { 95 } else { 185 };
    (cx, top + offset(b) + dy)
}

/// The centre of a conduit space: left or right of its basin.
pub fn conduit_pos(c: usize) -> (i32, i32) {
    let b = CONDUITS[c].basin;
    let (cx, top, w, _) = basin_box(b);
    let dx = w / 2 - 24;
    let x = if c.is_multiple_of(2) { cx - dx } else { cx + dx };
    (x, top + offset(b) + 140)
}

/// The centre of a powerhouse space.
pub fn powerhouse_pos(p: usize) -> (i32, i32) {
    let b = POWERHOUSES[p].basin;
    let (cx, top, _, _) = basin_box(b);
    let index = (0..p).filter(|&q| POWERHOUSES[q].basin == b).count() as i32;
    let count = (0..POWERHOUSES.len()).filter(|&q| POWERHOUSES[q].basin == b).count() as i32;
    if count == 4 {
        (cx - 26 + 52 * (index % 2), top + 38 + 50 * (index / 2))
    } else {
        let step = 52;
        (cx - step * (count - 1) / 2 + step * index, top + 30)
    }
}

/// Where a conduit's pipe arrives: the powerhouses of its basin.
fn pipe_end(basin: usize) -> (i32, i32) {
    let (cx, top, _, _) = basin_box(basin);
    (cx, top + 4)
}

fn headstream_pos(h: usize) -> (i32, i32) {
    let (cx, _, _, _) = basin_box(h);
    (cx, 44)
}

fn rivers() -> AnyView {
    let mut paths = Vec::new();
    for h in 0..HEADSTREAMS {
        let (x, y) = headstream_pos(h);
        let (_, top, _, _) = basin_box(h);
        paths.push(format!("M{x} {} L{x} {top}", y + 26));
    }
    for (b, basin) in BASINS.iter().enumerate() {
        let (x, top, _, h) = basin_box(b);
        let bottom = top + h;
        let (tx, ty) = match basin.flows_to {
            Some(next) => {
                let (nx, ntop, _, _) = basin_box(next);
                (nx, ntop)
            }
            None => (x, HEIGHT),
        };
        let mid = (bottom + ty) / 2;
        paths.push(format!("M{x} {bottom} C{x} {mid} {tx} {mid} {tx} {ty}"));
    }
    let lines = paths.into_iter().map(|d| view! { <path class="river" d=d /> }).collect_view();
    view! { <g class="rivers">{lines}</g> }.into_any()
}

fn pipes(view: &View) -> AnyView {
    let lines = CONDUITS
        .iter()
        .enumerate()
        .map(|(c, conduit)| {
            let (x, y) = conduit_pos(c);
            let (tx, ty) = pipe_end(conduit.to);
            let mid = (y + ty) / 2;
            let d = format!("M{x} {y} C{x} {mid} {tx} {} {tx} {ty}", ty - 40);
            let built = view.conduits[c].is_some();
            view! {
                <path
                    class=if built { "pipe built" } else { "pipe" }
                    d=d
                    stroke=look::value_color(conduit.value)
                />
            }
        })
        .collect_view();
    view! { <g class="pipes">{lines}</g> }.into_any()
}

fn area_class(area: Area) -> &'static str {
    match area {
        Area::Mountains => "basin mountains",
        Area::Hills => "basin hills",
        Area::Plains => "basin plains",
    }
}

fn basins() -> AnyView {
    let boxes = (0..BASINS.len())
        .map(|b| {
            let (cx, top, w, h) = basin_box(b);
            let lake = (b < 10).then(|| {
                let y = top + offset(b) + 48;
                view! {
                    <ellipse class="lake" cx=cx cy=y rx=w / 2 - 22 ry=30 />
                    <ellipse class="lake" cx=cx cy=y + 92 rx=w / 2 - 42 ry=24 />
                }
            });
            view! {
                <g class=area_class(BASINS[b].area)>
                    <rect class="basin-box" x=cx - w / 2 y=top width=w height=h rx=18 />
                    {lake}
                    <text class="basin-name" x=cx - w / 2 + 10 y=top + 18>{look::basin_name(b)}</text>
                </g>
            }
        })
        .collect_view();
    view! { <g class="basins">{boxes}</g> }.into_any()
}

/// Drops held by a dam, drawn above it.
fn drops(x: i32, y: i32, n: u8) -> AnyView {
    let dots = (0..n as i32)
        .map(|i| {
            let dx = x - 11 * (n as i32 - 1) + 22 * i;
            view! { <path class="drop" d=format!("M{dx} {} q-9 12 0 17 q9 -5 0 -17 Z", y - 13) /> }
        })
        .collect_view();
    view! { <g class="drops">{dots}</g> }.into_any()
}

/// The building spaces, with what is built on them.
fn spaces(view: &View, names: &[String]) -> AnyView {
    let dams = (0..DAM_SPACES)
        .map(|d| {
            let (x, y) = dam_pos(d);
            let paid = d % 2 == 0;
            let piece = view.dams[d].map(|dam| {
                let (fill, ink) = match dam.owner {
                    Some(s) => look::seat_colors(view, s),
                    None => (look::NEUTRAL, "#2b2116"),
                };
                let h = 10 + 8 * dam.level as i32;
                let bands = (1..dam.level as i32)
                    .map(|i| view! { <line class="band" x1=x - DAM.0 / 2 + 4 x2=x + DAM.0 / 2 - 4 y1=y + DAM.1 / 2 - 8 * i y2=y + DAM.1 / 2 - 8 * i stroke=ink /> })
                    .collect_view();
                let owner = match dam.owner {
                    Some(s) => format!("{}'s level {} Dam", look::name(s, names), dam.level),
                    None => format!("Level {} neutral Dam", dam.level),
                };
                view! {
                    <g class="piece dam">
                        <title>{format!("{owner}, holding {} drop{}", dam.water, if dam.water == 1 { "" } else { "s" })}</title>
                        <rect x=x - DAM.0 / 2 y=y + DAM.1 / 2 - h width=DAM.0 height=h rx=4 fill=fill />
                        {bands}
                        <text class="level" x=x y=y + DAM.1 / 2 - 4 fill=ink>{dam.level}</text>
                        {drops(x, y + DAM.1 / 2 - h - 4, dam.water)}
                    </g>
                }
            });
            view! {
                <g class=if paid { "space dam-space paid" } else { "space dam-space" }>
                    <rect x=x - DAM.0 / 2 y=y - DAM.1 / 2 width=DAM.0 height=DAM.1 rx=4 />
                    {paid.then(|| view! { <text class="fee" x=x + DAM.0 / 2 - 10 y=y + 5>"3"</text> })}
                </g>
                {piece}
            }
        })
        .collect_view();
    let conduits = CONDUITS
        .iter()
        .enumerate()
        .map(|(c, conduit)| {
            let (x, y) = conduit_pos(c);
            let (fill, ink) = match view.conduits[c] {
                Some(s) => look::seat_colors(view, s),
                None => ("#f6f1e4", "#2b2116"),
            };
            let built = view.conduits[c].is_some();
            view! {
                <g class=if built { "space conduit-space built" } else { "space conduit-space" }>
                    <title>{format!("Conduit {} from {} to {}", conduit.value, look::basin_name(conduit.basin), look::basin_name(conduit.to))}</title>
                    <circle cx=x cy=y r=CONDUIT_R fill=fill stroke=look::value_color(conduit.value) />
                    <text x=x y=y + 6 fill=ink>{conduit.value}</text>
                </g>
            }
        })
        .collect_view();
    let powerhouses = POWERHOUSES
        .iter()
        .enumerate()
        .map(|(p, space)| {
            let (x, y) = powerhouse_pos(p);
            let piece = view.powerhouses[p].map(|s| {
                let (fill, ink) = look::seat_colors(view, s);
                view! {
                    <g class="piece powerhouse">
                        <title>{format!("{}'s Powerhouse", look::name(s, names))}</title>
                        <rect x=x - PH / 2 + 3 y=y - PH / 2 + 3 width=PH - 6 height=PH - 6 rx=5 fill=fill />
                        <path class="bolt" d=format!("M{} {} l-8 12 h7 l-4 11 l11 -15 h-7 l4 -8 Z", x + 2, y - 13) fill=ink />
                    </g>
                }
            });
            view! {
                <g class=if space.paid { "space ph-space paid" } else { "space ph-space" }>
                    <rect x=x - PH / 2 y=y - PH / 2 width=PH height=PH rx=6 />
                    {space.paid.then(|| view! { <text class="fee" x=x + PH / 2 - 7 y=y - PH / 2 + 11>"3"</text> })}
                </g>
                {piece}
            }
        })
        .collect_view();
    view! { <g class="spaces">{dams}{conduits}{powerhouses}</g> }.into_any()
}

/// Where a target sits on the map, and how big its click area is.
fn target_area(target: Target) -> (i32, i32, i32, i32) {
    match target {
        Target::Base(d) | Target::Elevation(d) => {
            let (x, y) = dam_pos(d);
            (x - DAM.0 / 2 - 4, y - DAM.1 / 2 - 4, DAM.0 + 8, DAM.1 + 8)
        }
        Target::Conduit(c) => {
            let (x, y) = conduit_pos(c);
            (x - CONDUIT_R - 4, y - CONDUIT_R - 4, 2 * CONDUIT_R + 8, 2 * CONDUIT_R + 8)
        }
        Target::Powerhouse(p) => {
            let (x, y) = powerhouse_pos(p);
            (x - PH / 2 - 4, y - PH / 2 - 4, PH + 8, PH + 8)
        }
    }
}

fn target_marks(
    app: App,
    view: &Arc<View>,
    marks: Vec<(Target, String)>,
    choice: Option<(Option<Tile>, Option<Tile>)>,
    mixers: bool,
) -> AnyView {
    let list = marks
        .into_iter()
        .map(|(target, cost)| {
            let (x, y, w, h) = target_area(target);
            let v = Arc::clone(view);
            let click = move |_| {
                if let Some(action) = look::map_click(&v, choice, mixers, target) {
                    send(app, &ClientMsg::Move(GameMove::Barrage(action)));
                }
            };
            let label = format!("{} \u{2014} {cost}", look::target_text(target));
            let title = label.clone();
            let width = cost.chars().count() as i32 * 9;
            view! {
                <g class="target" role="button" tabindex="0" aria-label=label on:click=click>
                    <title>{title}</title>
                    <rect x=x y=y width=w height=h rx=8 />
                    <g class="cost" transform=format!("translate({} {})", x + w, y)>
                        <rect x=-6 y=-14 width=width + 12 height=22 rx=11 />
                        <text x=width / 2 y=2>{cost}</text>
                    </g>
                </g>
            }
        })
        .collect_view();
    view! { <g class="targets">{list}</g> }.into_any()
}

fn headstreams(app: App, view: &View, water: Option<(u8, bool)>, placed: [u8; HEADSTREAMS]) -> AnyView {
    let ui = app.games.barrage.ui;
    let total: u8 = placed.iter().sum();
    let list = (0..HEADSTREAMS)
        .map(|h| {
            let (x, y) = headstream_pos(h);
            let tile = HEADSTREAM_TILES[view.headstream_tiles[h]];
            let rounds = tile
                .iter()
                .enumerate()
                .map(|(r, n)| {
                    let now = r + 1 == view.round as usize;
                    view! { <tspan class=if now { "now" } else { "" }>{format!("{n} ")}</tspan> }
                })
                .collect_view();
            let open = water.is_some_and(|(n, _)| total < n);
            let click = move |_| {
                if open {
                    ui.drops.update(|d| d[h] += 1);
                }
            };
            let waiting = view.headstreams[h] + placed[h];
            let label = ["A", "B", "C", "D"][h];
            view! {
                <g class=if open { "headstream open" } else { "headstream" } on:click=click role=if open { "button" } else { "img" }>
                    <title>{format!("Headstream {label}: {} drop{} waiting; drops per round {:?}", view.headstreams[h], if view.headstreams[h] == 1 { "" } else { "s" }, tile)}</title>
                    <rect x=x - 80 y=y - 34 width=160 height=60 rx=12 />
                    <text class="hs-name" x=x - 68 y=y - 14>{label}</text>
                    <text class="hs-rounds" x=x + 66 y=y - 14>{rounds}</text>
                    {drops(x, y + 12, waiting.min(8))}
                    {(waiting > 8).then(|| view! { <text class="hs-count" x=x + 60 y=y + 16>{format!("\u{d7}{waiting}")}</text> })}
                    {(placed[h] > 0).then(|| view! { <text class="hs-added" x=x - 66 y=y + 18>{format!("+{}", placed[h])}</text> })}
                </g>
            }
        })
        .collect_view();
    view! { <g class="headstreams">{list}</g> }.into_any()
}

/// The whole map for the view, the targets of a click marked.
#[allow(clippy::too_many_arguments)]
pub fn map_svg(
    app: App,
    view: Arc<View>,
    names: &[String],
    marks: Vec<(Target, String)>,
    choice: Option<(Option<Tile>, Option<Tile>)>,
    mixers: bool,
    water: Option<(u8, bool)>,
    placed: [u8; HEADSTREAMS],
) -> AnyView {
    view! {
        <svg class="map" viewBox=format!("0 0 {WIDTH} {HEIGHT}") role="img" aria-label="The map of the Alps">
            {rivers()}
            {pipes(&view)}
            {basins()}
            {spaces(&view, names)}
            {headstreams(app, &view, water, placed)}
            {target_marks(app, &view, marks, choice, mixers)}
        </svg>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No two building spaces overlap, and every one is inside its basin.
    #[test]
    fn the_spaces_sit_apart_inside_their_basins() {
        let mut boxes: Vec<(i32, i32, i32, i32)> = Vec::new();
        for d in 0..DAM_SPACES {
            boxes.push(target_area(Target::Base(d)));
        }
        for c in 0..CONDUITS.len() {
            boxes.push(target_area(Target::Conduit(c)));
        }
        for p in 0..POWERHOUSES.len() {
            boxes.push(target_area(Target::Powerhouse(p)));
        }
        for (i, a) in boxes.iter().enumerate() {
            for b in &boxes[i + 1..] {
                let apart = a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1;
                assert!(apart, "{a:?} overlaps {b:?}");
            }
        }
        let inside = |basin: usize, (x, y): (i32, i32)| {
            let (cx, top, w, h) = basin_box(basin);
            (cx - w / 2..=cx + w / 2).contains(&x) && (top..=top + h).contains(&y)
        };
        for d in 0..DAM_SPACES {
            assert!(inside(dam_basin(d), dam_pos(d)), "dam {d}");
        }
        for (c, conduit) in CONDUITS.iter().enumerate() {
            assert!(inside(conduit.basin, conduit_pos(c)), "conduit {c}");
        }
        for (p, space) in POWERHOUSES.iter().enumerate() {
            assert!(inside(space.basin, powerhouse_pos(p)), "powerhouse {p}");
        }
        // The basins don't overlap either, and stay on the map.
        for a in 0..BASINS.len() {
            let (ax, at, aw, ah) = basin_box(a);
            assert!(ax - aw / 2 >= 0 && ax + aw / 2 <= WIDTH && at + ah <= HEIGHT);
            for b in a + 1..BASINS.len() {
                let (bx, bt, bw, bh) = basin_box(b);
                let apart = ax + aw / 2 <= bx - bw / 2 || bx + bw / 2 <= ax - aw / 2 || at + ah <= bt || bt + bh <= at;
                assert!(apart, "{} and {}", look::basin_name(a), look::basin_name(b));
            }
        }
    }
}
