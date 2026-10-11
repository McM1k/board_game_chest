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
use smallworld_core::{Action, Army, Offer, Phase, View};

use super::look::{self, Tool};
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
    let ctx = Ctx { app, view: Arc::new(view), names: Arc::new(names), connected: Arc::new(connected), tool };
    let over = ctx.view.phase == Phase::Over;

    view! {
        <div class="game-board">
            {top_bar(app, room)}
            {status_band(&ctx)}
            <div class="board-grid">
                <div class="map-col">
                    <div class="map-frame">{map_svg(app, &ctx.view, &ctx.names, tool, pending)}</div>
                    {legend()}
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

/// The key to the map's markers, drawn as they are on the map.
fn legend() -> AnyView {
    use super::map::{marker, CAMP, CAVERN, DRAGON, FORT, HERO, HOLE, LAIR, MAGIC, MINE};
    let items = [
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
    let list = items
        .into_iter()
        .map(|(class, glyph, label)| {
            view! {
                <li>
                    <svg class="key" viewBox="-12 -12 24 24" aria-hidden="true">{marker(0, 0, class, glyph)}</svg>
                    {label}
                </li>
            }
        })
        .collect_view();
    view! {
        <ul class="legend" aria-label="Map key">
            {list}
            <li>
                <svg class="key wide" viewBox="-16 -13 32 26" aria-hidden="true">
                    <g class="tribe"><rect x="-15" y="-12" width="30" height="24" rx="7" /><path d=super::map::TRIBE /></g>
                </svg>
                "Lost Tribe"
            </li>
            <li>
                <svg class="key wide" viewBox="-25 -13 50 26" aria-hidden="true">
                    <g class="token declined"><rect x="-24" y="-12" width="48" height="24" rx="7" fill="#8c57cf" /></g>
                </svg>
                "In decline"
            </li>
            <li>"Numbers: the regions, as the log names them"</li>
        </ul>
    }
    .into_any()
}

// ----- what you can do now ------------------------------------------------------------

fn button(ctx: &Ctx, label: &'static str, class: &'static str, action: Action) -> AnyView {
    let ctx = ctx.clone();
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
                <button class=class title=tool.hint() aria-pressed=(chosen == Some(tool)).to_string() on:click=move |_| ui.tool.set(tool)>
                    {tool.label()}
                </button>
            }
        })
        .collect_view();
    view! { <div class="tools" role="group" aria-label="What a click on the map does">{buttons}</div> }.into_any()
}

