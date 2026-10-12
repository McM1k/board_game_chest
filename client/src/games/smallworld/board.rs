//! The Small World game screen: the map, what you can do now, the column
//! of races and powers, the players and the log.
//!
//! Everything is drawn from the `View` the server sent for our seat; every
//! click asks the server for a move (`GameMove::SmallWorld`), whose answer
//! is the next state, which rebuilds the screen. What a click means is
//! decided in `look`, so it can be tested natively.

use std::sync::Arc;

use leptos::prelude::*;
use protocol::{ClientMsg, GameMove};
use smallworld_core::{Action, Army, Board, Offer, Phase, Power, Race, View};

use super::look::{self, Tool};
use super::map::map_svg;
use super::Ui;
use crate::app::{leave, send, App};

/// What every part of the board needs. Cheap to clone: the state is shared.
#[derive(Clone)]
struct Ctx {
    app: App,
    view: Arc<View>,
    board: Arc<Board>,
    names: Arc<Vec<String>>,
    connected: Arc<Vec<bool>>,
    /// The tool a click on the map uses now.
    tool: Option<Tool>,
}

impl Ctx {
    fn ui(&self) -> Ui {
        self.app.games.smallworld.ui
    }

    fn act(&self, action: Action) {
        send(self.app, &ClientMsg::Move(GameMove::SmallWorld(action)));
    }

    fn who(&self, seat: usize) -> String {
        look::who(seat, &self.names, self.view.you)
    }

    /// Our move now.
    fn acting(&self) -> bool {
        self.view.actor == Some(self.view.you)
    }
}

fn plural(n: u32) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// The running game, redrawn whenever the server sends a new state (or the
/// tool in hand changes).
pub fn board(app: App) -> AnyView {
    let small_world = app.games.smallworld;
    (move || {
        let view = small_world.view.get()?;
        Some(game_screen(app, view, small_world.names.get(), small_world.connected.get(), app.room.get()))
    })
    .into_any()
}

/// The whole game screen for one state the server sent.
pub fn game_screen(app: App, view: View, names: Vec<String>, connected: Vec<bool>, room: String) -> impl IntoView {
    let ui = app.games.smallworld.ui;
    let tool = look::effective_tool(&view, ui.tool.get());
    let pending = ui.gamble.get();
    let drawn = ui.drawn.get();
    let board = Arc::new(view.map());
    let ctx = Ctx { app, view: Arc::new(view), board, names: Arc::new(names), connected: Arc::new(connected), tool };
    let over = ctx.view.phase == Phase::Over;

    view! {
        <div class="game-board">
            {top_bar(app, room)}
            {status_band(&ctx)}
            <div class="board-grid">
                <div class="map-col">
                    <div class="map-frame">{map_svg(app, &ctx.view, &ctx.names, tool, pending, drawn)}</div>
                    {map_style(&ctx, drawn)}
                    {legend(&ctx, drawn)}
                </div>
                <aside class="side-col">
                    {over.then(|| results_panel(&ctx))}
                    {(!over).then(|| action_panel(&ctx))}
                    {races_panel(&ctx)}
                    {(!over).then(|| column_panel(&ctx))}
                    {log_panel(&ctx)}
                </aside>
            </div>
        </div>
    }
}

