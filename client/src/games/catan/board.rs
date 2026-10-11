//! The game screen: the island in the middle, the players, your hand, the
//! actions, trades and the log around it.

use super::map::{board_svg, robber_victims, PLAYER_COLORS};
use super::{Mode, Picker};
use crate::app::{leave, send, App};
use catan_core::{Action, DevCard, GameView, Hand, Phase, Resource, SetupExpect};
use leptos::prelude::*;
use protocol::{ClientMsg, GameMove};

fn act(app: App, a: Action) {
    send(app, &ClientMsg::Move(GameMove::Catan(a)));
}

fn sum(h: [u8; 5]) -> u32 {
    h.iter().map(|&n| n as u32).sum()
}

fn status_text(v: &GameView) -> String {
    let cur = v.players[v.current].name.clone();
    let mine = v.seat == Some(v.current);
    match &v.phase {
        Phase::Setup { expect, .. } => {
            let what = match expect {
                SetupExpect::Settlement => "settlement",
                SetupExpect::Road => "road",
            };
            if mine {
                format!("Your turn: place a {what}.")
            } else {
                format!("{cur} is placing a {what}…")
            }
        }
        Phase::Roll => {
            if mine {
                "Your turn — roll the dice.".into()
            } else {
                format!("{cur} is about to roll.")
            }
        }
        Phase::Main => {
            if mine {
                "Build, trade, or end your turn.".into()
            } else {
                format!("{cur} is taking their turn.")
            }
        }
        Phase::Discard { pending } => {
            let names: Vec<String> = pending.iter().map(|(s, _)| v.players[*s].name.clone()).collect();
            format!("A 7! Waiting for {} to discard.", names.join(", "))
        }
        Phase::MoveRobber { .. } => {
            if mine {
                "Move the robber: click a tile.".into()
            } else {
                format!("{cur} is moving the robber.")
            }
        }
        Phase::RoadBuilding { left } => {
            if mine {
                format!("Road Building: place {left} more road(s).")
            } else {
                format!("{cur} is building roads.")
            }
        }
        Phase::Finished { winner } => format!("{} wins the game!", v.players[*winner].name),
    }
}

/// The running game, redrawn whenever the server sends a new state.
pub fn board(app: App) -> AnyView {
    let catan = app.games.catan;
    (move || {
        let view = catan.view.get()?;
        Some(game_screen(app, view, catan.connected.get(), app.room.get()))
    })
    .into_any()
}

pub fn game_screen(app: App, v: GameView, connected: Vec<bool>, room: String) -> impl IntoView {
    let board = board_svg(app, &v);
    let status = status_text(&v);
    let dice = v
        .dice
        .map(|(a, b)| format!("🎲 {a} + {b} = {}", a + b))
        .unwrap_or_default();
    let my_vp = v.me.as_ref().map(|m| m.vp).unwrap_or(0);
    let modal_view = v.clone();

    view! {
        <div class="game">
            <header class="topbar">
                <div class="brand">"Catan"</div>
                <div class="room">"Room " <b>{room}</b></div>
                <div class="status">{status}</div>
                <div class="dice">{dice}</div>
                <div class="myvp">"Your points: " <b>{my_vp}</b> " / " {v.rules.victory_points}</div>
                <button class="link" on:click=move |_| leave(app)>"Leave"</button>
            </header>
            <div class="layout">
                <div class="board-wrap">{board}</div>
                <aside class="sidebar">
                    {players_panel(&v, &connected)}
                    {hand_panel(app, &v)}
                    {actions_panel(app, &v)}
                    {trade_panel(app, &v)}
                    {log_panel(&v)}
                </aside>
            </div>
            {move || modals(app, &modal_view)}
        </div>
    }
}

