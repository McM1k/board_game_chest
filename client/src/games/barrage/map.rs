//! The map: the four Headstreams, the twelve basins with their building
//! spaces, the rivers between them and the coloured pipes of the conduits,
//! and the pieces built. The spaces a click can act on now are marked with
//! what they cost; a click goes through `look::map_click`.
//!
//! By default the map is the printed board's picture (`look::BOARD_ART`),
//! each space where the board prints it and the pieces as the rulebook draws
//! them. The drawn map, a preference, draws each basin as water crosses it:
//! its powerhouses at the top (in the Hills and Plains), the lake, the upper
//! (red) dam space, the two conduit spaces, the lower dam space, and the
//! river on.

use std::sync::Arc;

use super::look;
use crate::app::{send, App};
use barrage_core::{
    dam_basin, Area, Company, Kind, Target, Tile, View, BASINS, CONDUITS, DAM_SPACES, HEADSTREAMS, HEADSTREAM_TILES,
    POWERHOUSES,
};
use leptos::prelude::*;
use protocol::{ClientMsg, GameMove};

pub const WIDTH: i32 = 1000;
pub const HEIGHT: i32 = 1290;
/// A dam space, a conduit space's radius, a powerhouse space.
const DAM: (i32, i32) = (78, 24);
const CONDUIT_R: i32 = 17;
const PH: i32 = 40;

/// The printed board's picture is 659 x 934 pixels, shown 1000 wide.
const PICTURE: (f32, f32) = (659.0, 934.0);
pub const PRINTED_HEIGHT: i32 = (WIDTH as f32 * PICTURE.1 / PICTURE.0) as i32;
/// The printed spaces' sizes: a dam space, a conduit space's radius, a
/// powerhouse space (in map units).
const P_DAM: (i32, i32) = (62, 50);
const P_CONDUIT_R: i32 = 19;
const P_PH: (i32, i32) = (60, 32);

/// A point of the board's picture (in its pixels) on the map.
fn on_board(x: f32, y: f32) -> (i32, i32) {
    let k = WIDTH as f32 / PICTURE.0;
    ((x * k).round() as i32, (y * k).round() as i32)
}

/// Where the board prints them, in the picture's pixels (read off it):
/// each dam basin's centre across, the upper and lower dam spaces' height
/// and the conduit spaces' height in each area.
const DAM_X: [f32; 10] = [135.0, 269.5, 406.0, 537.5, 226.0, 364.0, 507.5, 158.75, 300.0, 443.0];
fn printed_rows(area: Area) -> (f32, f32, f32) {
    match area {
        Area::Mountains => (214.0, 288.5, 250.0),
        Area::Hills => (464.0, 537.5, 502.5),
        Area::Plains => (693.75, 768.75, 731.0),
    }
}
/// The conduit spaces across, left and right of each dam basin.
const CONDUIT_X: [f32; 20] = [
    90.5, 175.5, 225.0, 310.0, 361.0, 447.5, 494.0, 579.0, 185.0, 269.0, 321.0, 406.0, 465.0, 549.5, 116.25, 201.25,
    257.5, 342.5, 400.0, 485.0,
];
/// The powerhouse spaces, in the order of `POWERHOUSES` (the red ones first).
const PH_AT: [(f32, f32); 23] = [
    (226.0, 366.5),
    (226.0, 392.5),
    (364.0, 366.5),
    (364.0, 392.5),
    (507.5, 366.5),
    (507.5, 392.5),
    (158.75, 597.5),
    (135.0, 622.5),
    (182.5, 622.5),
    (300.0, 597.5),
    (276.25, 622.5),
    (323.75, 622.5),
    (418.75, 597.5),
    (465.0, 597.5),
    (442.5, 622.5),
    (200.0, 840.0),
    (246.25, 840.0),
    (200.0, 865.0),
    (246.25, 865.0),
    (450.5, 840.0),
    (495.5, 840.0),
    (495.5, 865.0),
    (450.5, 865.0),
];
/// The Headstream tiles' places, above the Mountains basins.
const HEADSTREAM_Y: f32 = 85.0;

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
pub fn dam_pos(space: usize, printed: bool) -> (i32, i32) {
    let b = dam_basin(space);
    if printed {
        let (upper, lower, _) = printed_rows(BASINS[b].area);
        return on_board(DAM_X[b], if space.is_multiple_of(2) { upper } else { lower });
    }
    let (cx, top, _, _) = basin_box(b);
    let dy = if space.is_multiple_of(2) { 95 } else { 185 };
    (cx, top + offset(b) + dy)
}