fn top_bar(app: App, room: String) -> AnyView {
    view! {
        <header class="top-bar">
            <h1 class="brand">"Small World"</h1>
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
    view! {
        <section class="status" aria-live="polite">
            <div class="turn" aria-label=format!("Turn {} of {}", v.turn, v.turns)>
                <span class="turn-word">"Turn"</span>
                <span class="turn-num">{v.turn}</span>
                <span class="turn-of">{format!("of {}", v.turns)}</span>
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

/// The key to the map's markers, drawn as they are on the map: only those
/// this game can show. Over the printed board, what the board prints itself
/// is shown as printed, since our signs for it aren't on the map then.
fn legend(ctx: &Ctx, drawn: bool) -> AnyView {
    use super::map::*;
    let o = ctx.view.options;
    let mut items: Vec<(&'static str, &'static str, &'static str)> = vec![
        ("mine", MINE, "Mine"),
        ("magic", MAGIC, "Magic source"),
        ("cavern", CAVERN, "Cavern"),
        ("lair", LAIR, "Troll's Lair"),
        ("fort", FORT, "Fortress"),
        ("camp", CAMP, "Encampment"),
        ("hole", HOLE, "Hole-in-the-Ground"),
        ("hero", HERO, "Hero"),
        ("dragon", DRAGON, "Dragon"),
    ];
    if o.be_not_afraid {
        items.extend([("pot", POT, "Pot of Gold"), ("catapult", CATAPULT, "Catapult")]);
    }
    if o.royal_bonus {
        items.push(("behemoth", BEHEMOTH, "Behemoth"));
    }
    if o.spiders_web {
        items.extend([("winter", WINTER, "Winter"), ("loot", LOOT, "Loot"), ("lava", LAVA, "Lava")]);
    }
    if o.sky.is_some() {
        items.extend([
            ("beanstalk", BEANSTALK, "Beanstalk"),
            ("stairway", STAIRWAY, "Stairway"),
            ("captive", CAPTIVE, "Scavengers' captive"),
            ("cannon", CANNON, "Cannon"),
            ("burning", FIRE, "Burning"),
        ]);
    }
    let list = items
        .into_iter()
        .map(|(class, glyph, label)| {
            let sign = if !drawn && look::SYMBOLS.contains(&class) {
                view! { <img class="key" src=look::symbol_art(class) alt="" /> }.into_any()
            } else {
                view! { <svg class="key" viewBox="-12 -12 24 24" aria-hidden="true">{marker(0, 0, class, glyph)}</svg> }.into_any()
            };
            view! { <li>{sign}{label}</li> }
        })
        .collect_view();
    // The drawn map doesn't mark where the Lost Tribes start.
    let tribe_start = (!drawn).then(|| {
        view! {
            <li>
                <img class="key" src=look::symbol_art("tribe-start") alt="" />
                "Where a Lost Tribe starts"
            </li>
        }
    });
    view! {
        <ul class="legend" aria-label="Map key">
            {list}
            {tribe_start}
            <li>
                <svg class="key" viewBox="-25 -25 50 50" aria-hidden="true">{tribe_token(0, 0)}</svg>
                "Lost Tribe"
            </li>
            <li>
                <svg class="key" viewBox="-13 -13 26 26" aria-hidden="true">
                    <g class="token declined"><circle class="disc" r="11" fill="#8c57cf" /></g>
                </svg>
                "In decline"
            </li>
            <li>"Numbers: the regions, as the log names them"</li>
        </ul>
    }
    .into_any()
}

// ----- what you can do now ------------------------------------------------------------

fn button(ctx: &Ctx, label: impl Into<String>, class: &'static str, action: Action) -> AnyView {
    let ctx = ctx.clone();
    let label = label.into();
    view! { <button class=class on:click=move |_| ctx.act(action)>{label}</button> }.into_any()
}

fn tool_buttons(ctx: &Ctx) -> AnyView {
    let tools = look::tools(&ctx.view);
    if tools.len() < 2 {
        return ().into_any();
    }
    let ui = ctx.ui();
    let chosen = ctx.tool;
    let buttons = tools
        .into_iter()
        .map(|tool| {
            let class = if chosen == Some(tool) { "tool on" } else { "tool" };
            view! {
                <button
                    class=class
                    title=look::tool_hint(tool)
                    aria-pressed=(chosen == Some(tool)).to_string()
                    on:click=move |_| ui.tool.set(tool)
                >
                    {look::tool_label(tool)}
                </button>
            }
        })
        .collect_view();
    view! { <div class="tools" role="group" aria-label="What a click on the map does">{buttons}</div> }.into_any()
}

/// The roll asked for by a click on a region marked "?".
fn gamble_box(ctx: &Ctx) -> Option<AnyView> {
    let region = ctx.ui().gamble.get_untracked()?;
    let (_, cost) = *ctx.view.moves.gamble().iter().find(|(r, _)| *r == region)?;
    let hand = look::hand(&ctx.view);
    let whose = if ctx.view.phase == (Phase::Conquest { ghouls: true }) { "the Ghouls'" } else { "your" };
    let ui = ctx.ui();
    let go = ctx.clone();
    Some(
        view! {
            <div class="gamble" role="dialog" aria-label="Roll the reinforcement die">
                <p>
                    {format!(
                        "{} costs {cost} and you have {hand}: roll the reinforcement die (0, 0, 0, 1, 2 or 3) for the rest? Win or lose, {whose} conquests end.",
                        look::region_name(&ctx.board, region),
                    )}
                </p>
                <div class="row">
                    <button class="primary" on:click=move |_| {
                        go.ui().gamble.set(None);
                        go.act(Action::Gamble { region });
                    }>"Roll the die"</button>
                    <button on:click=move |_| ui.gamble.set(None)>"Cancel"</button>
                </div>
            </div>
        }
        .into_any(),
    )
}

/// The pieces the active race holds besides its tokens.
fn held_pieces(army: &Army) -> String {
    let mut parts = Vec::new();
    for (n, what) in [
        (army.fireballs, "Fireball"),
        (army.winter, "Winter marker"),
        (army.behemoths, "Behemoth"),
        (army.resting, "sitting out"),
    ] {
        if n > 0 {
            if what == "sitting out" {
                parts.push(format!("{n} {what}"));
            } else {
                parts.push(format!("{n} {what}{}", plural(n as u32)));
            }
        }
    }
    parts.join(", ")
}

/// Copycat: which power to copy until the next turn.
fn copy_picker(ctx: &Ctx) -> Option<AnyView> {
    let v = &ctx.view;
    if v.moves.copy.is_empty() {
        return None;
    }
    let choices = v
        .moves
        .copy
        .iter()
        .map(|&i| {
            let power = v.offers[i].power;
            let c = ctx.clone();
            view! {
                <button class="tool" title=power.blurb() on:click=move |_| c.act(Action::Copy { offer: Some(i) })>
                    {power.name()}
                </button>
            }
        })
        .collect_view();
    Some(
        view! {
            <div class="copy">
                <p class="caption">"Copy a power from the column until your next turn:"</p>
                <div class="tools">{choices}</div>
            </div>
        }
        .into_any(),
    )
}

/// Haggling: hand Trade Pacts out, or take them back.
fn pact_picker(ctx: &Ctx) -> Option<AnyView> {
    let v = &ctx.view;
    let m = &v.moves;
    if m.give_pact.is_empty() && m.take_pact.is_empty() {
        return None;
    }
    let rows = (0..v.players.len())
        .filter(|&s| s != v.you)
        .map(|seat| {
            let held = v.players[seat].pacts;
            let (give, take) = (m.give_pact.contains(&seat), m.take_pact.contains(&seat));
            let (g, t) = (ctx.clone(), ctx.clone());
            view! {
                <li>
                    <span>{format!("{}: {held}", ctx.who(seat))}</span>
                    <button class="tool" disabled=!take on:click=move |_| t.act(Action::Pact { seat, give: false })>"\u{2212}"</button>
                    <button class="tool" disabled=!give on:click=move |_| g.act(Action::Pact { seat, give: true })>"+"</button>
                </li>
            }
        })
        .collect_view();
    Some(
        view! {
            <div class="pacts">
                <p class="caption">"Trade Pacts (whoever holds them pays you a coin each if they take one of your regions on their turn, or is paid one each if they don't):"</p>
                <ul>{rows}</ul>
            </div>
        }
        .into_any(),
    )
}

fn action_panel(ctx: &Ctx) -> AnyView {
    let v = &ctx.view;
    if !ctx.acting() {
        let waiting = v.actor.map(|s| look::name(s, &ctx.names)).unwrap_or_default();
        return view! {
            <section class="panel actions">
                <h2>"Your move"</h2>
                <p class="hint">{format!("Waiting for {waiting}.")}</p>
            </section>
        }
        .into_any();
    }
    let m = &v.moves;
    let hand = look::hand(v);
    let me = v.players[v.you].active;
    let in_hand = {
        let pieces = me.map(|a| held_pieces(&a)).unwrap_or_default();
        let tokens = format!("{hand} token{} in hand", plural(hand as u32));
        if pieces.is_empty() {
            tokens
        } else {
            format!("{tokens} \u{b7} {pieces}")
        }
    };
    let marauding = me.is_some_and(|a| a.has(Power::Marauding)) && !v.this_turn.marauded;
    let body = match v.phase {
        Phase::Pick => view! { <p>"Pick a race and power from the column below."</p> }.into_any(),
        Phase::Conquest { ghouls } => {
            let done = match me {
                Some(a) if ghouls => format!("Done with the Ghouls, on to the {}", a.race.name()),
                _ if ghouls => "Done with the Ghouls".to_string(),
                _ if marauding => "Done: conquer again (Marauding)".to_string(),
                _ => "Done conquering".to_string(),
            };
            view! {
                <p class="in-hand">{in_hand}</p>
                {copy_picker(ctx)}
                {tool_buttons(ctx)}
                {gamble_box(ctx)}
                {pact_picker(ctx)}
                <div class="row">
                    {m.roll.then(|| button(ctx, "Roll the die (Berserk)", "", Action::Roll))}
                    {v.this_turn.berserk.map(|d| view! { <span class="die-face" title="Berserk's die">{format!("Die: {d}")}</span> })}
                    {m.decline.then(|| button(ctx, "Go into decline", "danger", Action::Decline))}
                    {button(ctx, done, "primary", Action::EndConquests)}
                </div>
            }
            .into_any()
        }
        Phase::Redeploy => {
            let allies = (!m.allies.is_empty()).then(|| peace_picker(ctx));
            let end = match &m.end_turn {
                Some(why) => view! {
                    <button class="primary" disabled=true title=why.clone()>"End turn"</button>
                }
                .into_any(),
                None => button(ctx, "End turn", "primary", Action::EndTurn),
            };
            view! {
                <p class="in-hand">{in_hand}</p>
                {tool_buttons(ctx)}
                {allies}
                {pact_picker(ctx)}
                {v.coins_now.map(|c| view! { <p class="hint">{format!("Ending now scores {c} coin{}.", plural(c))}</p> })}
                <div class="row">
                    {m.decline.then(|| button(ctx, "End turn and go into decline", "danger", Action::Decline))}
                    {end}
                </div>
            }
            .into_any()
        }
        Phase::Reinforce => view! {
            <p class="in-hand">{format!("{hand} token{} to put back", plural(hand as u32))}</p>
            <p class="hint">"Click your regions to put them back."</p>
        }
        .into_any(),
        Phase::Over => ().into_any(),
    };
    view! {
        <section class="panel actions you-act">
            <h2>"Your move"</h2>
            {body}
        </section>
    }
    .into_any()
}

/// Diplomat: whom to make peace with.
fn peace_picker(ctx: &Ctx) -> AnyView {
    let v = &ctx.view;
    let current = v.players[v.you].active.and_then(|a| a.ally);
    let choices = std::iter::once(None)
        .chain(v.moves.allies.iter().map(|&s| Some(s)))
        .map(|seat| {
            let label = seat.map_or("Nobody".to_string(), |s| ctx.who(s));
            let class = if seat == current { "tool on" } else { "tool" };
            let c = ctx.clone();
            view! { <button class=class on:click=move |_| c.act(Action::Ally { seat })>{label}</button> }
        })
        .collect_view();
    view! {
        <div class="peace">
            <p class="caption">"Make peace with (they can't attack you until your next turn):"</p>
            <div class="tools">{choices}</div>
        </div>
    }
    .into_any()
}

// ----- races and players ----------------------------------------------------------------

/// A race and its power as the box puts them together: the power's badge
/// slotted against the left end of the race's banner (no badge: a race in
/// decline lost its power).
fn combo_art(race: Race, power: Option<Power>) -> AnyView {
    view! {
        <span class="combo" aria-hidden="true">
            {power.map(|p| view! { <img class="combo-power" src=look::power_art(p) alt="" /> })}
            <img class="combo-race" src=look::banner_art(race) alt="" />
        </span>
    }
    .into_any()
}

/// What a race and its power do, shown while hovering (or focusing) the
/// combo.
fn combo_details(race: Race, power: Option<Power>, copied: Option<Power>) -> AnyView {
    view! {
        <div class="combo-details" role="tooltip">
            <p><b>{race.name()}</b>": "{race.blurb()}</p>
            {power.map(|p| view! { <p><b>{p.name()}</b>": "{p.blurb()}</p> })}
            {copied.map(|c| view! { <p><b>{c.name()}</b>" (copied): "{c.blurb()}</p> })}
        </div>
    }
    .into_any()
}

fn army_line(army: &Army, declined: bool) -> AnyView {
    let name = if declined {
        match army.power {
            Power::Spirit => format!("{} (Spirit)", army.race.name()),
            _ => army.race.name().to_string(),
        }
    } else {
        look::combo_name(army.race, army.power)
    };
    let copied = (!declined).then_some(army.copied).flatten();
    let copying = copied.map(|p| format!(" \u{b7} copying {}", p.name()));
    let extra = match (declined, army.hand, army.resting) {
        (true, 0, _) => String::new(),
        (true, h, _) => format!(" \u{b7} {h} in hand"),
        (false, h, 0) => format!(" \u{b7} {h} in hand"),
        (false, h, r) => format!(" \u{b7} {h} in hand, {r} sitting out"),
    };
    // In decline the power is lost, unless it is Spirit.
    let power = (!declined || army.power == Power::Spirit).then_some(army.power);
    view! {
        <li class=if declined { "army declined" } else { "army" } tabindex="0">
            {combo_art(army.race, power)}
            <span class="army-text">
                <span class="army-name">{name}</span>
                {(!declined).then(|| view! { <span class="tag">"active"</span> })}
                {declined.then(|| view! { <span class="tag tag-off">"in decline"</span> })}
                <span class="army-extra">{copying}{extra}</span>
            </span>
            {combo_details(army.race, power, copied)}
        </li>
    }
    .into_any()
}

fn races_panel(ctx: &Ctx) -> AnyView {
    let v = &ctx.view;
    let rows = v
        .players
        .iter()
        .enumerate()
        .map(|(seat, p)| {
            let connected = ctx.connected.get(seat).copied().unwrap_or(true);
            let coins = match p.coins {
                Some(c) => format!("{c} coin{}", plural(c)),
                None => "? coins".to_string(),
            };
            let armies = p
                .active
                .iter()
                .map(|a| army_line(a, false))
                .chain(p.declined.iter().map(|a| army_line(a, true)))
                .collect_view();
            let notes = {
                let mut n = vec![format!("{} token{} on the map", p.on_map, plural(p.on_map))];
                if p.pacts > 0 {
                    n.push(format!("{} Trade Pact{}", p.pacts, plural(p.pacts as u32)));
                }
                if p.free_pick {
                    n.push("next race free".to_string());
                }
                n.join(" \u{b7} ")
            };
            view! {
                <li class=if seat == v.current { "player current" } else { "player" }>
                    <div class="player-head">
                        <span class="swatch" style=format!("background:{}", look::seat_color(seat))></span>
                        <span class="player-name">{ctx.who(seat)}</span>
                        {(seat == v.current && v.phase != Phase::Over).then(|| view! { <span class="tag">"turn"</span> })}
                        {(!connected).then(|| view! { <span class="tag tag-off">"offline"</span> })}
                        <span class="coins" title="Victory coins: everybody else's stay hidden until the end">{coins}</span>
                    </div>
                    <ul class="armies">{armies}</ul>
                    <p class="note">{notes}</p>
                </li>
            }
        })
        .collect_view();
    view! {
        <section class="panel players-panel">
            <h2>"Players"</h2>
            <ul class="players">{rows}</ul>
        </section>
    }
    .into_any()
}

// ----- the column of races and powers ------------------------------------------------------

fn offer_row(ctx: &Ctx, i: usize, o: &Offer) -> AnyView {
    let pick = ctx.view.moves.pick.iter().find(|(p, _)| *p == i).copied();
    let can = ctx.acting() && pick.is_some();
    let price = pick.map_or_else(|| look::pick_cost(&ctx.view, i), |(_, c)| c);
    let cost = if price == 0 { "Free".to_string() } else { format!("{price} coin{}", plural(price)) };
    let tokens = o.race.tokens() + o.power.tokens() + o.tokens;
    // The picture prints the name and the tokens; screen readers get them here.
    let label = format!("{}, {tokens} tokens", look::combo_name(o.race, o.power));
    // What lies on the combo isn't printed: the coins, and the Homunculi's
    // extra tokens.
    let mut on_it = Vec::new();
    if o.coins > 0 {
        on_it.push(format!("+{} on it", o.coins));
    }
    if o.tokens > 0 {
        on_it.push(format!("+{} token{}", o.tokens, plural(o.tokens as u32)));
    }
    let c = ctx.clone();
    view! {
        <li class="offer" tabindex="0">
            <span class="sr">{label}</span>
            {combo_art(o.race, Some(o.power))}
            <div class="offer-foot">
                {(!on_it.is_empty()).then(|| view! { <span class="on-it">{on_it.join(" \u{b7} ")}</span> })}
                {if can {
                    view! { <button class="primary" on:click=move |_| c.act(Action::Pick { offer: i })>{format!("Take ({cost})")}</button> }.into_any()
                } else {
                    view! { <span class="offer-cost">{cost}</span> }.into_any()
                }}
            </div>
            {combo_details(o.race, Some(o.power), None)}
        </li>
    }
    .into_any()
}

fn column_panel(ctx: &Ctx) -> AnyView {
    let v = &ctx.view;
    let rows = v.offers.iter().enumerate().map(|(i, o)| offer_row(ctx, i, o)).collect_view();
    let picking = v.phase == Phase::Pick && ctx.acting();
    view! {
        <section class=if picking { "panel column-panel picking" } else { "panel column-panel" }>
            <h2>"Races and powers"</h2>
            <ol class="column">{rows}</ol>
            <p class="note">{format!("{} more races in the stack.", v.races_left)}</p>
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
        .take(14)
        .map(|event| view! { <li>{look::describe_event(event, &ctx.names, v.you, &ctx.board)}</li> })
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
        .map(|(seat, coins, tokens)| {
            let won = v.winners.contains(&seat);
            view! {
                <tr class=if won { "won" } else { "" }>
                    <td>{ctx.who(seat)}{won.then_some(" \u{2605}")}</td>
                    <td class="num">{coins}</td>
                    <td class="num">{tokens}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <section class="panel results">
            <h2>"Final coins"</h2>
            <table>
                <thead><tr><th>"Player"</th><th class="num">"Coins"</th><th class="num" title="The tie-break">"Tokens"</th></tr></thead>
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
    use leptos::tachys::view::RenderHtml;
    use smallworld_core::{Options, Race, RegionMove, RegionState, State, Tokens};

    fn names() -> Vec<String> {
        ["Ann", "Bob", "Cy", "Di", "Ed", "Flo"].iter().map(|s| s.to_string()).collect()
    }

    fn render(app: App, state: &State, seat: usize) -> String {
        let mut connected = vec![true; state.seats()];
        if let Some(c) = connected.get_mut(2) {
            *c = false;
        }
        game_screen(app, state.view_for(seat), names(), connected, "WXYZ".into()).to_html()
    }

    #[test]
    fn the_first_player_picks_from_the_column_and_the_others_wait() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let s = State::new(3, 5);
            let html = render(app, &s, 0);
            // The printed board by default, its picture under the regions.
            assert!(html.contains("class=\"map printed\"") && html.contains("WXYZ"));
            assert!(html.contains(&format!("href=\"{}\"", look::board_art(3))), "{html}");
            assert!(html.contains("Printed board") && html.contains("Drawn map"));
            assert!(!html.contains("marker mine"), "the picture shows the mines, and the key their printed sign");
            app.games.smallworld.ui.drawn.set(true);
            let drawn = render(app, &s, 0);
            assert!(drawn.contains("class=\"map\"") && !drawn.contains("board-art"));
            assert!(drawn.matches("marker mine").count() > 1);
            app.games.smallworld.ui.drawn.set(false);
            assert!(html.contains("You pick a race"));
            assert_eq!(html.matches("class=\"offer\"").count(), 6);
            assert_eq!(html.matches("Take (").count(), 6, "five coins: every combo is affordable");
            assert!(html.contains("Take (Free)"));
            // Each combo is assembled as in the box: the power's badge, then
            // the race's banner. Their texts are there for hovering.
            assert_eq!(html.matches("class=\"combo-power\"").count(), 6);
            assert_eq!(html.matches("class=\"combo-race\"").count(), 6);
            let top = s.offers[0];
            assert!(html.contains(&format!("src=\"{}\"", look::power_art(top.power))), "{html}");
            assert!(html.contains(&format!("src=\"{}\"", look::banner_art(top.race))));
            assert_eq!(html.matches("class=\"combo-details\"").count(), 6);
            assert!(shows(&html, top.race.blurb()) && shows(&html, top.power.blurb()));
            // Every region of the 3-player board is drawn.
            assert_eq!(html.matches("<polygon").count(), 30);
            assert!(html.contains("Turn") && html.contains("of 10"));
            // Coins: your own only.
            assert!(html.contains("5 coins") && html.contains("? coins"));
            assert!(html.contains("offline"));

            let other = render(app, &s, 1);
            assert!(other.contains("Ann picks a race") && other.contains("Waiting for Ann."));
            assert!(!other.contains("Take ("));
        });
    }

    #[test]
    fn each_combo_in_the_column_is_its_picture_with_the_price_under_it() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let mut s = State::new(3, 5);
            // Skipped twice: two coins and, for the Homunculi, two tokens on it.
            s.offers[1] = Offer { race: Race::Homunculi, power: Power::Flying, coins: 2, tokens: 2 };
            let html = render(app, &s, 0);
            let top = s.offers[0];
            // The picture already prints the name and the tokens: no text
            // beside it, only a name for screen readers.
            assert!(!html.contains("offer-name") && !html.contains("offer-tokens") && !html.contains("offer-body"));
            assert!(!html.contains(&format!("{} + {} = ", top.race.tokens(), top.power.tokens())), "{html}");
            let total = top.race.tokens() + top.power.tokens();
            let label = format!("<span class=\"sr\">{}, {total} tokens</span>", look::combo_name(top.race, top.power));
            assert!(shows(&html, &label), "{html}");
            // The price and the Take button sit under the picture.
            let offers: Vec<&str> = html.split("class=\"offer\">").skip(1).collect();
            assert_eq!(offers.len(), 6, "{html}");
            for offer in &offers {
                let art = offer.find("class=\"combo-race\"").unwrap();
                let foot = offer.find("class=\"offer-foot\"").unwrap();
                let take = offer.find("Take (").unwrap();
                assert!(art < foot && foot < take, "{offer}");
            }
            // The Homunculi's extra tokens aren't printed on the picture.
            let homunculi = offers[1];
            assert!(homunculi.contains("+2 on it") && homunculi.contains("+2 tokens"), "{homunculi}");
            let total = Race::Homunculi.tokens() + Power::Flying.tokens() + 2;
            assert!(shows(homunculi, &format!("Flying Homunculi, {total} tokens")), "{homunculi}");
        });
    }

    #[test]
    fn the_sky_islands_are_drawn_above_the_board() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let s = State::with_options(6, 5, Options::all());
            let html = render(app, &s, 3);
            assert_eq!(html.matches("<polygon").count(), 47 + 9);
            // Printed: both boards' pictures; drawn: the sky band, and the
            // Beanstalk's and the Stairway's markers.
            assert!(html.contains(&format!("href=\"{}\"", look::sky_art(s.board.sky.unwrap()))), "{html}");
            assert!(html.contains(&format!("href=\"{}\"", look::board_art(5))));
            app.games.smallworld.ui.drawn.set(true);
            let html = render(app, &s, 3);
            assert!(html.contains("Sky Islands") && html.contains("class=\"sky-band\""));
            assert!(html.contains("marker beanstalk") && html.contains("marker stairway"));
            assert!(html.contains("Pot of Gold") && shows(&html, "Scavengers' captive"), "the key lists the new pieces");
        });
    }

    #[test]
    fn over_the_printed_board_the_key_shows_its_printed_symbols() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let key = |s: &State| {
                let html = render(app, s, 0);
                html.split("class=\"legend\"").nth(1).unwrap().split("</ul>").next().unwrap().to_string()
            };
            let printed_sign = |symbol: &str| format!("<img src=\"{}\"", look::symbol_art(symbol));
            let s = State::new(3, 5);
            let printed = key(&s);
            // What the board prints, as printed: no sign of ours for it.
            for symbol in ["mine", "magic", "cavern", "tribe-start"] {
                assert!(printed.contains(&printed_sign(symbol)), "{symbol}: {printed}");
            }
            for ours in ["marker mine", "marker magic", "marker cavern"] {
                assert!(!printed.contains(ours), "{ours}: {printed}");
            }
            assert!(shows(&printed, "Where a Lost Tribe starts"));
            // What we draw on it keeps our signs, the Lost Tribes' token too.
            assert!(printed.contains("marker lair") && printed.contains("marker hero"), "{printed}");
            assert!(printed.contains(&format!("<image href=\"{}\"", look::tribe_art())));
            assert!(!printed.contains("beanstalk"), "no Sky Islands");
            // The drawn map's key is unchanged.
            app.games.smallworld.ui.drawn.set(true);
            let drawn = key(&s);
            assert!(drawn.contains("marker mine") && drawn.contains("marker magic") && drawn.contains("marker cavern"));
            assert!(!drawn.contains("/symbols/") && !shows(&drawn, "Where a Lost Tribe starts"), "{drawn}");
            // The Sky Islands print the Beanstalk and the Stairway.
            let sky = State::with_options(4, 5, Options::all());
            let drawn = key(&sky);
            assert!(drawn.contains("marker beanstalk") && drawn.contains("marker stairway"), "{drawn}");
            app.games.smallworld.ui.drawn.set(false);
            let printed = key(&sky);
            for symbol in ["beanstalk", "stairway"] {
                assert!(printed.contains(&printed_sign(symbol)), "{symbol}: {printed}");
                assert!(!printed.contains(&format!("marker {symbol}")), "{symbol}: {printed}");
            }
            assert!(printed.contains("marker captive") && printed.contains("marker pot"), "{printed}");
        });
    }

    #[test]
    fn conquering_marks_the_regions_with_their_cost() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let mut s = State::new(3, 5);
            s.apply(0, Action::Pick { offer: 0 }).unwrap();
            let html = render(app, &s, 0);
            let targets = s.moves(0).conquer().len();
            assert_eq!(html.matches("target").count(), targets, "{html}");
            assert!(html.contains("Done conquering"));
            assert!(!html.contains("Go into decline"), "nothing to decline yet");
            let r = s.moves(0).conquer()[0].0;
            s.apply(0, Action::Conquer { region: r }).unwrap();
            let html = render(app, &s, 0);
            assert!(html.contains("class=\"token\""));
            // The token is the race's portrait; the player lists it too.
            let race = s.players[0].active.unwrap().race;
            assert!(html.contains(&format!("<image href=\"{}\"", look::race_art(race))), "{html}");
            // The player's race is assembled too, its texts on hover.
            assert_eq!(html.matches("class=\"combo-details\"").count(), 6 + 1, "{html}");
            assert!(!html.contains("class=\"powers\""));
            // Now the regions out of reach show their price, faded.
            let m = s.moves(0);
            let far = |&&(r, _): &&(usize, u8)| !m.can(RegionMove::Conquer, r) && !m.can(RegionMove::Gamble, r);
            let priced = m.prices.iter().filter(far).count();
            assert!(priced > 0);
            assert_eq!(html.matches("badge price").count(), priced);
            assert!(html.contains("faded: out of reach for now"));
            // The Abandon tool is offered only for regions held before this turn.
            assert!(!html.contains(">Abandon<"));
            s.apply(0, Action::EndConquests).unwrap();
            let html = render(app, &s, 0);
            assert!(html.contains("redeploying") && html.contains("End turn"));
            assert!(html.contains("disabled"), "tokens still in hand: {html}");
        });
    }

    #[test]
    fn tokens_are_big_enough_to_make_out_and_the_lost_tribes_have_their_picture() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let mut s = State::new(5, 5);
            let tribes = s.regions.iter().filter(|r| r.lost_tribe).count();
            assert!(tribes > 0);
            let html = render(app, &s, 0);
            // Each Lost Tribe is its printed token's picture, and so is the key's.
            let art = format!("<image href=\"{}\"", look::tribe_art());
            assert_eq!(html.matches(&art).count(), tribes + 1, "{html}");
            assert_eq!(html.matches("class=\"tribe\"").count(), tribes + 1);
            for tribe in html.split("class=\"tribe\"").skip(1) {
                let tribe = tribe.split("</g>").next().unwrap();
                assert!(tribe.contains("class=\"disc\"") && tribe.contains(&art) && !tribe.contains("<rect"), "{tribe}");
            }
            // A race token's disc, on the map (the key's are smaller).
            s.apply(0, Action::Pick { offer: 0 }).unwrap();
            let r = s.moves(0).conquer()[0].0;
            s.apply(0, Action::Conquer { region: r }).unwrap();
            let html = render(app, &s, 0);
            let map = html.split("class=\"legend\"").next().unwrap();
            let token = map.split("class=\"token\"").nth(1).expect("a token");
            let radius: f32 = token.split("r=\"").nth(1).and_then(|t| t.split('"').next()).unwrap().parse().unwrap();
            assert!(radius >= 22.0, "{radius}");
        });
    }

    #[test]
    fn a_region_short_of_tokens_asks_before_rolling_the_die() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let mut s = State::new(2, 5);
            s.apply(0, Action::Pick { offer: 0 }).unwrap();
            s.players[0].active.as_mut().unwrap().hand = 1;
            let html = render(app, &s, 0);
            assert!(html.contains("badge die") && !html.contains("Roll the die"));
            let (region, _) = s.moves(0).gamble()[0];
            app.games.smallworld.ui.gamble.set(Some(region));
            let html = render(app, &s, 0);
            assert!(html.contains("Roll the die") && html.contains("Cancel"), "{html}");
            assert!(html.contains("region") && html.contains("pending"));
        });
    }

    #[test]
    fn the_next_turn_offers_the_decline() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let mut s = State::new(2, 5);
            for seat in [0, 1] {
                s.apply(seat, Action::Pick { offer: 0 }).unwrap();
                let region = s.moves(seat).conquer()[0].0;
                s.apply(seat, Action::Conquer { region }).unwrap();
                s.apply(seat, Action::EndConquests).unwrap();
                while !s.moves(seat).can_end_turn {
                    let m = s.moves(seat);
                    let take = m.list(RegionMove::Take);
                    let action = if m.end_turn.as_deref().is_some_and(|w| w.contains("Amazons")) && !take.is_empty() {
                        Action::Take { region: take[0] }
                    } else {
                        Action::Put { region: m.list(RegionMove::Put)[0] }
                    };
                    s.apply(seat, action).unwrap();
                }
                s.apply(seat, Action::EndTurn).unwrap();
                while s.phase == Phase::Reinforce {
                    let seat = s.actor().unwrap();
                    s.apply(seat, Action::Put { region: s.moves(seat).list(RegionMove::Put)[0] }).unwrap();
                }
            }
            assert_eq!((s.turn, s.current), (2, 0));
            let html = render(app, &s, 0);
            assert!(html.contains("Go into decline") && html.contains("Done conquering"));
            assert!(html.contains("put your race in decline"));
        });
    }

    #[test]
    fn the_ghouls_in_decline_say_which_race_conquers_after_them() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let mut s = State::new(2, 5);
            // Ann's Ghouls went into decline with the Spirit power.
            s.offers.retain(|o| o.race != Race::Ghouls && o.power != Power::Spirit);
            s.races.retain(|&r| r != Race::Ghouls);
            s.players[0].declined.push(Army::new(Race::Ghouls, Power::Spirit, 0));
            let ghouls = Tokens { race: Race::Ghouls, count: 4, declined: true };
            s.regions[7] = RegionState { tokens: Some(ghouls), ..RegionState::default() };
            let next = s.offers[0];
            s.apply(0, Action::Pick { offer: 0 }).unwrap();
            assert_eq!(s.phase, Phase::Conquest { ghouls: true });
            let html = render(app, &s, 0);
            let hand = s.players[0].active.unwrap().hand;
            let then = format!("then your {}, with {hand}.", look::combo_name(next.race, next.power));
            assert!(shows(&html, &then), "{html}");
            assert!(shows(&html, &format!("Done with the Ghouls, on to the {}", next.race.name())), "{html}");

            // Short of tokens: the die only ends the Ghouls' conquests.
            s.players[0].declined[0].hand = 1;
            let (region, _) = s.moves(0).gamble()[0];
            app.games.smallworld.ui.gamble.set(Some(region));
            let html = render(app, &s, 0);
            assert!(shows(&html, "it ends the Ghouls' conquests"), "{html}");
            assert!(shows(&html, "Win or lose, the Ghouls' conquests end."), "{html}");
            app.games.smallworld.ui.gamble.set(None);

            s.apply(0, Action::EndConquests).unwrap();
            let html = render(app, &s, 0);
            assert!(html.contains("You are conquering") && html.contains("Done conquering"), "{html}");
            assert!(!html.contains("Done with the Ghouls"));
        });
    }

    #[test]
    fn copycats_and_hagglers_get_their_controls() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let mut s = State::with_options(3, 5, Options { sky: None, ..Options::all() });
            s.offers[0].power = Power::Copycat;
            s.offers[1].power = Power::Commando;
            s.apply(0, Action::Pick { offer: 0 }).unwrap();
            let html = render(app, &s, 0);
            assert!(html.contains("Copy a power from the column"), "{html}");
            let mut s = State::with_options(3, 5, Options { sky: None, ..Options::all() });
            s.offers[0].power = Power::Haggling;
            s.apply(0, Action::Pick { offer: 0 }).unwrap();
            let html = render(app, &s, 0);
            assert!(html.contains("Trade Pacts (whoever holds them"), "{html}");
        });
    }

    #[test]
    fn whole_games_render_for_every_seat_to_the_end() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            for (players, options) in [(3, Options::default()), (4, Options::all())] {
                let mut s = State::with_options(players, 11, options);
                let mut steps = 0;
                while s.phase != Phase::Over {
                    let seat = s.actor().unwrap();
                    if steps % 9 == 0 {
                        for viewer in 0..players {
                            let html = render(app, &s, viewer);
                            assert!(html.contains("class=\"map printed\""));
                        }
                    }
                    let m = s.moves(seat);
                    let first = |kind: RegionMove| m.list(kind).first().copied();
                    let why = m.end_turn.as_deref().unwrap_or("");
                    let action = if let Some(&(offer, _)) = m.pick.first() {
                        Action::Pick { offer }
                    } else if let Some(region) = first(RegionMove::Conquer) {
                        Action::Conquer { region }
                    } else if m.end_conquests {
                        Action::EndConquests
                    } else if why.contains("Heroes") {
                        Action::Hero { region: *m.list(RegionMove::Hero).iter().find(|&&r| !s.regions[r].hero).unwrap() }
                    } else if why.contains("Cannons") {
                        Action::Cannon {
                            region: *m.list(RegionMove::Cannon).iter().find(|&&r| s.regions[r].cannon.is_none()).unwrap(),
                        }
                    } else if why.contains("Amazons") && s.players[seat].active.is_some_and(|a| a.hand < 4) {
                        Action::Take { region: first(RegionMove::Take).unwrap() }
                    } else if !m.can_end_turn && first(RegionMove::Put).is_some() {
                        Action::Put { region: first(RegionMove::Put).unwrap() }
                    } else {
                        Action::EndTurn
                    };
                    s.apply(seat, action).unwrap();
                    steps += 1;
                    assert!(steps < 40_000);
                }
                let html = render(app, &s, 2);
                assert!(html.contains("Final coins") && html.contains("The game is over"));
                assert!(!html.contains("? coins"));
            }
        });
    }
}