/// The roll asked for by a click on a region marked "?".
fn gamble_box(ctx: &Ctx) -> Option<AnyView> {
    let region = ctx.ui().gamble.get_untracked()?;
    let (_, cost) = *ctx.view.moves.gamble.iter().find(|(r, _)| *r == region)?;
    let hand = look::hand(&ctx.view);
    let ui = ctx.ui();
    let go = ctx.clone();
    Some(
        view! {
            <div class="gamble" role="dialog" aria-label="Roll the reinforcement die">
                <p>
                    {format!(
                        "{} costs {cost} and you have {hand}: roll the reinforcement die (0, 0, 0, 1, 2 or 3) for the rest? Win or lose, your conquests end.",
                        look::region_name(ctx.view.board, region),
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
    let body = match v.phase {
        Phase::Pick => view! { <p>"Pick a race and power from the column below."</p> }.into_any(),
        Phase::Conquest { ghouls } => view! {
            <p class="in-hand">{format!("{hand} token{} in hand", if hand == 1 { "" } else { "s" })}</p>
            {tool_buttons(ctx)}
            {gamble_box(ctx)}
            <div class="row">
                {m.roll.then(|| button(ctx, "Roll the die (Berserk)", "", Action::Roll))}
                {v.this_turn.berserk.map(|d| view! { <span class="die-face" title="Berserk's die">{format!("Die: {d}")}</span> })}
                {m.decline.then(|| button(ctx, "Go into decline", "danger", Action::Decline))}
                {button(ctx, if ghouls { "Done with the Ghouls" } else { "Done conquering" }, "primary", Action::EndConquests)}
            </div>
        }
        .into_any(),
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
                <p class="in-hand">{format!("{hand} token{} in hand", if hand == 1 { "" } else { "s" })}</p>
                {tool_buttons(ctx)}
                {allies}
                {v.coins_now.map(|c| view! { <p class="hint">{format!("Ending now scores {c} coin{}.", if c == 1 { "" } else { "s" })}</p> })}
                <div class="row">
                    {m.decline.then(|| button(ctx, "End turn and go into decline", "danger", Action::Decline))}
                    {end}
                </div>
            }
            .into_any()
        }
        Phase::Reinforce => view! {
            <p class="in-hand">{format!("{hand} token{} to put back", if hand == 1 { "" } else { "s" })}</p>
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

fn army_line(army: &Army, declined: bool) -> AnyView {
    let name = if declined {
        if army.power == smallworld_core::Power::Spirit {
            format!("{} (Spirit)", army.race.name())
        } else {
            army.race.name().to_string()
        }
    } else {
        look::combo_name(army.race, army.power)
    };
    let extra = match (declined, army.hand, army.resting) {
        (true, 0, _) => String::new(),
        (true, h, _) => format!(" \u{b7} {h} in hand"),
        (false, h, 0) => format!(" \u{b7} {h} in hand"),
        (false, h, r) => format!(" \u{b7} {h} in hand, {r} resting"),
    };
    view! {
        <li class=if declined { "army declined" } else { "army" } title=if declined { String::new() } else { format!("{} {}", army.race.blurb(), army.power.blurb()) }>
            <span class="army-name">{name}</span>
            {(!declined).then(|| view! { <span class="tag">"active"</span> })}
            {declined.then(|| view! { <span class="tag tag-off">"in decline"</span> })}
            <span class="army-extra">{extra}</span>
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
                Some(c) => format!("{c} coin{}", if c == 1 { "" } else { "s" }),
                None => "? coins".to_string(),
            };
            let armies = p
                .active
                .iter()
                .map(|a| army_line(a, false))
                .chain(p.declined.iter().map(|a| army_line(a, true)))
                .collect_view();
            let mine = (seat == v.you).then(|| {
                p.active.map(|a| {
                    view! {
                        <div class="powers">
                            <p><b>{a.race.name()}</b>": "{a.race.blurb()}</p>
                            <p><b>{a.power.name()}</b>": "{a.power.blurb()}</p>
                        </div>
                    }
                })
            });
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
                    {mine}
                    <p class="note">{format!("{} token{} on the map", p.on_map, if p.on_map == 1 { "" } else { "s" })}</p>
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
    let can = ctx.acting() && ctx.view.moves.pick.contains(&i);
    let cost = if i == 0 { "Free".to_string() } else { format!("{i} coin{}", if i == 1 { "" } else { "s" }) };
    let tokens = o.race.tokens() + o.power.tokens();
    let c = ctx.clone();
    view! {
        <li class="offer">
            <div class="offer-head">
                <span class="offer-name">{look::combo_name(o.race, o.power)}</span>
                <span class="offer-tokens" title="Tokens: race + power">{format!("{} + {} = {tokens}", o.race.tokens(), o.power.tokens())}</span>
            </div>
            <p class="offer-text"><b>{o.race.name()}</b>": "{o.race.blurb()}</p>
            <p class="offer-text"><b>{o.power.name()}</b>": "{o.power.blurb()}</p>
            <div class="offer-foot">
                {(o.coins > 0).then(|| view! { <span class="on-it">{format!("+{} on it", o.coins)}</span> })}
                {if can {
                    view! { <button class="primary" on:click=move |_| c.act(Action::Pick { offer: i })>{format!("Take ({cost})")}</button> }.into_any()
                } else {
                    view! { <span class="offer-cost">{cost}</span> }.into_any()
                }}
            </div>
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
        .map(|event| view! { <li>{look::describe_event(event, &ctx.names, v.you, v.board)}</li> })
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
    use leptos::tachys::view::RenderHtml;
    use smallworld_core::State;

    fn names() -> Vec<String> {
        ["Ann", "Bob", "Cy"].iter().map(|s| s.to_string()).collect()
    }

    fn render(app: App, state: &State, seat: usize) -> String {
        game_screen(app, state.view_for(seat), names(), vec![true, true, false], "WXYZ".into()).to_html()
    }

    #[test]
    fn the_first_player_picks_from_the_column_and_the_others_wait() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let s = State::new(3, 5);
            let html = render(app, &s, 0);
            assert!(html.contains("class=\"map\"") && html.contains("WXYZ"));
            assert!(html.contains("You pick a race"));
            assert_eq!(html.matches("class=\"offer\"").count(), 6);
            assert_eq!(html.matches("Take (").count(), 6, "five coins: every combo is affordable");
            assert!(html.contains("Take (Free)"));
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
    fn conquering_marks_the_regions_with_their_cost() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let mut s = State::new(3, 5);
            s.apply(0, Action::Pick { offer: 0 }).unwrap();
            let html = render(app, &s, 0);
            let targets = s.moves(0).conquer.len();
            assert_eq!(html.matches("target").count(), targets, "{html}");
            assert!(html.contains("Done conquering"));
            assert!(!html.contains("Go into decline"), "nothing to decline yet");
            let r = s.moves(0).conquer[0].0;
            s.apply(0, Action::Conquer { region: r }).unwrap();
            let html = render(app, &s, 0);
            assert!(html.contains("class=\"token\""));
            // The Abandon tool is offered only for regions held before this turn.
            assert!(!html.contains(">Abandon<"));
            s.apply(0, Action::EndConquests).unwrap();
            let html = render(app, &s, 0);
            assert!(html.contains("redeploying") && html.contains("End turn"));
            assert!(html.contains("disabled"), "tokens still in hand: {html}");
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
            let (region, _) = s.moves(0).gamble[0];
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
                let region = s.moves(seat).conquer[0].0;
                s.apply(seat, Action::Conquer { region }).unwrap();
                s.apply(seat, Action::EndConquests).unwrap();
                while !s.moves(seat).can_end_turn {
                    let m = s.moves(seat);
                    let action = if m.end_turn.as_deref().is_some_and(|w| w.contains("Amazons")) && !m.take.is_empty() {
                        Action::Take { region: m.take[0] }
                    } else {
                        Action::Put { region: m.put[0] }
                    };
                    s.apply(seat, action).unwrap();
                }
                s.apply(seat, Action::EndTurn).unwrap();
                while s.phase == Phase::Reinforce {
                    let seat = s.actor().unwrap();
                    s.apply(seat, Action::Put { region: s.moves(seat).put[0] }).unwrap();
                }
            }
            assert_eq!((s.turn, s.current), (2, 0));
            let html = render(app, &s, 0);
            assert!(html.contains("Go into decline") && html.contains("Done conquering"));
            assert!(html.contains("put your race in decline"));
        });
    }

    #[test]
    fn a_whole_game_renders_for_every_seat_to_the_end() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let mut s = State::new(3, 11);
            let mut steps = 0;
            while s.phase != Phase::Over {
                let seat = s.actor().unwrap();
                if steps % 7 == 0 {
                    for viewer in 0..3 {
                        let html = render(app, &s, viewer);
                        assert!(html.contains("class=\"map\""));
                    }
                }
                let m = s.moves(seat);
                let action = if let Some(&offer) = m.pick.first() {
                    Action::Pick { offer }
                } else if let Some(&(region, _)) = m.conquer.first() {
                    Action::Conquer { region }
                } else if m.end_conquests {
                    Action::EndConquests
                } else if !m.can_end_turn && !m.put.is_empty() {
                    Action::Put { region: m.put[0] }
                } else if !m.can_end_turn && !m.take.is_empty() {
                    Action::Take { region: m.take[0] }
                } else if !m.can_end_turn && !m.hero.is_empty() {
                    Action::Hero { region: *m.hero.iter().find(|&&r| !s.regions[r].hero).unwrap() }
                } else {
                    Action::EndTurn
                };
                s.apply(seat, action).unwrap();
                steps += 1;
                assert!(steps < 20_000);
            }
            let html = render(app, &s, 2);
            assert!(html.contains("Final coins") && html.contains("The game is over"));
            assert!(!html.contains("? coins"));
        });
    }
}
