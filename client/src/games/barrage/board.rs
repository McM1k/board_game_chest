//! The Barrage game screen: the map, what you can do now, the Management
//! board, the Contracts, the companies, the rounds and the log.
//!
//! Everything is drawn from the `View` the server sent for our seat; every
//! click asks the server for a move (`GameMove::Barrage`), whose answer is
//! the next state, which rebuilds the screen. A move that needs more than
//! one click (a Turbine Station space, then which production) is built up
//! in `Ui` first. What a click means is decided in `look`, so it can be
//! tested natively.

use std::sync::Arc;

use barrage_core::{
    rows, Action, Bonus, Company, Kind, Phase, Pick, Section, Spot, Tile, View, CONTRACTS_TAKEN, FEE,
    HEADSTREAMS, OBJECTIVE_VP, WHEEL,
};
use leptos::prelude::*;
use protocol::{ClientMsg, GameMove};

use super::look;
use super::map::map_svg;
use super::Ui;
use crate::app::{leave, send, App};

/// What every part of the board needs. Cheap to clone: the state is shared.
#[derive(Clone)]
struct Ctx {
    app: App,
    view: Arc<View>,
    names: Arc<Vec<String>>,
    connected: Arc<Vec<bool>>,
}

impl Ctx {
    fn ui(&self) -> Ui {
        self.app.games.barrage.ui
    }

    fn act(&self, action: Action) {
        send(self.app, &ClientMsg::Move(GameMove::Barrage(action)));
    }

    fn who(&self, seat: usize) -> String {
        look::who(seat, &self.names, self.view.you)
    }

    /// Our move now.
    fn acting(&self) -> bool {
        self.view.actor == Some(self.view.you)
    }

    /// Our move, with nothing waiting to be settled.
    fn free_to_act(&self) -> bool {
        self.acting() && self.view.todo.is_none() && self.view.phase == Phase::Actions
    }
}

fn plural(n: impl Into<u64>) -> &'static str {
    if n.into() == 1 {
        ""
    } else {
        "s"
    }
}

/// The running game, redrawn whenever the server sends a new state (or a
/// move is being put together).
pub fn board(app: App) -> AnyView {
    let barrage = app.games.barrage;
    (move || {
        let view = barrage.view.get()?;
        Some(game_screen(app, view, barrage.names.get(), barrage.connected.get(), app.room.get()))
    })
    .into_any()
}

/// The whole game screen for one state the server sent.
pub fn game_screen(app: App, view: View, names: Vec<String>, connected: Vec<bool>, room: String) -> impl IntoView {
    let ui = app.games.barrage.ui;
    let spot = ui.spot.get();
    let choice = ui.tile.get();
    let mixers = ui.mixers.get();
    let placed = ui.drops.get();
    let ctx = Ctx { app, view: Arc::new(view), names: Arc::new(names), connected: Arc::new(connected) };
    let v = &ctx.view;
    let over = v.phase == Phase::Over;
    let marks = if ctx.acting() { look::targets(v, choice, mixers) } else { Vec::new() };
    let water = if ctx.acting() { look::water_wanted(v, spot) } else { None };
    let drawn = ui.drawn.get();
    let map = map_svg(app, Arc::clone(&ctx.view), &ctx.names, marks, choice, mixers, water, placed, drawn);

    view! {
        <div class="game-board">
            {top_bar(app, room)}
            {status_band(&ctx)}
            <div class="board-grid">
                <div class="map-col">
                    <div class="map-frame">{map}</div>
                    {map_style(&ctx, drawn)}
                    {legend(drawn)}
                </div>
                <aside class="side-col">
                    {over.then(|| results_panel(&ctx))}
                    {(!over).then(|| action_panel(&ctx, spot, choice, mixers, placed))}
                    {(v.phase != Phase::Setup).then(|| management_panel(&ctx, spot))}
                    {(v.phase != Phase::Setup).then(|| contracts_panel(&ctx, spot))}
                    {players_panel(&ctx)}
                    {rounds_panel(&ctx)}
                    {log_panel(&ctx)}
                </aside>
            </div>
        </div>
    }
}

fn top_bar(app: App, room: String) -> AnyView {
    view! {
        <header class="top-bar">
            <h1 class="brand">"Barrage"</h1>
            <span class="room">"Room " <b class="room-tag">{room}</b></span>
            <button class="link" on:click=move |_| leave(app)>
                "Leave game"
            </button>
        </header>
    }
    .into_any()
}

fn status_band(ctx: &Ctx) -> AnyView {
    let v = &ctx.view;
    let round = v.round.max(1);
    view! {
        <section class="status" aria-live="polite">
            <div class="turn" aria-label=format!("Round {round} of 5")>
                <span class="turn-word">"Round"</span>
                <span class="turn-num">{round}</span>
                <span class="turn-of">"of 5"</span>
            </div>
            <div class="phase">
                <h2>{look::phase_title(v, &ctx.names)}</h2>
                <p class="hint">{look::hint(v, &ctx.names)}</p>
            </div>
        </section>
    }
    .into_any()
}

/// The printed board's picture, or the map drawn in flat colours.
fn map_style(ctx: &Ctx, drawn: bool) -> AnyView {
    let ui = ctx.ui();
    let choice = move |label: &'static str, on: bool| {
        view! {
            <button class=if on == drawn { "tool on" } else { "tool" } aria-pressed=(on == drawn).to_string() on:click=move |_| ui.set_drawn(on)>
                {label}
            </button>
        }
    };
    view! {
        <div class="tools map-style" role="group" aria-label="Map style">
            {choice("Printed board", false)}
            {choice("Drawn map", true)}
        </div>
    }
    .into_any()
}

/// A token's picture before a count, the count's meaning in words for a
/// screen reader and on hover.
/// Without a picture, the word follows the count.
fn token(src: Option<String>, n: impl std::fmt::Display, what: &str) -> AnyView {
    let label = format!("{n} {what}");
    let text = if src.is_some() { n.to_string() } else { label.clone() };
    view! {
        <span class="res" title=label.clone() aria-label=label>
            {src.map(|src| view! { <img src=src alt="" aria-hidden="true" /> })}
            {text}
        </span>
    }
    .into_any()
}