/// The centre of a conduit space: left or right of its basin.
pub fn conduit_pos(c: usize, printed: bool) -> (i32, i32) {
    let b = CONDUITS[c].basin;
    if printed {
        return on_board(CONDUIT_X[c], printed_rows(BASINS[b].area).2);
    }
    let (cx, top, w, _) = basin_box(b);
    let dx = w / 2 - 24;
    let x = if c.is_multiple_of(2) { cx - dx } else { cx + dx };
    (x, top + offset(b) + 140)
}

/// The centre of a powerhouse space.
pub fn powerhouse_pos(p: usize, printed: bool) -> (i32, i32) {
    if printed {
        return on_board(PH_AT[p].0, PH_AT[p].1);
    }
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

fn headstream_pos(h: usize, printed: bool) -> (i32, i32) {
    if printed {
        return on_board(DAM_X[h], HEADSTREAM_Y);
    }
    let (cx, _, _, _) = basin_box(h);
    (cx, 44)
}

fn rivers() -> AnyView {
    let mut paths = Vec::new();
    for h in 0..HEADSTREAMS {
        let (x, y) = headstream_pos(h, false);
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
            let (x, y) = conduit_pos(c, false);
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

/// Drops held by a dam or waiting on a Headstream, drawn in a row centred
/// on `x` with their tips at `y`: the rulebook's drop on the printed board.
fn drops(x: i32, y: i32, n: u8, printed: bool) -> AnyView {
    let dots = (0..n as i32)
        .map(|i| {
            if printed {
                let dx = x - 8 * (n as i32 - 1) + 16 * i;
                view! { <image class="drop-art" href=look::token_art("drop") x=dx - 8 y=y width="16" height="20" /> }.into_any()
            } else {
                let dx = x - 11 * (n as i32 - 1) + 22 * i;
                view! { <path class="drop" d=format!("M{dx} {y} q-9 12 0 17 q9 -5 0 -17 Z") /> }.into_any()
            }
        })
        .collect_view();
    view! { <g class="drops">{dots}</g> }.into_any()
}

/// A piece's picture standing on `(x, bottom)`, fitted in `w` x `h`.
fn piece_image(href: String, x: i32, bottom: i32, w: i32, h: i32) -> AnyView {
    view! { <image href=href x=x - w / 2 y=bottom - h width=w height=h preserveAspectRatio="xMidYMax meet" /> }.into_any()
}

/// The company a seat plays (`None` for a neutral piece, or before the
/// companies are picked).
fn company_of(view: &View, seat: Option<usize>) -> Option<Company> {
    view.players.get(seat?)?.company
}

/// The building spaces, with what is built on them.
fn spaces(view: &View, names: &[String], printed: bool) -> AnyView {
    let dam = if printed { P_DAM } else { DAM };
    let dams = (0..DAM_SPACES)
        .map(|d| {
            let (x, y) = dam_pos(d, printed);
            let paid = d % 2 == 0;
            let piece = view.dams[d].map(|built| {
                let owner = match built.owner {
                    Some(s) => format!("{}'s level {} Dam", look::name(s, names), built.level),
                    None => format!("Level {} neutral Dam", built.level),
                };
                let title = format!("{owner}, holding {} drop{}", built.water, if built.water == 1 { "" } else { "s" });
                if printed {
                    // The Base on the space, each Elevation on top of it.
                    let company = company_of(view, built.owner);
                    let bottom = y + dam.1 / 2 - 4;
                    let rise = 18;
                    let stack = (0..built.level as i32)
                        .map(|i| {
                            let kind = if i == 0 { Kind::Base } else { Kind::Elevation };
                            piece_image(look::piece_art(company, kind), x, bottom - rise * i, dam.0 - 4, 40)
                        })
                        .collect_view();
                    let top = bottom - 40 - rise * (built.level as i32 - 1);
                    return view! {
                        <g class="piece dam">
                            <title>{title}</title>
                            {stack}
                            <text class="level-tag" x=x + dam.0 / 2 - 2 y=bottom - 4>{built.level}</text>
                            {drops(x, top - 14, built.water, true)}
                        </g>
                    }
                    .into_any();
                }
                let (fill, ink) = match built.owner {
                    Some(s) => look::seat_colors(view, s),
                    None => (look::NEUTRAL, "#2b2116"),
                };
                let h = 10 + 8 * built.level as i32;
                let bands = (1..built.level as i32)
                    .map(|i| view! { <line class="band" x1=x - dam.0 / 2 + 4 x2=x + dam.0 / 2 - 4 y1=y + dam.1 / 2 - 8 * i y2=y + dam.1 / 2 - 8 * i stroke=ink /> })
                    .collect_view();
                view! {
                    <g class="piece dam">
                        <title>{title}</title>
                        <rect x=x - dam.0 / 2 y=y + dam.1 / 2 - h width=dam.0 height=h rx=4 fill=fill />
                        {bands}
                        <text class="level" x=x y=y + dam.1 / 2 - 4 fill=ink>{built.level}</text>
                        {drops(x, y + dam.1 / 2 - h - 17, built.water, false)}
                    </g>
                }
                .into_any()
            });
            let class = match (paid, printed) {
                (true, true) => "space dam-space paid printed",
                (true, false) => "space dam-space paid",
                (false, true) => "space dam-space printed",
                (false, false) => "space dam-space",
            };
            view! {
                <g class=class>
                    <rect x=x - dam.0 / 2 y=y - dam.1 / 2 width=dam.0 height=dam.1 rx=4 />
                    {(paid && !printed).then(|| view! { <text class="fee" x=x + dam.0 / 2 - 10 y=y + 5>"3"</text> })}
                </g>
                {piece}
            }
        })
        .collect_view();
    let conduits = CONDUITS
        .iter()
        .enumerate()
        .map(|(c, conduit)| {
            let (x, y) = conduit_pos(c, printed);
            let built = view.conduits[c].is_some();
            let title = format!("Conduit {} from {} to {}", conduit.value, look::basin_name(conduit.basin), look::basin_name(conduit.to));
            if printed {
                let piece = view.conduits[c].map(|s| {
                    let title = format!("{}'s Conduit", look::name(s, names));
                    view! {
                        <g class="piece conduit">
                            <title>{title}</title>
                            {piece_image(look::piece_art(company_of(view, Some(s)), Kind::Conduit), x, y + 12, 30, 46)}
                        </g>
                    }
                });
                return view! {
                    <g class=if built { "space conduit-space printed built" } else { "space conduit-space printed" }>
                        <title>{title}</title>
                        <circle cx=x cy=y r=P_CONDUIT_R />
                    </g>
                    {piece}
                }
                .into_any();
            }
            let (fill, ink) = match view.conduits[c] {
                Some(s) => look::seat_colors(view, s),
                None => ("#f6f1e4", "#2b2116"),
            };
            view! {
                <g class=if built { "space conduit-space built" } else { "space conduit-space" }>
                    <title>{title}</title>
                    <circle cx=x cy=y r=CONDUIT_R fill=fill stroke=look::value_color(conduit.value) />
                    <text x=x y=y + 6 fill=ink>{conduit.value}</text>
                </g>
            }
            .into_any()
        })
        .collect_view();
    let ph = if printed { P_PH } else { (PH, PH) };
    let powerhouses = POWERHOUSES
        .iter()
        .enumerate()
        .map(|(p, space)| {
            let (x, y) = powerhouse_pos(p, printed);
            let piece = view.powerhouses[p].map(|s| {
                let title = format!("{}'s Powerhouse", look::name(s, names));
                if printed {
                    let art = look::piece_art(company_of(view, Some(s)), Kind::Powerhouse);
                    return view! {
                        <g class="piece powerhouse">
                            <title>{title}</title>
                            {piece_image(art, x, y + ph.1 / 2 + 2, ph.0 - 4, 42)}
                        </g>
                    }
                    .into_any();
                }
                let (fill, ink) = look::seat_colors(view, s);
                view! {
                    <g class="piece powerhouse">
                        <title>{title}</title>
                        <rect x=x - PH / 2 + 3 y=y - PH / 2 + 3 width=PH - 6 height=PH - 6 rx=5 fill=fill />
                        <path class="bolt" d=format!("M{} {} l-8 12 h7 l-4 11 l11 -15 h-7 l4 -8 Z", x + 2, y - 13) fill=ink />
                    </g>
                }
                .into_any()
            });
            let class = match (space.paid, printed) {
                (true, true) => "space ph-space paid printed",
                (true, false) => "space ph-space paid",
                (false, true) => "space ph-space printed",
                (false, false) => "space ph-space",
            };
            view! {
                <g class=class>
                    <rect x=x - ph.0 / 2 y=y - ph.1 / 2 width=ph.0 height=ph.1 rx=6 />
                    {(space.paid && !printed).then(|| view! { <text class="fee" x=x + PH / 2 - 7 y=y - PH / 2 + 11>"3"</text> })}
                </g>
                {piece}
            }
        })
        .collect_view();
    view! { <g class="spaces">{dams}{conduits}{powerhouses}</g> }.into_any()
}

/// Where a target sits on the map, and how big its click area is.
fn target_area(target: Target, printed: bool) -> (i32, i32, i32, i32) {
    let (dam, conduit_r, ph, m) = if printed { (P_DAM, P_CONDUIT_R, P_PH, 3) } else { (DAM, CONDUIT_R, (PH, PH), 4) };
    match target {
        Target::Base(d) | Target::Elevation(d) => {
            let (x, y) = dam_pos(d, printed);
            (x - dam.0 / 2 - m, y - dam.1 / 2 - m, dam.0 + 2 * m, dam.1 + 2 * m)
        }
        Target::Conduit(c) => {
            let (x, y) = conduit_pos(c, printed);
            (x - conduit_r - m, y - conduit_r - m, 2 * (conduit_r + m), 2 * (conduit_r + m))
        }
        Target::Powerhouse(p) => {
            let (x, y) = powerhouse_pos(p, printed);
            (x - ph.0 / 2 - m, y - ph.1 / 2 - m, ph.0 + 2 * m, ph.1 + 2 * m)
        }
    }
}

fn target_marks(
    app: App,
    view: &Arc<View>,
    marks: Vec<(Target, String)>,
    choice: Option<(Option<Tile>, Option<Tile>)>,
    mixers: bool,
    printed: bool,
) -> AnyView {
    let list = marks
        .into_iter()
        .map(|(target, cost)| {
            let (x, y, w, h) = target_area(target, printed);
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

fn headstreams(app: App, view: &View, water: Option<(u8, bool)>, placed: [u8; HEADSTREAMS], printed: bool) -> AnyView {
    let ui = app.games.barrage.ui;
    let total: u8 = placed.iter().sum();
    let list = (0..HEADSTREAMS)
        .map(|h| {
            let (x, y) = headstream_pos(h, printed);
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
            let class = match (open, printed) {
                (true, true) => "headstream open printed",
                (true, false) => "headstream open",
                (false, true) => "headstream printed",
                (false, false) => "headstream",
            };
            // The printed tiles' places are narrower than the drawn boxes.
            let half = if printed { 76 } else { 80 };
            let shown = waiting.min(if printed { 7 } else { 8 });
            view! {
                <g class=class on:click=click role=if open { "button" } else { "img" }>
                    <title>{format!("Headstream {label}: {} drop{} waiting; drops per round {:?}", view.headstreams[h], if view.headstreams[h] == 1 { "" } else { "s" }, tile)}</title>
                    <rect x=x - half y=y - 34 width=2 * half height=60 rx=12 />
                    <text class="hs-name" x=x - half + 12 y=y - 14>{label}</text>
                    <text class="hs-rounds" x=x + half - 14 y=y - 14>{rounds}</text>
                    {drops(x, if printed { y - 6 } else { y - 1 }, shown, printed)}
                    {(waiting > shown).then(|| view! { <text class="hs-count" x=x + half - 20 y=y + 16>{format!("\u{d7}{waiting}")}</text> })}
                    {(placed[h] > 0).then(|| view! { <text class="hs-added" x=x - half + 14 y=y + 18>{format!("+{}", placed[h])}</text> })}
                </g>
            }
        })
        .collect_view();
    view! { <g class="headstreams">{list}</g> }.into_any()
}

/// The whole map for the view, the targets of a click marked: the printed
/// board's picture, or the map drawn in flat colours (`drawn`).
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
    drawn: bool,
) -> AnyView {
    let printed = !drawn;
    let height = if printed { PRINTED_HEIGHT } else { HEIGHT };
    let ground = if printed {
        view! {
            <image class="board-art" href=look::BOARD_ART x="0" y="0" width=WIDTH height=height preserveAspectRatio="none" />
        }
        .into_any()
    } else {
        view! { {rivers()} {pipes(&view)} {basins()} }.into_any()
    };
    view! {
        <svg
            class="map"
            viewBox=format!("0 0 {WIDTH} {height}")
            role="img"
            aria-label="The map of the Alps"
        >
            {ground}
            {spaces(&view, names, printed)}
            {headstreams(app, &view, water, placed, printed)}
            {target_marks(app, &view, marks, choice, mixers, printed)}
        </svg>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use barrage_core::DAM_BASINS;

    /// No two building spaces overlap, and every one is inside its basin.
    #[test]
    fn the_spaces_sit_apart_inside_their_basins() {
        let mut boxes: Vec<(i32, i32, i32, i32)> = Vec::new();
        for d in 0..DAM_SPACES {
            boxes.push(target_area(Target::Base(d), false));
        }
        for c in 0..CONDUITS.len() {
            boxes.push(target_area(Target::Conduit(c), false));
        }
        for p in 0..POWERHOUSES.len() {
            boxes.push(target_area(Target::Powerhouse(p), false));
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
            assert!(inside(dam_basin(d), dam_pos(d, false)), "dam {d}");
        }
        for (c, conduit) in CONDUITS.iter().enumerate() {
            assert!(inside(conduit.basin, conduit_pos(c, false)), "conduit {c}");
        }
        for (p, space) in POWERHOUSES.iter().enumerate() {
            assert!(inside(space.basin, powerhouse_pos(p, false)), "powerhouse {p}");
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

    /// On the printed board too, and every space is where the board prints
    /// it: the upper dam above the lower, the left conduit left, the red
    /// powerhouse spaces first in each basin's top row.
    #[test]
    fn the_printed_spaces_sit_apart_on_the_board() {
        let mut boxes: Vec<(i32, i32, i32, i32)> = Vec::new();
        for d in 0..DAM_SPACES {
            boxes.push(target_area(Target::Base(d), true));
        }
        for c in 0..CONDUITS.len() {
            boxes.push(target_area(Target::Conduit(c), true));
        }
        for p in 0..POWERHOUSES.len() {
            boxes.push(target_area(Target::Powerhouse(p), true));
        }
        for (i, a) in boxes.iter().enumerate() {
            assert!(a.0 >= 0 && a.1 >= 0 && a.0 + a.2 <= WIDTH && a.1 + a.3 <= PRINTED_HEIGHT, "{a:?} off the board");
            for b in &boxes[i + 1..] {
                let apart = a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1;
                assert!(apart, "{a:?} overlaps {b:?}");
            }
        }
        for b in 0..DAM_BASINS {
            let (ux, uy) = dam_pos(2 * b, true);
            let (lx, ly) = dam_pos(2 * b + 1, true);
            assert!(uy + 60 < ly && (ux - lx).abs() < 4, "{}", look::basin_name(b));
            let (left, ly2) = conduit_pos(2 * b, true);
            let (right, ry) = conduit_pos(2 * b + 1, true);
            assert!(left < ux && ux < right && ly2 == ry && uy < ry && ry < ly, "{}", look::basin_name(b));
        }
        for b in 0..BASINS.len() {
            let spaces: Vec<usize> = (0..POWERHOUSES.len()).filter(|&p| POWERHOUSES[p].basin == b).collect();
            let top = spaces.iter().map(|&p| powerhouse_pos(p, true).1).min();
            for &p in &spaces {
                if POWERHOUSES[p].paid && b != barrage_core::B2 {
                    assert_eq!(Some(powerhouse_pos(p, true).1), top, "{} {p}", look::basin_name(b));
                }
            }
        }
        // The Headstreams above their Mountains basins.
        for h in 0..HEADSTREAMS {
            let (x, y) = headstream_pos(h, true);
            let (dx, dy) = dam_pos(2 * h, true);
            assert!((x - dx).abs() < 4 && y < dy - 100);
        }
    }
}