fn players_panel(v: &GameView, connected: &[bool]) -> impl IntoView {
    let rows = v
        .players
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_cur = i == v.current && !matches!(v.phase, Phase::Finished { .. });
            let you = v.seat == Some(i);
            let online = connected.get(i).copied().unwrap_or(true);
            view! {
                <div class="player" class:current=is_cur>
                    <span class="swatch" style=format!("background:{}", PLAYER_COLORS[i % 4]) />
                    <div class="pname">
                        {if is_cur { "▶ " } else { "" }}
                        {p.name.clone()}
                        {you.then(|| view! { <span class="badge">"you"</span> })}
                        {(!online).then(|| view! { <span class="badge off">"offline"</span> })}
                        {(v.longest_road == Some(i)).then(|| view! { <span class="badge gold">"Longest Road"</span> })}
                        {(v.largest_army == Some(i)).then(|| view! { <span class="badge gold">"Largest Army"</span> })}
                    </div>
                    <div class="pstats">
                        <span title="Victory points (not counting hidden cards)">{format!("{} VP", p.public_vp)}</span>
                        <span title="Resource cards">{format!("🂠 {}", p.hand_count)}</span>
                        <span title="Development cards">{format!("★ {}", p.dev_count)}</span>
                        <span title="Knights played">{format!("⚔ {}", p.knights)}</span>
                        <span title="Longest road">{format!("🛣 {}", p.road_length)}</span>
                    </div>
                </div>
            }
        })
        .collect_view();
    view! { <section class="panel"><h3>"Players"</h3>{rows}</section> }
}

fn hand_panel(app: App, v: &GameView) -> AnyView {
    let Some(me) = &v.me else { return ().into_any() };

    let chips = (0..5)
        .map(|i| {
            view! {
                <div class=format!("hand-card res-{i}")>
                    <div class="n">{me.hand.0[i]}</div>
                    <div class="l">{Resource::ALL[i].name()}</div>
                </div>
            }
        })
        .collect_view();

    let can_play_now = !v.dev_played && v.seat == Some(v.current);
    let in_main = matches!(v.phase, Phase::Main);
    let in_roll = matches!(v.phase, Phase::Roll);

    let kinds = [
        (DevCard::Knight, "Knight"),
        (DevCard::RoadBuilding, "Road Building"),
        (DevCard::YearOfPlenty, "Year of Plenty"),
        (DevCard::Monopoly, "Monopoly"),
        (DevCard::VictoryPoint, "Victory Point"),
    ];
    let dev_rows = kinds
        .iter()
        .filter_map(|&(kind, label)| {
            let playable = me.dev.iter().filter(|c| **c == kind).count();
            let fresh = me.new_dev.iter().filter(|c| **c == kind).count();
            if playable + fresh == 0 {
                return None;
            }
            let can_play = can_play_now
                && playable > 0
                && match kind {
                    DevCard::Knight => in_main || in_roll,
                    DevCard::VictoryPoint => false,
                    _ => in_main,
                };
            let on_play = move |_| match kind {
                DevCard::Knight => act(app, Action::PlayKnight),
                DevCard::RoadBuilding => act(app, Action::PlayRoadBuilding),
                DevCard::YearOfPlenty => {
                    app.games.catan.ui.picks.set(vec![]);
                    app.games.catan.ui.picker.set(Some(Picker::YearOfPlenty));
                }
                DevCard::Monopoly => app.games.catan.ui.picker.set(Some(Picker::Monopoly)),
                DevCard::VictoryPoint => {}
            };
            Some(view! {
                <div class="dev-row">
                    <span>{format!("{label} ×{playable}")}
                        {(fresh > 0).then(|| view! { <span class="muted">{format!(" (+{fresh} new)")}</span> })}
                    </span>
                    {(kind != DevCard::VictoryPoint).then(|| view! {
                        <button class="small" disabled=!can_play on:click=on_play>"Play"</button>
                    })}
                </div>
            })
        })
        .collect_view();

    view! {
        <section class="panel">
            <h3>"Your hand"</h3>
            <div class="hand">{chips}</div>
            <div class="devs">{dev_rows}</div>
        </section>
    }
    .into_any()
}