/// A seat's Credits, Machineries and Engineers, as tokens.
fn stock(v: &View, seat: usize) -> AnyView {
    let p = &v.players[seat];
    let engineer = p.company.map(look::engineer_art);
    view! {
        <span class="res-row">
            {token(Some(look::token_art("credit")), p.credits, "Credits")}
            {token(Some(look::token_art("excavator")), p.excavators, "Excavators")}
            {token(Some(look::token_art("mixer")), p.mixers, "Concrete Mixers")}
            {token(engineer, p.engineers, "Engineers left")}
        </span>
    }
    .into_any()
}

fn legend(drawn: bool) -> AnyView {
    let values = (1..=5u8)
        .map(|v| {
            view! {
                <li>
                    <svg class="key" viewBox="0 0 30 12" aria-hidden="true">
                        <line x1="2" y1="6" x2="28" y2="6" stroke=look::value_color(v) stroke-width="5" stroke-linecap="round" />
                    </svg>
                    {format!("Conduit {v}")}
                </li>
            }
        })
        .collect_view();
    view! {
        <ul class="legend" aria-label="Map key">
            {values}
            <li><span class="key-box paid"></span>"Red space: 3 Credits more"</li>
            {if drawn {
                view! { <li><span class="key-box neutral"></span>"Neutral Dam"</li> }.into_any()
            } else {
                view! {
                    <li><img class="key-piece" src=look::piece_art(None, Kind::Base) alt="" />"Neutral Dam"</li>
                    <li><img class="key-piece" src=look::token_art("drop") alt="" />"Water"</li>
                }
                .into_any()
            }}
            <li>"Water flows down: Headstreams A to D, then from basin to basin; each basin's upper dam fills first."</li>
        </ul>
    }
    .into_any()
}

// ----- what you can do now ------------------------------------------------------------

fn button(ctx: &Ctx, label: impl Into<String>, class: &'static str, action: Action) -> AnyView {
    let ctx = ctx.clone();
    let label = label.into();
    view! { <button class=class on:click=move |_| ctx.act(action.clone())>{label}</button> }.into_any()
}

fn setup_picker(ctx: &Ctx) -> AnyView {
    let v = &ctx.view;
    let Some(setup) = &v.setup else { return ().into_any() };
    let ui = ctx.ui();
    let chosen_pair = ui.pair.get();
    let chosen_contract = ui.contract.get();
    let pairs = setup
        .pairs
        .iter()
        .enumerate()
        .map(|(i, &(company, officer, taken))| {
            let (fill, ink) = look::company_colors(company);
            let class = match (taken, chosen_pair == Some(i)) {
                (Some(_), _) => "pick taken",
                (None, true) => "pick on",
                _ => "pick",
            };
            let by = taken.map(|s| format!(" (taken by {})", ctx.who(s)));
            view! {
                <li>
                    <button class=class disabled=taken.is_some() on:click=move |_| ui.pair.set(Some(i))>
                        <span class="swatch" style=format!("background:{fill};color:{ink}")></span>
                        <b>{company.name()}</b>{format!(" \u{b7} {}", company.ceo())}{by}
                        <span class="pick-line">{format!("{}: {}", officer.name(), officer.ability())}</span>
                        <span class="pick-line">{format!("Third Powerhouse: {}", company.ability())}</span>
                    </button>
                </li>
            }
        })
        .collect_view();
    let contracts = setup
        .contracts
        .iter()
        .enumerate()
        .map(|(i, &(id, taken))| {
            let class = match (taken, chosen_contract == Some(i)) {
                (Some(_), _) => "pick taken",
                (None, true) => "pick on",
                _ => "pick",
            };
            view! {
                <li>
                    <button class=class disabled=taken.is_some() on:click=move |_| ui.contract.set(Some(i))>
                        <span class=look::contract_class(id)>{look::contract_text(id)}</span>
                    </button>
                </li>
            }
        })
        .collect_view();
    let ready = chosen_pair.zip(chosen_contract);
    let c = ctx.clone();
    view! {
        <p class="caption">"A Company and its Executive Officer:"</p>
        <ul class="picks">{pairs}</ul>
        <p class="caption">"A Starting Contract:"</p>
        <ul class="picks">{contracts}</ul>
        <div class="row">
            <button class="primary" disabled=ready.is_none() on:click=move |_| {
                if let Some((pair, contract)) = ready {
                    c.act(Action::Choose { pair, contract });
                }
            }>"Take them"</button>
        </div>
    }
    .into_any()
}

/// Drops to place on the Headstreams, by clicking them on the map.
fn water_picker(ctx: &Ctx, n: u8, now: bool, placed: [u8; HEADSTREAMS], spot: Option<Spot>, optional: bool) -> AnyView {
    let ui = ctx.ui();
    let total: u8 = placed.iter().sum();
    let when = if now { "they flow at once" } else { "they flow at the end of the round" };
    let at: Vec<String> = placed
        .iter()
        .enumerate()
        .filter(|(_, &d)| d > 0)
        .map(|(h, &d)| format!("{d} on {}", ["A", "B", "C", "D"][h]))
        .collect();
    // The Water Management space places 1 or 2 drops; a reward up to its number.
    let ready = spot.is_none() || (total >= 1 && (!now || total == 1));
    let c = ctx.clone();
    view! {
        <p>{format!("Click the Headstreams at the top of the map to place {n} drop{} ({when}).", plural(n))}</p>
        <p class="hint">{if at.is_empty() { "None placed yet.".to_string() } else { format!("Placed: {}.", at.join(", ")) }}</p>
        <div class="row">
            <button class="primary" disabled=!ready on:click=move |_| {
                c.act(Action::Water { space: spot, drops: placed });
            }>{if spot.is_some() { "Place them" } else if total < n && optional { "Place these" } else { "Place them" }}</button>
            <button on:click=move |_| ui.drops.set([0; HEADSTREAMS])>"Start over"</button>
            {spot.is_some().then(|| view! { <button class="link" on:click=move |_| ui.clear()>"Cancel"</button> })}
        </div>
    }
    .into_any()
}

/// Productions to choose from: one button per number of drops, with the
/// energy it makes.
fn production_picker(ctx: &Ctx, spot: Option<Spot>) -> AnyView {
    let v = &ctx.view;
    let row = spot.map(|s| s.row);
    let credits = look::credits_after(v, spot);
    let options = v
        .moves
        .productions
        .iter()
        .map(|o| {
            let most = o.most(credits);
            let choices = (1..=most)
                .filter_map(|drops| {
                    let energy = look::option_energy(v, o, drops, row);
                    (energy >= 1).then(|| {
                        let c = ctx.clone();
                        let production = look::production(o, drops);
                        view! {
                            <button class="tool" on:click=move |_| c.act(Action::Produce { space: spot, production })>
                                {format!("{drops} drop{} \u{2192} {energy} energy", plural(drops))}
                            </button>
                        }
                    })
                })
                .collect::<Vec<_>>();
            let none = choices.is_empty();
            view! {
                <li class=if none { "production none" } else { "production" }>
                    <span>{look::option_text(v, &ctx.names, o)}</span>
                    <div class="tools">{choices}</div>
                </li>
            }
        })
        .collect_view();
    view! {
        <p class="caption">"Produce: drops from a Dam, through a Conduit, to your Powerhouse."</p>
        <ul class="productions">{options}</ul>
    }
    .into_any()
}

/// The Contracts in your supply, each with a button.
fn contract_buttons(ctx: &Ctx, ids: &[u8], label: &'static str, make: fn(u8) -> Action) -> AnyView {
    let list = ids
        .iter()
        .map(|&id| {
            let c = ctx.clone();
            view! {
                <li>
                    <span class=look::contract_class(id)>{look::contract_text(id)}</span>
                    <button class="tool" on:click=move |_| c.act(make(id))>{label}</button>
                </li>
            }
        })
        .collect_view();
    view! { <ul class="contract-list">{list}</ul> }.into_any()
}

/// A decision waiting in the todo list.
fn todo_panel(ctx: &Ctx, placed: [u8; HEADSTREAMS]) -> AnyView {
    let v = &ctx.view;
    let m = &v.moves;
    let todo = v.todo.expect("a todo");
    // A Contract to fulfill has its own "keep them" button.
    let skip = (m.skip && m.fulfill.is_empty()).then(|| button(ctx, "Skip", "", Action::Skip));
    let body = if let Some(n) = m.machinery {
        let splits = look::machinery_splits(n)
            .into_iter()
            .map(|(e, mix)| button(ctx, look::splits_text(e, mix), "tool", Action::Machinery { excavators: e }))
            .collect_view();
        view! { <p>{format!("Take {n} Machiner{}:", if n == 1 { "y" } else { "ies" })}</p><div class="tools">{splits}</div> }
            .into_any()
    } else if let Some((n, now)) = m.water {
        water_picker(ctx, n, now, placed, None, true)
    } else if !m.places.is_empty() {
        view! { <p>{format!("Click a marked space on the map to {}.", look::todo_text(&todo).replace("their", "your"))}</p> }
            .into_any()
    } else if !m.fulfill.is_empty() {
        let energy = match todo.task {
            barrage_core::Task::Contract { energy } => energy,
            _ => 0,
        };
        view! {
            <p>{format!("You produced {energy} energy: fulfill one Contract?")}</p>
            {contract_buttons(ctx, &m.fulfill, "Fulfill", |id| Action::Fulfill { contract: Some(id) })}
            <div class="row">{button(ctx, "Keep them for later", "", Action::Fulfill { contract: None })}</div>
        }
        .into_any()
    } else if m.discard {
        view! {
            <p>"You may keep 3 Contracts: discard one."</p>
            {contract_buttons(ctx, &v.players[v.you].contracts, "Discard", |id| Action::Discard { contract: id })}
        }
        .into_any()
    } else if !m.productions.is_empty() {
        production_picker(ctx, None)
    } else {
        ().into_any()
    };
    view! {
        {body}
        <div class="row">{skip}</div>
    }
    .into_any()
}

/// Building: the tiles to build with, then a click on the map.
fn build_section(ctx: &Ctx, choice: Option<(Option<Tile>, Option<Tile>)>, mixers: bool) -> AnyView {
    let v = &ctx.view;
    let ui = ctx.ui();
    let choices = look::tile_choices(v);
    let next = match look::next_construction(v, v.you) {
        Some((e, fee)) => format!(
            "Your next construction takes {e} Engineer{}{}.",
            plural(e),
            if fee { " and 3 Credits" } else { "" }
        ),
        None => "Your Company board has no construction space left this round.".into(),
    };
    if choices.is_empty() {
        return view! { <p class="hint">{next}" Nothing can be built right now."</p> }.into_any();
    }
    let buttons = choices
        .into_iter()
        .map(|key| {
            let on = choice == Some(key);
            let title = key.1.or(key.0).map(look::tile_text).unwrap_or_default();
            view! {
                <button
                    class=if on { "tool on" } else { "tool" }
                    title=title
                    aria-pressed=on.to_string()
                    on:click=move |_| {
                        ui.spot.set(None);
                        ui.tile.set(if on { None } else { Some(key) });
                    }
                >
                    {look::choice_name(key.0, key.1)}
                </button>
            }
        })
        .collect_view();
    let mcdowell = v.moves.builds.iter().any(|o| o.build.mixers);
    let jill = mcdowell.then(|| {
        view! {
            <label class="option-toggle">
                <input type="checkbox" prop:checked=mixers on:change=move |_| ui.mixers.set(!mixers) />
                <span>"Pay Conduits with Concrete Mixers (Jill McDowell)"</span>
            </label>
        }
    });
    view! {
        <div class="build">
            <p class="caption">"Build with a Technology tile:"</p>
            <div class="tools">{buttons}</div>
            {jill}
            <p class="hint">
                {next}
                {if choice.is_some() { " Click a marked space on the map (E: Excavators, M: Concrete Mixers, \u{a2}: Credits)." } else { "" }}
            </p>
        </div>
    }
    .into_any()
}