fn actions_panel(app: App, v: &GameView) -> AnyView {
    let (Some(me), true) = (&v.me, v.seat == Some(v.current)) else {
        return ().into_any();
    };
    match &v.phase {
        Phase::Roll => view! {
            <section class="panel">
                <button class="primary big" on:click=move |_| act(app, Action::Roll)>"Roll dice"</button>
            </section>
        }
        .into_any(),
        Phase::Main => {
            let p = &v.players[v.current];
            let has = |items: &[(Resource, u8)]| me.hand.contains(&Hand::of(items));
            let road_ok = has(&[(Resource::Wood, 1), (Resource::Brick, 1)]) && p.roads_left > 0 && !v.legal.roads.is_empty();
            let settle_ok = has(&[
                (Resource::Wood, 1),
                (Resource::Brick, 1),
                (Resource::Sheep, 1),
                (Resource::Wheat, 1),
            ]) && p.settlements_left > 0
                && !v.legal.settlements.is_empty();
            let city_ok = has(&[(Resource::Wheat, 2), (Resource::Ore, 3)])
                && p.cities_left > 0
                && !v.legal.cities.is_empty();
            let dev_ok = has(&[(Resource::Sheep, 1), (Resource::Wheat, 1), (Resource::Ore, 1)]) && v.deck_left > 0;

            let toggle = move |m: Mode| {
                app.games.catan.ui.mode.update(|cur| *cur = if *cur == m { Mode::None } else { m });
            };
            view! {
                <section class="panel">
                    <h3>"Build"</h3>
                    <div class="btn-row">
                        <button disabled=!road_ok title="1 Wood + 1 Brick"
                            class:active=move || app.games.catan.ui.mode.get() == Mode::Road
                            on:click=move |_| toggle(Mode::Road)>"Road"</button>
                        <button disabled=!settle_ok title="1 Wood + 1 Brick + 1 Sheep + 1 Wheat"
                            class:active=move || app.games.catan.ui.mode.get() == Mode::Settlement
                            on:click=move |_| toggle(Mode::Settlement)>"Settlement"</button>
                        <button disabled=!city_ok title="2 Wheat + 3 Ore"
                            class:active=move || app.games.catan.ui.mode.get() == Mode::City
                            on:click=move |_| toggle(Mode::City)>"City"</button>
                        <button disabled=!dev_ok title="1 Sheep + 1 Wheat + 1 Ore"
                            on:click=move |_| act(app, Action::BuyDevCard)>
                            {format!("Dev card ({})", v.deck_left)}
                        </button>
                    </div>
                    <p class="muted small-text">"Costs: road = wood+brick · settlement = wood+brick+sheep+wheat · city = 2 wheat+3 ore · dev card = sheep+wheat+ore"</p>
                    <button class="primary" on:click=move |_| {
                        app.games.catan.ui.mode.set(Mode::None);
                        act(app, Action::EndTurn);
                    }>"End turn"</button>
                </section>
            }
            .into_any()
        }
        _ => ().into_any(),
    }
}

fn trade_panel(app: App, v: &GameView) -> AnyView {
    let (Some(me), Phase::Main) = (&v.me, &v.phase) else {
        return ().into_any();
    };
    let seat = v.seat.unwrap();
    let my_turn = seat == v.current;

    if let Some(t) = &v.trade {
        let summary = format!(
            "{} offers {} for {}",
            v.players[t.from].name,
            describe(&t.give),
            describe(&t.want)
        );
        if t.from == seat {
            let others = v
                .players
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != seat)
                .map(|(i, p)| {
                    let accepted = t.accepted.contains(&i);
                    let declined = t.declined.contains(&i);
                    view! {
                        <div class="trade-reply">
                            <span>{p.name.clone()}</span>
                            {if accepted {
                                view! { <button class="small primary" on:click=move |_| act(app, Action::ConfirmTrade { with: i })>"Trade!"</button> }.into_any()
                            } else if declined {
                                view! { <span class="muted">"declined"</span> }.into_any()
                            } else {
                                view! { <span class="muted">"thinking…"</span> }.into_any()
                            }}
                        </div>
                    }
                })
                .collect_view();
            return view! {
                <section class="panel">
                    <h3>"Your offer"</h3>
                    <p>{summary}</p>
                    {others}
                    <button class="small" on:click=move |_| act(app, Action::CancelTrade)>"Cancel offer"</button>
                </section>
            }
            .into_any();
        }
        let affordable = me.hand.contains(&t.want);
        let accepted = t.accepted.contains(&seat);
        let declined = t.declined.contains(&seat);
        return view! {
            <section class="panel">
                <h3>"Trade offer"</h3>
                <p>{summary}</p>
                <div class="btn-row">
                    <button class="primary" class:active=accepted disabled=!affordable
                        on:click=move |_| act(app, Action::RespondTrade { accept: true })>"Accept"</button>
                    <button class:active=declined
                        on:click=move |_| act(app, Action::RespondTrade { accept: false })>"Decline"</button>
                </div>
                {(!affordable).then(|| view! { <p class="muted">"You don't have the requested cards."</p> })}
            </section>
        }
        .into_any();
    }

    if !my_turn {
        return ().into_any();
    }

    let hand_max = me.hand.0;
    let ratios = me.ratios;
    let give_row = (0..5)
        .map(|i| {
            let ok = me.hand.0[i] >= ratios[i];
            view! {
                <button class=format!("res-btn res-{i}") disabled=!ok
                    class:active=move || app.games.catan.ui.bank_give.get() == Some(i)
                    on:click=move |_| app.games.catan.ui.bank_give.set(Some(i))>
                    {format!("{} {}:1", Resource::ALL[i].name(), ratios[i])}
                </button>
            }
        })
        .collect_view();
    let get_row = (0..5)
        .map(|i| {
            view! {
                <button class=format!("res-btn res-{i}")
                    class:active=move || app.games.catan.ui.bank_get.get() == Some(i)
                    on:click=move |_| app.games.catan.ui.bank_get.set(Some(i))>
                    {Resource::ALL[i].name()}
                </button>
            }
        })
        .collect_view();

    view! {
        <section class="panel">
            <h3>"Trade with the bank"</h3>
            <div class="label">"Give"</div><div class="btn-row wrap">{give_row}</div>
            <div class="label">"Get"</div><div class="btn-row wrap">{get_row}</div>
            <button disabled=move || {
                    let (g, r) = (app.games.catan.ui.bank_give.get(), app.games.catan.ui.bank_get.get());
                    g.is_none() || r.is_none() || g == r
                }
                on:click=move |_| {
                    if let (Some(g), Some(r)) = (app.games.catan.ui.bank_give.get_untracked(), app.games.catan.ui.bank_get.get_untracked()) {
                        act(app, Action::BankTrade { give: Resource::ALL[g], get: Resource::ALL[r] });
                        app.games.catan.ui.bank_give.set(None);
                        app.games.catan.ui.bank_get.set(None);
                    }
                }>"Trade with bank"</button>
        </section>
        <section class="panel">
            <h3>"Offer a trade to players"</h3>
            <div class="two-col">
                <div><div class="label">"You give"</div>{hand_editor(app.games.catan.ui.give, hand_max)}</div>
                <div><div class="label">"You want"</div>{hand_editor(app.games.catan.ui.want, [9; 5])}</div>
            </div>
            <button disabled=move || {
                    let (g, w) = (app.games.catan.ui.give.get(), app.games.catan.ui.want.get());
                    sum(g) == 0 || sum(w) == 0 || (0..5).any(|i| g[i] > 0 && w[i] > 0)
                }
                on:click=move |_| {
                    let (g, w) = (app.games.catan.ui.give.get_untracked(), app.games.catan.ui.want.get_untracked());
                    act(app, Action::ProposeTrade { give: Hand(g), want: Hand(w) });
                    app.games.catan.ui.give.set([0; 5]);
                    app.games.catan.ui.want.set([0; 5]);
                }>"Offer trade"</button>
        </section>
    }
    .into_any()
}