/// The rest of a move on a Management space that needs more than a click.
fn spot_section(ctx: &Ctx, spot: Spot, placed: [u8; HEADSTREAMS]) -> AnyView {
    let v = &ctx.view;
    let ui = ctx.ui();
    let cancel = view! { <button class="link" on:click=move |_| ui.clear()>"Cancel"</button> };
    let body = match spot.section {
        Section::Turbine => production_picker(ctx, Some(spot)),
        Section::Water => {
            let (n, now) = look::water_wanted(v, Some(spot)).unwrap_or((1, true));
            return view! { <div class="spot-move">{water_picker(ctx, n, now, placed, Some(spot), false)}</div> }.into_any();
        }
        Section::Shop => view! {
            <p>"Which Machinery?"</p>
            <div class="tools">
                {button(ctx, "1 Excavator", "tool", Action::Buy { space: spot, mixer: false })}
                {button(ctx, "1 Concrete Mixer", "tool", Action::Buy { space: spot, mixer: true })}
            </div>
        }
        .into_any(),
        Section::Contracts => {
            let picks = ui.picks.get();
            let most = CONTRACTS_TAKEN[spot.row as usize];
            let ready = look::picks_ready(spot, &picks);
            let c = ctx.clone();
            view! {
                <p>{format!("Pick {} face-up Contract{} in the Contract Office below.", most, plural(most as u8))}</p>
                <div class="row">
                    <button class="primary" disabled=!ready on:click=move |_| c.act(Action::Contracts { space: spot, picks: picks.clone() })>
                        "Take them"
                    </button>
                </div>
            }
            .into_any()
        }
        _ => ().into_any(),
    };
    view! {
        <div class="spot-move">
            <p class="caption">{look::spot_label(v, spot)}</p>
            {body}
            <div class="row">{cancel}</div>
        </div>
    }
    .into_any()
}

fn bank_section(ctx: &Ctx) -> AnyView {
    let v = &ctx.view;
    let most = v.moves.bank;
    if most == 0 {
        return ().into_any();
    }
    let ui = ctx.ui();
    let n = ui.bank.get().clamp(1, most);
    let c = ctx.clone();
    view! {
        <div class="bank">
            <span class="caption">"Bank:"</span>
            <button class="tool" disabled={n <= 1} on:click=move |_| ui.bank.set(n - 1)>"\u{2212}"</button>
            <span class="count">{n}</span>
            <button class="tool" disabled={n >= most} on:click=move |_| ui.bank.set(n + 1)>"+"</button>
            <button on:click=move |_| c.act(Action::Bank { engineers: n })>
                {format!("Send {n} Engineer{} for {n} Credit{}", plural(n), plural(n))}
            </button>
        </div>
    }
    .into_any()
}

fn sekibo_section(ctx: &Ctx) -> AnyView {
    let v = &ctx.view;
    if v.moves.copy.is_empty() {
        return ().into_any();
    }
    let p = &v.players[v.you];
    let fee = if p.sekibo == 1 { " (3 Credits, the second time this round)" } else { "" };
    let choices = v
        .moves
        .copy
        .iter()
        .map(|&o| {
            let c = ctx.clone();
            view! { <button class="tool" title=look::officer_line(o) on:click=move |_| c.act(Action::Copy { officer: o })>{o.name()}</button> }
        })
        .collect_view();
    view! {
        <div class="copy">
            <p class="caption">{format!("Mahiri Sekibo: place an Engineer to copy an officer for this turn{fee}, then act.")}</p>
            <div class="tools">{choices}</div>
        </div>
    }
    .into_any()
}

fn action_panel(
    ctx: &Ctx,
    spot: Option<Spot>,
    choice: Option<(Option<Tile>, Option<Tile>)>,
    mixers: bool,
    placed: [u8; HEADSTREAMS],
) -> AnyView {
    let v = &ctx.view;
    if !ctx.acting() {
        let waiting = match (v.actor, &v.todo) {
            (Some(s), Some(t)) => format!("Waiting for {} to {}.", look::name(s, &ctx.names), look::todo_text(t)),
            (Some(s), None) => format!("Waiting for {}.", look::name(s, &ctx.names)),
            _ => String::new(),
        };
        return view! {
            <section class="panel actions">
                <h2>"Your move"</h2>
                <p class="hint">{waiting}</p>
            </section>
        }
        .into_any();
    }
    let p = &v.players[v.you];
    let body = if v.phase == Phase::Setup {
        setup_picker(ctx)
    } else if v.todo.is_some() {
        todo_panel(ctx, placed)
    } else {
        let copied = p.copied.map(|o| view! { <p class="note">{format!("Copying {} for this turn.", o.name())}</p> });
        let pass = v.moves.pass.then(|| button(ctx, "Pass", "danger", Action::Pass));
        view! {
            <p class="in-hand">{stock(v, v.you)}</p>
            {copied}
            {spot.map(|s| spot_section(ctx, s, placed))}
            {spot.is_none().then(|| view! {
                {build_section(ctx, choice, mixers)}
                {(!v.moves.spots.is_empty()).then(|| view! { <p class="hint">"Or take a space of the Management board below."</p> })}
                {bank_section(ctx)}
                {sekibo_section(ctx)}
            })}
            <div class="row">{pass}</div>
        }
        .into_any()
    };
    view! {
        <section class="panel actions you-act">
            <h2>"Your move"</h2>
            {body}
        </section>
    }
    .into_any()
}

// ----- the Management board --------------------------------------------------------------

/// Engineer figures, for an action space.
fn engineers(n: u8) -> AnyView {
    let figures = (0..n)
        .map(|_| {
            view! {
                <svg class="eng" viewBox="0 0 10 14" aria-hidden="true">
                    <circle cx="5" cy="3.5" r="3" />
                    <path d="M0.5 14 Q0.5 7.5 5 7.5 Q9.5 7.5 9.5 14 Z" />
                </svg>
            }
        })
        .collect_view();
    view! { <span class="engineers">{figures}</span> }.into_any()
}

fn space_button(ctx: &Ctx, spot: Spot, chosen: Option<Spot>) -> AnyView {
    let v = &ctx.view;
    let Some((def, row)) = spot.def() else { return ().into_any() };
    let closed = row.players > v.players.len();
    let taken = v.placements.iter().find(|p| p.spot == spot).map(|p| p.seat);
    let open = ctx.free_to_act() && v.moves.spots.contains(&spot);
    let mut class = String::from("space-btn");
    if def.fee {
        class.push_str(" fee");
    }
    if closed {
        class.push_str(" closed");
    }
    if open {
        class.push_str(" open");
    }
    if chosen == Some(spot) {
        class.push_str(" on");
    }
    let occupant = taken.map(|s| {
        let (fill, _) = look::seat_colors(v, s);
        view! { <span class="occupant" style=format!("background:{fill}") title=look::name(s, &ctx.names)></span> }
    });
    let mut label = format!("{}: {} Engineer{}", look::spot_label(v, spot), def.engineers, plural(def.engineers));
    if def.fee {
        label.push_str(" and 3 Credits");
    }
    if closed {
        label.push_str(" (not with this many players)");
    }
    let c = ctx.clone();
    let ui = ctx.ui();
    let click = move |_| {
        if let Some(action) = look::spot_action(spot) {
            c.act(action);
        } else {
            ui.clear();
            ui.spot.set(Some(spot));
        }
    };
    view! {
        <button class=class disabled=!open title=label.clone() aria-label=label on:click=click>
            {engineers(def.engineers)}
            {def.fee.then(|| view! { <span class="fee-tag">{format!("+{FEE}")}</span> })}
            {occupant}
        </button>
    }
    .into_any()
}

fn section_panel(ctx: &Ctx, section: Section, chosen: Option<Spot>) -> AnyView {
    let v = &ctx.view;
    let list = rows(section)
        .iter()
        .enumerate()
        .map(|(r, row)| {
            let left = Spot::new(section, r as u8, false);
            let right = row.right.map(|_| Spot::new(section, r as u8, true));
            let extra = (section == Section::Patent).then(|| {
                v.patents[r].map(|t| view! { <span class="tile-effect">{barrage_core::advanced_effect(t)}</span> })
            });
            view! {
                <li class="mrow">
                    {space_button(ctx, left, chosen)}
                    <span class="mlabel">{look::spot_label(v, left)}{extra}</span>
                    {right.map(|s| space_button(ctx, s, chosen))}
                </li>
            }
        })
        .collect_view();
    view! {
        <div class="section">
            <h3>{section.name()}</h3>
            <ul>{list}</ul>
        </div>
    }
    .into_any()
}

fn management_panel(ctx: &Ctx, chosen: Option<Spot>) -> AnyView {
    let v = &ctx.view;
    let bank: Vec<String> = v
        .placements
        .iter()
        .filter(|p| p.spot.section == Section::Bank)
        .map(|p| format!("{} {}", look::name(p.seat, &ctx.names), p.engineers))
        .collect();
    let sections = [Section::Turbine, Section::Water, Section::Workshop, Section::Shop, Section::Contracts, Section::Patent]
        .into_iter()
        .map(|s| section_panel(ctx, s, chosen))
        .collect_view();
    view! {
        <section class="panel management">
            <h2>"Management board"</h2>
            <div class="sections">{sections}</div>
            <p class="note">{if bank.is_empty() { "Bank: nobody yet this round.".to_string() } else { format!("Bank: {}.", bank.join(", ")) }}</p>
            <p class="hint">"Grey spaces are closed with this many players; red ones cost 3 Credits more."</p>
        </section>
    }
    .into_any()
}

fn contracts_panel(ctx: &Ctx, chosen: Option<Spot>) -> AnyView {
    let v = &ctx.view;
    let ui = ctx.ui();
    let picking = chosen.filter(|s| s.section == Section::Contracts && ctx.free_to_act());
    let picks = ui.picks.get();
    let tiers = ["Green", "Yellow", "Red"]
        .into_iter()
        .enumerate()
        .map(|(t, tier)| {
            let cards = (0..2u8)
                .map(|slot| {
                    let pick = Pick { tier: t as u8, slot };
                    match v.offers[t][slot as usize] {
                        Some(id) => {
                            let on = picks.contains(&pick);
                            let most = picking.map_or(0, |s| CONTRACTS_TAKEN[s.row as usize]);
                            let toggle = move |_| {
                                if most > 0 {
                                    ui.picks.update(|p| *p = look::toggle_pick(p, pick, most));
                                }
                            };
                            view! {
                                <li>
                                    <button
                                        class=if on { "card on" } else { "card" }
                                        disabled=picking.is_none()
                                        aria-pressed=on.to_string()
                                        on:click=toggle
                                    >
                                        <span class=look::contract_class(id)>{look::contract_text(id)}</span>
                                    </button>
                                </li>
                            }
                            .into_any()
                        }
                        None => view! { <li class="empty">"\u{2014}"</li> }.into_any(),
                    }
                })
                .collect_view();
            view! {
                <div class="tier">
                    <h3>{format!("{tier} ({} left)", v.piles[t])}</h3>
                    <ul class="cards">{cards}</ul>
                </div>
            }
        })
        .collect_view();
    let nationals = v
        .nationals
        .iter()
        .map(|&id| view! { <li><span class=look::contract_class(id)>{look::contract_text(id)}</span></li> })
        .collect_view();
    view! {
        <section class="panel contracts-panel">
            <h2>"Contract Office"</h2>
            <div class="tiers">{tiers}</div>
            <h3>"National Contracts (the first to produce enough takes one)"</h3>
            <ul class="cards nationals">{nationals}</ul>
        </section>
    }
    .into_any()
}