fn describe(h: &Hand) -> String {
    let parts: Vec<String> = Resource::ALL
        .iter()
        .filter(|r| h.get(**r) > 0)
        .map(|r| format!("{} {}", h.get(*r), r.name()))
        .collect();
    parts.join(", ")
}

fn hand_editor(sig: RwSignal<[u8; 5]>, max: [u8; 5]) -> impl IntoView {
    (0..5)
        .map(|i| {
            view! {
                <div class="editor-row">
                    <span class=format!("chip res-{i}")>{Resource::ALL[i].name()}</span>
                    <button class="step" on:click=move |_| sig.update(|h| h[i] = h[i].saturating_sub(1))>"−"</button>
                    <span class="count">{move || sig.get()[i]}</span>
                    <button class="step" disabled={move || sig.get()[i] >= max[i]}
                        on:click=move |_| sig.update(|h| h[i] += 1)>"+"</button>
                </div>
            }
        })
        .collect_view()
}

fn log_panel(v: &GameView) -> impl IntoView {
    let lines = v
        .log
        .iter()
        .rev()
        .take(14)
        .map(|l| view! { <li>{l.clone()}</li> })
        .collect_view();
    view! { <section class="panel"><h3>"Log"</h3><ul class="log">{lines}</ul></section> }
}

fn modals(app: App, v: &GameView) -> AnyView {
    let Some(me) = &v.me else { return ().into_any() };
    let seat = v.seat.unwrap();

    // Winner.
    if let Phase::Finished { winner } = &v.phase {
        let name = v.players[*winner].name.clone();
        let you = *winner == seat;
        return view! {
            <div class="modal-bg"><div class="modal card">
                <h2>{if you { "🏆 You win!".to_string() } else { format!("🏆 {name} wins!") }}</h2>
                <button class="primary" on:click=move |_| leave(app)>"Back to start"</button>
            </div></div>
        }
        .into_any();
    }

    // Discard after a 7.
    if let Phase::Discard { pending } = &v.phase {
        if let Some(&(_, need)) = pending.iter().find(|(s, _)| *s == seat) {
            let need = need as u32;
            let max = me.hand.0;
            return view! {
                <div class="modal-bg"><div class="modal card">
                    <h2>"A 7 was rolled"</h2>
                    <p>{format!("You hold more than {} cards. Discard {need}.", v.rules.discard_above)}</p>
                    {hand_editor(app.games.catan.ui.discard, max)}
                    <button class="primary"
                        disabled=move || sum(app.games.catan.ui.discard.get()) != need
                        on:click=move |_| {
                            act(app, Action::Discard(Hand(app.games.catan.ui.discard.get_untracked())));
                            app.games.catan.ui.discard.set([0; 5]);
                        }>
                        {move || format!("Discard ({}/{need})", sum(app.games.catan.ui.discard.get()))}
                    </button>
                </div></div>
            }
            .into_any();
        }
    }

    // Choose whom to steal from.
    if let (Phase::MoveRobber { .. }, Some(tile), true) =
        (&v.phase, app.games.catan.ui.robber_tile.get(), v.seat == Some(v.current))
    {
        let buttons = robber_victims(v, tile)
            .into_iter()
            .map(|p| {
                let name = v.players[p].name.clone();
                view! {
                    <button on:click=move |_| {
                        act(app, Action::MoveRobber { tile, victim: Some(p) });
                        app.games.catan.ui.robber_tile.set(None);
                    }>
                        <span class="swatch" style=format!("background:{}", PLAYER_COLORS[p % 4]) />
                        {name}
                    </button>
                }
            })
            .collect_view();
        return view! {
            <div class="modal-bg"><div class="modal card">
                <h2>"Steal from…"</h2>
                <div class="btn-row wrap">{buttons}</div>
                <button class="link" on:click=move |_| app.games.catan.ui.robber_tile.set(None)>"Cancel"</button>
            </div></div>
        }
        .into_any();
    }

    // Year of Plenty / Monopoly picker.
    if let Some(kind) = app.games.catan.ui.picker.get() {
        let title = match kind {
            Picker::YearOfPlenty => "Year of Plenty: pick 2 resources from the bank",
            Picker::Monopoly => "Monopoly: pick a resource to take from everyone",
        };
        let buttons = (0..5)
            .map(|i| {
                view! {
                    <button class=format!("res-btn res-{i}") on:click=move |_| match kind {
                        Picker::Monopoly => {
                            act(app, Action::PlayMonopoly(Resource::ALL[i]));
                            app.games.catan.ui.picker.set(None);
                        }
                        Picker::YearOfPlenty => {
                            let mut picks = app.games.catan.ui.picks.get_untracked();
                            picks.push(i);
                            if picks.len() == 2 {
                                act(app, Action::PlayYearOfPlenty(Resource::ALL[picks[0]], Resource::ALL[picks[1]]));
                                app.games.catan.ui.picker.set(None);
                                app.games.catan.ui.picks.set(vec![]);
                            } else {
                                app.games.catan.ui.picks.set(picks);
                            }
                        }
                    }>{Resource::ALL[i].name()}</button>
                }
            })
            .collect_view();
        return view! {
            <div class="modal-bg"><div class="modal card">
                <h2>{title}</h2>
                <div class="btn-row wrap">{buttons}</div>
                <button class="link" on:click=move |_| { app.games.catan.ui.picker.set(None); app.games.catan.ui.picks.set(vec![]); }>"Cancel"</button>
            </div></div>
        }
        .into_any();
    }

    ().into_any()
}