// ----- the companies ------------------------------------------------------------------

fn wheel_view(v: &View, seat: usize) -> AnyView {
    let p = &v.players[seat];
    // The segments in the order they come back, the next one first.
    let cells = (1..WHEEL)
        .map(|k| {
            let seg = p.wheel[(p.open + k) % WHEEL];
            let text = if seg.is_empty() {
                "\u{b7}".to_string()
            } else {
                let tile = seg.tile.map(|t| t.name()).unwrap_or_else(|| "copy".into());
                look::cost_text(seg.excavators, seg.mixers, 0).replace("free", "").trim().to_string() + " " + &tile
            };
            view! {
                <li class=if seg.is_empty() { "seg empty" } else { "seg" } title=format!("Back in {k} turn{}", plural(k as u8))>
                    <span class="seg-turns">{k}</span>
                    <span class="seg-text">{text}</span>
                </li>
            }
        })
        .collect_view();
    view! { <ol class="wheel" aria-label="Construction Wheel, next back first">{cells}</ol> }.into_any()
}

fn player_card(ctx: &Ctx, seat: usize) -> AnyView {
    let v = &ctx.view;
    let p = &v.players[seat];
    let connected = ctx.connected.get(seat).copied().unwrap_or(true);
    let (fill, ink) = look::seat_colors(v, seat);
    let company = p.company.map(|c| format!("{} \u{b7} {}", c.name(), c.ceo())).unwrap_or_else(|| "picking…".into());
    let officer = p.officer.map(|o| {
        let copied = p.copied.map(|c| format!(" (copying {})", c.name())).unwrap_or_default();
        view! { <p class="officer" title=o.ability()><b>{o.name()}</b>{copied}": "{o.ability()}</p> }
    });
    let ability = p.company.map(|c| {
        let on = p.built(Kind::Powerhouse) >= 3;
        view! {
            <p class=if on { "ability on" } else { "ability" }>
                {if on { "Ability on: " } else { "Third Powerhouse: " }}{c.ability()}
            </p>
        }
    });
    let pieces = Kind::ALL
        .iter()
        .map(|&k| view! { <li>{format!("{} {}/{}", k.plural(), p.built(k), k.pieces())}</li> })
        .collect_view();
    let tiles: Vec<String> = p.tiles.iter().map(|t| t.name()).collect();
    let contracts = p
        .contracts
        .iter()
        .map(|&id| view! { <li><span class=look::contract_class(id)>{look::contract_text(id)}</span></li> })
        .collect_view();
    let bonus = barrage_core::powerhouse_bonus(p.built(Kind::Powerhouse));
    let current = v.current == Some(seat) && v.phase == Phase::Actions;
    view! {
        <li class=if current { "player current" } else { "player" }>
            <div class="player-head">
                <span class="swatch" style=format!("background:{fill};color:{ink}")></span>
                <span class="player-name">{ctx.who(seat)}</span>
                {current.then(|| view! { <span class="tag">"turn"</span> })}
                {p.passed.then(|| view! { <span class="tag tag-off">"passed"</span> })}
                {(!connected).then(|| view! { <span class="tag tag-off">"offline"</span> })}
                <span class="vp" title="Victory Points">{format!("{} VP", p.vp)}</span>
            </div>
            <p class="company">{company}</p>
            {officer}
            {ability}
            <p class="stock">
                {stock(v, seat)}
                <span class="energy">{format!("\u{26a1} {} energy \u{b7} production {:+}", p.energy, bonus)}</span>
            </p>
            <ul class="pieces">{pieces}</ul>
            <p class="tiles">{format!("Tiles: {}", if tiles.is_empty() { "none (all on the wheel)".to_string() } else { tiles.join(", ") })}</p>
            {wheel_view(v, seat)}
            <ul class="cards mine">{contracts}</ul>
            <p class="note">{format!("{} Contract{} fulfilled", p.fulfilled.len(), plural(p.fulfilled.len() as u8))}</p>
        </li>
    }
    .into_any()
}

fn players_panel(ctx: &Ctx) -> AnyView {
    let v = &ctx.view;
    let order: Vec<usize> = if v.phase == Phase::Setup { (0..v.players.len()).collect() } else { v.order.clone() };
    let cards = order.into_iter().map(|s| player_card(ctx, s)).collect_view();
    view! {
        <section class="panel players-panel">
            <h2>"Companies, in turn order"</h2>
            <ul class="players">{cards}</ul>
        </section>
    }
    .into_any()
}

fn bonus_text(bonus: Bonus) -> String {
    bonus.text()
}

fn rounds_panel(ctx: &Ctx) -> AnyView {
    let v = &ctx.view;
    let tiles = v
        .bonus
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            let r = i as u8 + 1;
            let class = match r.cmp(&v.round) {
                std::cmp::Ordering::Less => "bonus past",
                std::cmp::Ordering::Equal => "bonus now",
                _ => "bonus",
            };
            view! {
                <li class=class>
                    <span class="bonus-round">{format!("Round {r}")}</span>
                    <span>{bonus_text(b)}</span>
                    <span class="bonus-need">{format!("full from {} energy", 6 * r)}</span>
                </li>
            }
        })
        .collect_view();
    let counts: Vec<String> =
        v.objective_counts.iter().enumerate().map(|(s, n)| format!("{} {n}", look::name(s, &ctx.names))).collect();
    view! {
        <section class="panel rounds">
            <h2>"Scoring"</h2>
            <ol class="bonuses">{tiles}</ol>
            <p class="note">
                "Each round: 6 VP to the most energy, 2 to the second; Credits by energy (3 and \u{2212}3 VP for none); the Bonus tile from 6 energy, 4 VP less for each section behind the round's."
            </p>
            <h3>"Objective"</h3>
            <p>{format!("{} \u{2014} {} / {} / {} VP", v.objective.text(), OBJECTIVE_VP[0], OBJECTIVE_VP[1], OBJECTIVE_VP[2])}</p>
            <p class="note">{format!("So far: {}.", counts.join(", "))}</p>
        </section>
    }
    .into_any()
}

fn log_panel(ctx: &Ctx) -> AnyView {
    let v = &ctx.view;
    let lines = v
        .log
        .iter()
        .rev()
        .take(18)
        .map(|event| view! { <li>{look::describe_event(event, &ctx.names, v.you)}</li> })
        .collect_view();
    view! {
        <section class="panel log-panel">
            <h2>"What happened"</h2>
            <ol class="log">{lines}</ol>
        </section>
    }
    .into_any()
}

fn results_panel(ctx: &Ctx) -> AnyView {
    let v = &ctx.view;
    let rows = look::standings(v)
        .into_iter()
        .map(|seat| {
            let won = v.winners.contains(&seat);
            let p = &v.players[seat];
            view! {
                <tr class=if won { "won" } else { "" }>
                    <td>{ctx.who(seat)}{won.then_some(" \u{2605}")}</td>
                    <td>{p.company.map(Company::name).unwrap_or_default()}</td>
                    <td class="num">{p.vp}</td>
                    <td class="num">{p.energy}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <section class="panel results">
            <h2>"Final scores"</h2>
            <table>
                <thead><tr><th>"Player"</th><th>"Company"</th><th class="num">"VP"</th><th class="num" title="The tie-break">"Last round's energy"</th></tr></thead>
                <tbody>{rows}</tbody>
            </table>
        </section>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::shows;
    use barrage_core::{Dam, Production, State, Target};
    use leptos::tachys::view::RenderHtml;

    fn names() -> Vec<String> {
        ["Ann", "Bob", "Cy", "Di"].iter().map(|s| s.to_string()).collect()
    }

    fn render(app: App, state: &State, seat: usize) -> String {
        let mut connected = vec![true; state.seats()];
        if let Some(c) = connected.get_mut(2) {
            *c = false;
        }
        game_screen(app, state.view_for(seat), names(), connected, "WXYZ".into()).to_html()
    }

    fn started(players: usize, seed: u64) -> State {
        let mut s = State::new(players, seed);
        for seat in 0..players {
            s.apply(seat, Action::Choose { pair: seat, contract: seat }).unwrap();
        }
        s
    }

    #[test]
    fn the_first_seat_picks_a_company_and_the_others_wait() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            app.games.barrage.ui.set_drawn(true);
            let s = State::new(3, 5);
            let html = render(app, &s, 0);
            assert!(html.contains("class=\"map\"") && html.contains("WXYZ"));
            assert!(html.contains("You pick your company"));
            assert_eq!(html.matches("class=\"pick\"").count(), 6, "three pairs, three contracts");
            let (company, officer, _) = s.setup.as_ref().unwrap().pairs[0];
            assert!(shows(&html, company.name()) && shows(&html, officer.ability()));
            assert!(html.contains("Take them") && html.contains("disabled"));
            assert!(!html.contains("Management board"), "not before the game starts");
            // Twelve basins and every space.
            assert_eq!(html.matches("class=\"basin-box\"").count(), 12);
            assert_eq!(html.matches("dam-space").count(), 20);
            assert_eq!(html.matches("conduit-space").count(), 20);
            assert_eq!(html.matches("ph-space").count(), 23);
            assert_eq!(html.matches("class=\"piece dam\"").count(), 3, "the neutral dams");
            let other = render(app, &s, 1);
            assert!(other.contains("Ann picks a company") && other.contains("Waiting for Ann."));

            // Picked: the button lights up.
            app.games.barrage.ui.pair.set(Some(1));
            app.games.barrage.ui.contract.set(Some(0));
            let html = render(app, &s, 0);
            assert_eq!(html.matches("class=\"pick on\"").count(), 2);
        });
    }

    #[test]
    fn the_printed_board_shows_the_pieces_as_made() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let mut s = started(2, 9);
            let seat = s.actor().unwrap();
            let company = s.players[seat].company.unwrap();
            let slug = company.name().to_lowercase();
            s.dams[1] = Some(Dam { owner: Some(seat), level: 2, water: 3 });
            s.conduits[0] = Some(seat);
            s.powerhouses[6] = Some(seat);
            let html = render(app, &s, seat);
            // The printed board by default, with the switch.
            assert!(html.contains("board-art") && html.contains("/barrage-art/board.webp"));
            assert!(html.contains("Printed board") && html.contains("Drawn map"));
            assert!(!html.contains("class=\"basin-box\""), "no drawn basins over the picture");
            // The pieces as the box has them: a Base and an Elevation, the
            // Conduit, the Powerhouse, the drops, the neutral Dams.
            for kind in ["base", "elevation", "conduit", "powerhouse"] {
                assert!(html.contains(&format!("/barrage-art/pieces/{slug}-{kind}.webp")), "{kind}");
            }
            assert!(html.contains("/barrage-art/pieces/neutral-base.webp"));
            assert!(html.matches("/barrage-art/pieces/drop.webp").count() >= 3);
            // Every space can still be clicked where the board prints it.
            assert_eq!(html.matches("dam-space").count(), 20);
            assert_eq!(html.matches("conduit-space").count(), 20);
            assert_eq!(html.matches("ph-space").count(), 23);
            // The stock with the tokens' pictures.
            for token in ["credit", "excavator", "mixer"] {
                assert!(html.contains(&format!("/barrage-art/pieces/{token}.webp")), "{token}");
            }
            // The drawn map, by choice.
            app.games.barrage.ui.set_drawn(true);
            let drawn = render(app, &s, seat);
            assert!(drawn.contains("class=\"basin-box\"") && !drawn.contains("board-art"));
            assert!(!drawn.contains(&format!("/barrage-art/pieces/{slug}-base.webp")));
        });
    }

    #[test]
    fn the_management_board_offers_its_open_spaces() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let s = started(2, 6);
            let seat = s.actor().unwrap();
            let html = render(app, &s, seat);
            assert!(html.contains("Management board") && html.contains("Turbine Station") && html.contains("Patent Office"));
            // Closed rows with two players.
            assert!(html.contains("space-btn closed"));
            assert!(html.contains("Your move") && html.contains("Build with a Technology tile"));
            assert!(html.contains("Send 1 Engineer for 1 Credit"));
            assert!(html.contains("Contract Office") && html.contains("National Contracts"));
            // Every company in turn order, with its wheel.
            assert_eq!(html.matches("class=\"wheel\"").count(), 2);
            assert!(html.contains("Round") && html.contains("of 5"));
            let other = render(app, &s, 1 - seat);
            assert!(!other.contains("space-btn open"));
        });
    }

    #[test]
    fn a_tile_in_hand_marks_where_it_builds() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let s = started(2, 7);
            let seat = s.actor().unwrap();
            assert!(!render(app, &s, seat).contains("class=\"target\""));
            app.games.barrage.ui.tile.set(Some((Some(Tile::Basic(Kind::Base)), None)));
            let html = render(app, &s, seat);
            let free = s.dams.iter().filter(|d| d.is_none()).count();
            assert_eq!(html.matches("class=\"target\"").count(), free, "every free dam space");
            assert!(html.contains("tool on") && html.contains("Click a marked space"));
        });
    }

    #[test]
    fn a_turbine_space_lists_the_productions() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let mut s = started(2, 8);
            let seat = s.actor().unwrap();
            s.dams = vec![None; 20];
            s.dams[1] = Some(Dam { owner: None, level: 2, water: 2 });
            s.conduits[0] = Some(seat);
            s.powerhouses[6] = Some(seat);
            app.games.barrage.ui.spot.set(Some(Spot::new(Section::Turbine, 0, false)));
            let html = render(app, &s, seat);
            assert!(html.contains("1 drop \u{2192} 7 energy") && html.contains("2 drops \u{2192} 12 energy"), "{html}");
            // Production, then the Contract it can fulfill.
            let production = Production { powerhouse: 6, conduit: 0, dam: 1, drops: 2 };
            s.apply(seat, Action::Produce { space: Some(Spot::new(Section::Turbine, 0, false)), production }).unwrap();
            if !s.todo.is_empty() {
                let html = render(app, &s, seat);
                assert!(html.contains("You produced 12 energy") && html.contains("Fulfill"));
            }
        });
    }

    #[test]
    fn decisions_get_their_own_controls() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let mut s = started(2, 9);
            let seat = s.actor().unwrap();
            s.todo = vec![barrage_core::Todo { seat, task: barrage_core::Task::Reward(barrage_core::Reward::Machinery(2)) }];
            let html = render(app, &s, seat);
            assert!(html.contains("2 Excavators") && html.contains("1 Excavator + 1 Mixer") && html.contains("2 Concrete Mixers"));
            assert!(!html.contains(">Skip<"));
            s.todo = vec![barrage_core::Todo { seat, task: barrage_core::Task::Reward(barrage_core::Reward::WaterNow(2)) }];
            let html = render(app, &s, seat);
            assert!(html.contains("Click the Headstreams") && html.contains("headstream open"));
            s.todo = vec![barrage_core::Todo { seat, task: barrage_core::Task::Reward(barrage_core::Reward::Conduit(2)) }];
            let html = render(app, &s, seat);
            assert!(html.contains("class=\"target\"") && html.contains(">Skip<"));
            let other = render(app, &s, 1 - seat);
            assert!(other.contains("Waiting for") && other.contains("build Conduit of value 2 or less"), "{other}");
            let _ = Target::Base(0);
        });
    }

    #[test]
    fn whole_games_render_for_every_seat_to_the_end() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            for players in 2..=4 {
                let mut s = State::new(players, players as u64 + 40);
                let mut steps = 0;
                while s.phase != Phase::Over {
                    let seat = s.actor().unwrap();
                    if steps % 7 == 0 {
                        for viewer in 0..players {
                            let html = render(app, &s, viewer);
                            assert!(html.contains("class=\"map\""));
                        }
                    }
                    let v = s.view_for(seat);
                    let m = &v.moves;
                    let action = if m.choose {
                        let setup = v.setup.as_ref().unwrap();
                        let pair = setup.pairs.iter().position(|p| p.2.is_none()).unwrap();
                        let contract = setup.contracts.iter().position(|c| c.1.is_none()).unwrap();
                        Action::Choose { pair, contract }
                    } else if let Some(n) = m.machinery {
                        Action::Machinery { excavators: n }
                    } else if m.water.is_some() {
                        Action::Water { space: None, drops: [1, 0, 0, 0] }
                    } else if let Some(&target) = m.places.first() {
                        Action::Place { target }
                    } else if let Some(&c) = m.fulfill.first() {
                        Action::Fulfill { contract: Some(c) }
                    } else if m.discard {
                        Action::Discard { contract: v.players[seat].contracts[0] }
                    } else if v.todo.is_some() {
                        Action::Skip
                    } else if let Some(o) = m.builds.first() {
                        Action::Build(o.build)
                    } else if m.pass {
                        Action::Pass
                    } else {
                        Action::Bank { engineers: m.bank.min(4) }
                    };
                    s.apply(seat, action).unwrap();
                    steps += 1;
                }
                for viewer in 0..players {
                    let html = render(app, &s, viewer);
                    assert!(html.contains("Final scores"), "{html}");
                    assert!(!html.contains("Your move"));
                }
            }
        });
    }
}