/// Native render smoke tests: build real game states with the rules and
/// render the game screen to HTML, catching panics (bad indexing, unwraps)
/// that the type checker can't see.
#[cfg(test)]
mod tests {
    use super::super::look::{now_ms, Fresh, FreshKind};
    use super::*;
    use catan_core::{Game, Rules};
    use leptos::tachys::view::RenderHtml;
    use rand::{rngs::StdRng, SeedableRng};

    fn game_after_setup(n: usize) -> (Game, StdRng) {
        let mut rng = StdRng::seed_from_u64(42);
        let names = (0..n).map(|i| format!("P{i}")).collect();
        let mut g = Game::new(Rules::default(), names, &mut rng);
        while let Phase::Setup { expect, .. } = g.phase.clone() {
            let s = g.current;
            let a = match expect {
                SetupExpect::Settlement => Action::BuildSettlement { vertex: g.legal_settlements(s, true)[0] },
                SetupExpect::Road => Action::BuildRoad { edge: g.legal_roads(s, g.setup_vertex)[0] },
            };
            g.apply(s, a, &mut rng).unwrap();
        }
        (g, rng)
    }

    fn render(g: &Game, seat: Option<usize>) -> String {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let view = g.view_for(seat);
            let n = view.players.len();
            game_screen(app, view, vec![true; n], "ABCD".into()).to_html()
        })
    }

    #[test]
    fn renders_every_phase_for_every_seat() {
        let (mut g, mut rng) = game_after_setup(3);
        for seat in [Some(0), Some(1), Some(2), None] {
            let html = render(&g, seat);
            assert!(html.contains("Roll dice") == (seat == Some(0)));
        }
        g.do_roll(2, 3, &mut rng).unwrap();
        g.players[0].hand = Hand([4, 4, 4, 4, 4]);
        g.players[0].dev = vec![DevCard::Knight, DevCard::Monopoly];
        for seat in [Some(0), Some(1), None] {
            let html = render(&g, seat);
            assert!(html.contains("<svg"));
        }
        let html = render(&g, Some(0));
        assert!(html.contains("End turn"));
        assert!(html.contains("Trade with the bank"));
        assert!(html.contains("Knight"));

        // Robber, discard and open-trade states.
        g.players[1].hand = Hand([3, 3, 3, 0, 0]);
        g.phase = Phase::Roll;
        g.do_roll(3, 4, &mut rng).unwrap();
        for seat in [Some(0), Some(1), Some(2)] {
            let html = render(&g, seat);
            assert!(!html.is_empty());
        }
        assert!(render(&g, Some(1)).contains("A 7 was rolled"));
        g.phase = Phase::MoveRobber { back_to_roll: false };
        assert!(render(&g, Some(0)).contains("target"));
        g.phase = Phase::Main;
        g.apply(0, Action::ProposeTrade { give: Hand([1, 0, 0, 0, 0]), want: Hand([0, 0, 1, 0, 0]) }, &mut rng)
            .unwrap();
        assert!(render(&g, Some(1)).contains("Trade offer"));
        assert!(render(&g, Some(0)).contains("Your offer"));
        g.phase = Phase::Finished { winner: 2 };
        assert!(render(&g, Some(2)).contains("You win"));
    }

    #[test]
    fn board_shows_terrain_art_and_fresh_embers() {
        let (g, _) = game_after_setup(3);
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let view = g.view_for(Some(0));
            let n = view.players.len();
            let render = |app: App, view: GameView| game_screen(app, view, vec![true; n], "ABCD".into()).to_html();

            // Art for every terrain, drawn from gradients + a hex clip.
            let plain = render(app, view.clone());
            for marker in [
                "g-forest", "g-hills", "g-pasture", "g-fields", "g-mountains", "g-desert", "hexclip",
                "#13422a", "#6e2815", "#8a8a80", "#a8801e", "#4a5568", "#2f5e2c",
            ] {
                assert!(plain.contains(marker), "missing {marker}");
            }
            assert_eq!(plain.matches("class=\"ember\"").count(), 0);

            // Harbours: one coloured 2:1 token per resource, four neutral 3:1 tokens.
            for color in ["#2d6a4f", "#c1553b", "#7bc99b", "#e0b94f", "#8d99ae"] {
                assert!(plain.contains(&format!("fill=\"{color}\"")), "no harbour in {color}");
            }
            assert_eq!(plain.matches("fill=\"#fdf6e3\"").count(), 4);

            // One ember per fresh item; expired ones are not drawn.
            let road = view.roads.iter().position(|r| r.is_some()).unwrap();
            let piece = view.buildings.iter().position(|b| b.is_some()).unwrap();
            let now = now_ms();
            app.games.catan.fresh.set(vec![
                Fresh { kind: FreshKind::Road(road), at: now - 100.0 },
                Fresh { kind: FreshKind::Piece(piece), at: now - 500.0 },
                Fresh { kind: FreshKind::Robber(view.robber), at: now },
                Fresh { kind: FreshKind::Piece(piece), at: now - 5000.0 },
            ]);
            let lit = render(app, view);
            assert_eq!(lit.matches("class=\"ember\"").count(), 3);
            assert!(lit.contains("animation-delay:-"));
        });
    }

    #[test]
    fn the_board_follows_the_latest_state() {
        let (g, _) = game_after_setup(3);
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            assert!(!board(app).to_html().contains("topbar"), "nothing to draw before a state arrives");
            app.room.set("ABCD".into());
            app.games.catan.connected.set(vec![true; 3]);
            app.games.catan.view.set(Some(g.view_for(Some(1))));
            let html = board(app).to_html();
            assert!(html.contains("topbar") && html.contains("ABCD"));
        });
    }
}
