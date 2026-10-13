//! The board of It's a Wonderful World: the card table you draft, plan and
//! produce at, and the final standings.
//!
//! Everything here is drawn from the `View` the server sent for our seat, and
//! every click asks the server for a move (`GameMove::Wonderful`); the
//! server's answer is the next state, which rebuilds the screen. What a click
//! means is decided in `look`, so it can be tested natively.
//!
//! Cards are built from `span`s only (and their picture's `img`), so a whole
//! card can sit inside a `<button>` (the draft) as well as in a `<div>`.

use std::sync::Arc;

use leptos::prelude::*;
use protocol::{ClientMsg, GameMove};
use wonderful_core::{
    Action, Bonus, Building, Card, CardId, Cost, Kind, Per, Phase, PlayerView, Res, Token, View, DRAFT_PICKS, ROUNDS,
    WRAP_UP,
};

use super::look::{self, CubeFrom, Held, Slot};
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
        self.app.games.wonderful.ui
    }

    fn me(&self) -> &PlayerView {
        &self.view.players[self.view.you]
    }

    fn act(&self, action: Action) {
        send(self.app, &ClientMsg::Move(GameMove::Wonderful(action)));
    }

    /// A seat's name, "You" for ourselves.
    fn who(&self, seat: usize) -> String {
        look::who(seat, &self.names, self.view.you)
    }

    /// A seat's own name, ours included.
    fn name(&self, seat: usize) -> String {
        self.names.get(seat).cloned().unwrap_or_else(|| format!("Player {}", seat + 1))
    }
}

/// The running game, redrawn whenever the server sends a new state.
pub fn board(app: App) -> AnyView {
    let wonderful = app.games.wonderful;
    (move || {
        let view = wonderful.view.get()?;
        Some(game_screen(app, view, wonderful.names.get(), wonderful.connected.get(), app.room.get()))
    })
    .into_any()
}

/// The whole game screen for one state the server sent.
pub fn game_screen(app: App, view: View, names: Vec<String>, connected: Vec<bool>, room: String) -> impl IntoView {
    let ctx = Ctx { app, view: Arc::new(view), names: Arc::new(names), connected: Arc::new(connected) };
    let over = ctx.view.phase == Phase::Over;

    view! {
        <div class="game-board">
            {top_bar(app, room)}
            {status_band(&ctx)}
            <div class="board-grid">
                <div class="main-col">
                    {phase_panel(&ctx)}
                    {tray(&ctx)}
                    {(!over).then(|| buildings_panel(&ctx))}
                    {empire_panel(&ctx)}
                </div>
                <aside class="side-col">
                    {rivals_panel(&ctx)}
                    {log_panel(&ctx)}
                </aside>
            </div>
            {cube_picker(&ctx)}
        </div>
    }
}

// ----- small parts -------------------------------------------------------------------

/// A glossy token with a letter on it.
fn cube(class: &'static str, letter: &'static str) -> AnyView {
    view! { <span class=format!("cube {class}") aria-hidden="true">{letter}</span> }.into_any()
}

/// What a token or space shows: a resource's card-type symbol (Materials
/// wear the Structure's, and so on), a character token's picture, or else
/// its letter.
fn face(res: Option<Res>, token: Option<Token>, letter: &'static str) -> AnyView {
    match (res, token) {
        (Some(res), _) => kind_icon(look::res_kind(res)),
        (None, Some(token)) => token_face(token, letter),
        (None, None) => letter.into_any(),
    }
}

/// A General's or Financier's printed picture over its letter. A picture
/// that fails to load removes itself, which brings back the letter and the
/// coloured shape (the CSS keys on `:has(> .token-art)`).
fn token_face(token: Token, letter: &'static str) -> AnyView {
    view! { {letter}<img class="token-art" src=look::token_art(token) alt="" onerror="this.remove()" /> }.into_any()
}

/// A General or Financier token.
fn token_cube(token: Token) -> AnyView {
    let (class, letter) = match token {
        Token::General => ("general", "Gen"),
        Token::Financier => ("financier", "Fin"),
    };
    view! { <span class=format!("cube {class}") aria-hidden="true">{token_face(token, letter)}</span> }.into_any()
}

fn res_cube(res: Res) -> AnyView {
    view! { <span class=format!("cube {}", look::res_class(res)) aria-hidden="true">{face(Some(res), None, "")}</span> }
        .into_any()
}

fn held_cube(held: Held) -> AnyView {
    match (held.res(), held.token()) {
        (Some(res), _) => res_cube(res),
        (None, Some(token)) => token_cube(token),
        (None, None) => cube(held.class(), held.letter()),
    }
}

/// A card type's symbol on a chip of its colour, named for a tooltip and a
/// screen reader: it stands in for the word.
fn kind_badge(kind: Kind) -> AnyView {
    view! {
        <span class=format!("kind-badge {}", look::kind_class(kind)) role="img" aria-label=kind.name() title=kind.name()>
            {kind_icon(kind)}
        </span>
    }
    .into_any()
}

/// What a card scores per: a card type's symbol, or a character token.
fn per_badge(per: Per) -> AnyView {
    match per {
        Per::Kind(kind) => kind_badge(kind),
        Per::Token(token) => token_badge(token),
    }
}

fn token_badge(token: Token) -> AnyView {
    view! { <span class="per-token" role="img" aria-label=token.name() title=token.name()>{bonus_cube(Bonus::Token(token))}</span> }
        .into_any()
}

fn bonus_cube(bonus: Bonus) -> AnyView {
    match bonus {
        Bonus::Krystallium => cube("krystallium", "K"),
        Bonus::Token(token) => token_cube(token),
    }
}

/// `n` of `of` round markers lit, e.g. the cubes on the Empire.
fn pips(n: u8, of: u8) -> AnyView {
    let dots = (0..of)
        .map(|i| view! { <span class={if i < n { "pip on" } else { "pip" }}></span> })
        .collect_view();
    view! { <span class="pips" role="img" aria-label=format!("{n} of {of}")>{dots}</span> }.into_any()
}

pub(super) fn kind_icon(kind: Kind) -> AnyView {
    let path = match kind {
        Kind::Structure => "M2 14V6l6-4 6 4v8H9.5v-4h-3v4z",
        Kind::Vehicle => "M8 1c3 2 4.5 5 3.5 9h-7C3.5 6 5 3 8 1zm-3 10-2.5 4 3.5-1zm6 0 2.5 4-3.5-1z",
        Kind::Research => "M6 1h4v1.5h-.8v3.7l3.9 6.4a1.2 1.2 0 0 1-1 1.8H3.9a1.2 1.2 0 0 1-1-1.8l3.9-6.4V2.5H6z",
        Kind::Project => "M8 1l6 3.5v7L8 15l-6-3.5v-7zm0 4a3 3 0 1 0 0 6 3 3 0 0 0 0-6z",
        Kind::Discovery => "M8 1l1.8 5.2L15 8l-5.2 1.8L8 15l-1.8-5.2L1 8l5.2-1.8z",
    };
    view! {
        <svg class="kind-icon" viewBox="0 0 16 16" aria-hidden="true">
            <path d=path fill="currentColor" fill-rule="evenodd" />
        </svg>
    }
    .into_any()
}

/// What a seat produces each round, as cubes with a number.
fn production_line(production: [u8; 5]) -> AnyView {
    let stats = Res::ALL
        .iter()
        .map(|&res| {
            let n = production[res.index()];
            view! {
                <span class="stat" title=format!("{}: {} per round", res.name(), n)>
                    {res_cube(res)}
                    <b>{n}</b>
                </span>
            }
        })
        .collect_view();
    view! { <span class="stats production-line" role="group" aria-label="Production per round">{stats}</span> }
        .into_any()
}

// ----- cards ---------------------------------------------------------------------------

fn card_class(card: &Card, extra: &str) -> String {
    format!("card {} {extra}", look::kind_class(card.kind))
}

/// The spaces a card needs filled, drawn as the cubes it asks for.
fn requirement_sockets(cost: &Cost) -> AnyView {
    let list = look::slots(cost, &Cost::default())
        .into_iter()
        .map(|slot| {
            view! {
                <span class=format!("socket req {}", slot.space.class()) title=slot.space.name()>
                    {face(slot.space.res(), slot.space.token(), slot.space.letter())}
                </span>
            }
        })
        .collect_view();
    view! { <span class="sockets">{list}</span> }.into_any()
}

fn row(class: &'static str, label: &'static str, body: AnyView) -> AnyView {
    view! {
        <span class=format!("row {class}")>
            <span class="row-label">{label}</span>
            <span class="row-body">{body}</span>
        </span>
    }
    .into_any()
}

/// A corrupted resource (Corruption & Ascension): its cube, struck out.
fn corrupt_cube(res: Res) -> AnyView {
    view! {
        <span class=format!("cube corrupt {}", look::res_class(res)) title=format!("{} corrupted", res.name()) aria-hidden="true">
            {face(Some(res), None, "")}
        </span>
    }
    .into_any()
}

fn makes_row(card: &'static Card) -> Option<AnyView> {
    let mut icons: Vec<AnyView> = Vec::new();
    for res in Res::ALL {
        for _ in 0..card.produces[res.index()] {
            icons.push(res_cube(res));
        }
    }
    for res in Res::ALL {
        for _ in 0..card.corruption[res.index()] {
            icons.push(corrupt_cube(res));
        }
    }
    let per = card.scaled.map(|(res, kind)| {
        view! {
            <span class="per">
                {res_cube(res)}
                <span class="per-text">"per"</span>
                {kind_badge(kind)}
            </span>
        }
    });
    if icons.is_empty() && per.is_none() {
        return None;
    }
    Some(row("makes", "Makes", view! { {icons}{per} }.into_any()))
}

/// The points, with card types and characters drawn as their symbols; the
/// row's tooltip spells them out.
fn worth_row(card: &'static Card) -> Option<AnyView> {
    let text = look::points_parts(card);
    if text.is_empty() {
        return None;
    }
    let mut parts: Vec<AnyView> = Vec::new();
    if card.vp > 0 {
        parts.push(view! { <span class="vp">{format!("{} VP", card.vp)}</span> }.into_any());
    }
    let per = |points: u8, of: Vec<AnyView>| {
        view! {
            <span class="vp-per">
                <span class="vp">{format!("{points} VP")}</span>
                <span class="per-text">"per"</span>
                {of}
            </span>
        }
        .into_any()
    };
    if let Some((kind, points)) = card.combo {
        parts.push(per(points, vec![kind_badge(kind)]));
    }
    if let Some((token, points)) = card.per_token {
        parts.push(per(points, vec![token_badge(token)]));
    }
    if let Some((a, b, points)) = card.duo {
        parts.push(per(points, vec![per_badge(a), view! { <span class="vp">"+"</span> }.into_any(), per_badge(b)]));
    }
    let parts: Vec<AnyView> = parts
        .into_iter()
        .enumerate()
        .flat_map(|(i, part)| {
            let plus = (i > 0).then(|| view! { <span class="vp">"+"</span> }.into_any());
            plus.into_iter().chain([part])
        })
        .collect();
    Some(row("worth", "Worth", view! { <span class="worth-body" title=text.join(" + ")>{parts}</span> }.into_any()))
}

fn built_row(card: &'static Card) -> Option<AnyView> {
    let text = look::bonus_text(card)?;
    let cubes = card.bonus.iter().map(|&bonus| bonus_cube(bonus)).collect_view();
    Some(row(
        "built",
        "When built",
        view! {
            {cubes}
            <span class="bonus-text">{text}</span>
        }
        .into_any(),
    ))
}

/// Two arrows chasing each other: what recycling the card gives.
fn recycle_icon() -> AnyView {
    view! {
        <svg class="recycle-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path
                d="M12 4V1L8 5l4 4V6c3.31 0 6 2.69 6 6 0 1.01-.25 1.97-.7 2.8l1.46 1.46C19.54 15.03 20 13.57 20 12c0-4.42-3.58-8-8-8zm0 14c-3.31 0-6-2.69-6-6 0-1.01.25-1.97.7-2.8L5.24 7.74C4.46 8.97 4 10.43 4 12c0 4.42 3.58 8 8 8v3l4-4-4-4v3z"
                fill="currentColor"
            />
        </svg>
    }
    .into_any()
}

/// What a card shows inside its frame. `cost` is the row of spaces (static
/// for a card in hand, live for a building). Its band names the type, the
/// expansion's mark if it has one, and the cube recycling it gives; under it
/// comes the printed card's picture, with the card's name over its foot.
fn card_body(card: &'static Card, cost: Option<AnyView>) -> AnyView {
    let cost = cost.map(|c| view! { <span class="card-cost">{c}</span> });
    view! {
        <span class="card-head">
            {kind_icon(card.kind)}
            <span class="card-kind">{card.kind.name()}</span>
            <span class="card-tags">
                {card.ascension.then(|| view! { <span class="card-deck" title="Corruption & Ascension">"C&A"</span> })}
                <span class="card-recycle" title=format!("Recycles into {}", look::recycle_cube(card))>
                    {recycle_icon()}
                    {res_cube(card.recycle)}
                </span>
            </span>
        </span>
        <span class="card-art">
            <img src=look::card_art(card) alt="" width="400" height="300" loading="lazy" decoding="async" />
            <span class="card-name">{card.name}</span>
        </span>
        {cost}
        <span class="card-rows">{makes_row(card)}{worth_row(card)}{built_row(card)}</span>
    }
    .into_any()
}

/// A card that isn't a button: in the draft area, in the Empire, in a list.
fn card_static(card: &'static Card, with_cost: bool, extra: &str) -> AnyView {
    let cost = with_cost.then(|| requirement_sockets(&card.cost));
    let summary = look::card_summary(card);
    view! {
        <div class=card_class(card, extra) role="group" aria-label=summary.clone() title=summary>
            {card_body(card, cost)}
        </div>
    }
    .into_any()
}

// ----- the status band ------------------------------------------------------------------

fn top_bar(app: App, room: String) -> AnyView {
    view! {
        <header class="top-bar">
            <h1 class="brand">"It's a Wonderful World"</h1>
            <span class="room">"Room " <b class="room-tag">{room}</b></span>
            <button class="link" on:click=move |_| leave(app)>
                "Leave game"
            </button>
        </header>
    }
    .into_any()
}

fn status_band(ctx: &Ctx) -> AnyView {
    let view = &ctx.view;
    let waiting = look::waiting_on(view);
    let over = view.phase == Phase::Over;
    let seats = view
        .players
        .iter()
        .enumerate()
        .map(|(seat, _)| {
            let done = !waiting.contains(&seat);
            view! {
                <li class={if done { "seat done" } else { "seat" }}>
                    <span class="dot"></span>
                    {ctx.who(seat)}
                    <span class="sr">{if done { " is done" } else { " is deciding" }}</span>
                </li>
            }
        })
        .collect_view();
    // The ring's arrows show which way the hands go this round.
    let passing = format!(
        "cards pass to the {} player",
        if view.passes_to_next { "next" } else { "previous" }
    );

    view! {
        <section class="status" aria-live="polite">
            <div class="turn">
            <div class="round" aria-label=format!("Round {} of {}, {}", view.round, ROUNDS, passing)>
                <span class="round-word">"Round"</span>
                <span
                    class={if view.passes_to_next { "round-ring next" } else { "round-ring previous" }}
                    title=passing.clone()
                >
                    <img class="round-art" src=look::round_art(view.passes_to_next) alt="" onerror="this.remove()" />
                    <span class="round-num">{view.round}</span>
                </span>
                <span class="round-of">{format!("of {ROUNDS}")}</span>
            </div>
            {turn_track(ctx)}
            </div>
            <div class="phase">
                <h2>{look::phase_title(view)}</h2>
                <p class="hint">{look::hint(view, &ctx.names)}</p>
            </div>
            {(!over).then(|| view! { <ul class="seats">{seats}</ul> })}
        </section>
    }
    .into_any()
}

fn phase_panel(ctx: &Ctx) -> AnyView {
    match ctx.view.phase {
        Phase::Draft => draft_panel(ctx),
        Phase::Planning => planning_panel(ctx),
        Phase::Production { step } => production_panel(ctx, step),
        Phase::Over => results_panel(ctx),
    }
}

// ----- draft ------------------------------------------------------------------------------

fn draft_panel(ctx: &Ctx) -> AnyView {
    let view = &ctx.view;
    let picked = view.picked;

    let pick = picked.map(|card| {
        view! {
            <div class="your-pick">{card_static(card.def(), true, "picked")}</div>
        }
    });

    let hand = view
        .hand
        .iter()
        .map(|&card| {
            let ctx = ctx.clone();
            let def = card.def();
            let summary = look::card_summary(def);
            view! {
                <li>
                    <button
                        class=card_class(def, if picked.is_some() { "pickable passing" } else { "pickable" })
                        disabled=picked.is_some()
                        aria-label=summary.clone()
                        title=summary
                        on:click=move |_| ctx.act(Action::Draft { card })
                    >
                        {card_body(def, Some(requirement_sockets(&def.cost)))}
                    </button>
                </li>
            }
        })
        .collect_view();

    let passing = (picked.is_some() && !view.hand.is_empty()).then(|| {
        let to = ctx.name(look::pass_to(view));
        view! { <p class="caption">{format!("Passing on to {to}")}</p> }
    });

    let kept = (!view.drafted.is_empty()).then(|| {
        let cards = view
            .drafted
            .iter()
            .map(|&card| view! { <li>{card_static(card.def(), true, "")}</li> })
            .collect_view();
        view! {
            <h3 class="subhead">
                "Kept so far"
                <span class="count">{format!("{} of {}", view.drafted.len(), DRAFT_PICKS)}</span>
            </h3>
            <ul class="cards kept">{cards}</ul>
        }
    });

    view! {
        <section class="panel draft">
            <h2>{if picked.is_some() { "Your pick is in" } else { "Pick a card" }}</h2>
            {pick}
            {passing}
            <ul class="cards hand">{hand}</ul>
            {kept}
        </section>
    }
    .into_any()
}

// ----- planning ---------------------------------------------------------------------------

fn planning_panel(ctx: &Ctx) -> AnyView {
    let view = &ctx.view;
    let me = ctx.me();
    let left = view.drafted.len();

    let cards = view
        .drafted
        .iter()
        .map(|&card| {
            let def = card.def();
            let build = {
                let ctx = ctx.clone();
                move |_| ctx.act(Action::Build { card })
            };
            let recycle = {
                let ui = ctx.ui();
                move |_| ui.cube_from.set(Some(CubeFrom::Recycle(card)))
            };
            view! {
                <li class="plan-card">
                    {card_static(def, true, "")}
                    <div class="plan-actions">
                        <button class="primary" on:click=build>
                            "Build"
                        </button>
                        <button on:click=recycle>"Recycle for " {res_cube(def.recycle)}</button>
                    </div>
                </li>
            }
        })
        .collect_view();

    let done = {
        let ctx = ctx.clone();
        move |_| ctx.act(Action::Ready)
    };
    let label = if me.ready { "Waiting for the others…" } else { "Done planning" };

    view! {
        <section class="panel planning">
            <h2>
                {if left > 0 {
                    format!("Plan your {left} card{}", if left == 1 { "" } else { "s" })
                } else {
                    "Planning".to_string()
                }}
            </h2>
            {(left > 0).then(|| view! { <ul class="cards plan">{cards}</ul> })}
            <div class="panel-actions">
                <button class="primary" disabled={left > 0 || me.ready} on:click=done>
                    {label}
                </button>
            </div>
        </section>
    }
    .into_any()
}

// ----- production -------------------------------------------------------------------------

fn production_panel(ctx: &Ctx, step: u8) -> AnyView {
    let me = ctx.me();
    let choose = me.choose.then(|| {
        let take = |token: Token| {
            let ctx = ctx.clone();
            move |_| ctx.act(Action::Choose { token })
        };
        view! {
            <div class="choose">
                <p>"You produced the most Science: take a character."</p>
                <div class="choose-buttons">
                    <button class="primary" on:click=take(Token::General)>
                        "Take a General"
                    </button>
                    <button class="primary" on:click=take(Token::Financier)>
                        "Take a Financier"
                    </button>
                </div>
            </div>
        }
    });

    view! {
        <section class="panel production">
            {race(ctx, step)}
            {choose}
        </section>
    }
    .into_any()
}

/// The round's progression, beside the round ring: the draft, the planning,
/// then each production step with its resource, the character at stake and,
/// once played, who took it. A cursor marks where the round is.
fn turn_track(ctx: &Ctx) -> AnyView {
    // Draft 0, Planning 1, then production step n at 2 + n.
    let now = match ctx.view.phase {
        Phase::Draft => 0,
        Phase::Planning => 1,
        Phase::Production { step } => 2 + usize::from(step),
        Phase::Over => usize::MAX,
    };
    let at = |i: usize| match i.cmp(&now) {
        std::cmp::Ordering::Less => "track-step done",
        std::cmp::Ordering::Equal => "track-step now",
        std::cmp::Ordering::Greater => "track-step",
    };
    let word = |i: usize, name: &'static str, short: &'static str| {
        view! {
            <li class=at(i) aria-current={if i == now { "step" } else { "false" }} title=name>
                <span class="track-mark word" aria-hidden="true">{short}</span>
                <span class="sr">{name}</span>
            </li>
        }
    };
    let production = (0..=WRAP_UP)
        .map(|step| {
            let i = 2 + usize::from(step);
            let (icon, name) = match Res::ALL.get(step as usize) {
                Some(&res) => (res_cube(res), res.name()),
                None => (cube("wrap", "+"), "Wrap-up"),
            };
            view! {
                <li class=at(i) aria-current={if i == now { "step" } else { "false" }} title=format!("Production: {name}")>
                    <span class="track-mark">{icon}</span>
                    <span class="sr">{format!("Production: {name}")}</span>
                    {step_award(ctx, step)}
                </li>
            }
        })
        .collect_view();
    view! {
        <ol class="track" aria-label="This round">
            {word(0, "Draft", "Draft")}
            {word(1, "Planning", "Plan")}
            {production}
        </ol>
    }
    .into_any()
}

/// What producing the most of a step's resource wins, as the production
/// board prints it above each resource (a General, a Financier, or either
/// for Science), then who took it once the step is played.
fn step_award(ctx: &Ctx, step: u8) -> Option<AnyView> {
    let res = *Res::ALL.get(step as usize)?;
    let (tokens, prize) = match res.supremacy_token() {
        Some(token) => (vec![token], format!("a {}", token.name())),
        None => (vec![Token::General, Token::Financier], "a General or a Financier".to_string()),
    };
    let stake = tokens.into_iter().map(token_cube).collect_view();
    let winners = ctx.view.awards.get(step as usize);
    let (taken, title) = match winners {
        None => (None, format!("Most {}: {prize}", res.name())),
        Some(seats) if seats.is_empty() => {
            (Some("nobody".to_string()), format!("{prize}: nobody (a tie, or none produced)"))
        }
        Some(seats) => {
            let who = seats.iter().map(|&s| ctx.who(s)).collect::<Vec<_>>().join(" & ");
            (Some(who.clone()), format!("{prize}: {who}"))
        }
    };
    let class = match winners {
        None => "step-award",
        Some(seats) if seats.is_empty() => "step-award nobody",
        Some(_) => "step-award won",
    };
    Some(
        view! {
            <span class=class title=title.clone() aria-label=title>
                <span class="award-tokens">{stake}</span>
                {taken.map(|who| view! { <span class="award-who">{who}</span> })}
            </span>
        }
        .into_any(),
    )
}

/// Who produced how much in this step, and who takes the character.
fn race(ctx: &Ctx, step: u8) -> Option<AnyView> {
    let res = *Res::ALL.get(step as usize)?;
    let view = &ctx.view;
    let most = view.players.iter().map(|p| p.produced).max().unwrap_or(0);
    if most == 0 {
        return None;
    }
    let leaders = look::supremacy(view);
    let prize = match res.supremacy_token() {
        Some(token) => format!("takes a {}", token.name()),
        None => "chooses a character".to_string(),
    };
    let rows = view
        .players
        .iter()
        .enumerate()
        .map(|(seat, p)| {
            let lead = leaders.contains(&seat);
            let width = u32::from(p.produced) * 100 / u32::from(most);
            view! {
                <li class={if lead { "race-row lead" } else { "race-row" }}>
                    <span class="race-name">{ctx.who(seat)}</span>
                    <span class="race-bar" style=format!("--w:{width}%")></span>
                    <span class="race-n">{p.produced}</span>
                    {lead.then(|| view! { <span class="race-prize">{prize.clone()}</span> })}
                </li>
            }
        })
        .collect_view();
    let tied = leaders.is_empty();
    Some(
        view! {
            <div class="race">
                <h3 class="subhead">
                    {res_cube(res)}
                    {format!("{} produced", res.name())}
                    {tied.then(|| view! { <span class="count">"tied: no character"</span> })}
                </h3>
                <ul class="race-list">{rows}</ul>
            </div>
        }
        .into_any(),
    )
}

// ----- the pieces you hold ------------------------------------------------------------------

fn piece_button(ctx: &Ctx, held: Held, n: u8) -> AnyView {
    let ui = ctx.ui();
    let selected = {
        let ctx = ctx.clone();
        move || look::effective_held(&ctx.view, ctx.ui().held.get()) == Some(held)
    };
    let class = {
        let selected = selected.clone();
        move || if selected() { "piece selected" } else { "piece" }
    };
    view! {
        <button
            class=class
            aria-pressed=move || selected().to_string()
            title=held.name()
            on:click=move |_| ui.held.set(Some(held))
        >
            {held_cube(held)}
            <span class="count">{format!("×{n}")}</span>
            <span class="piece-name">{held.name()}</span>
        </button>
    }
    .into_any()
}

/// What you can place, the Empire as a place for cubes, and (in production)
/// the button that ends your step. Sticks to the bottom of the screen while
/// you scroll through your buildings.
fn tray(ctx: &Ctx) -> AnyView {
    let view = &ctx.view;
    let in_production = matches!(view.phase, Phase::Production { .. });
    if !in_production && view.phase != Phase::Planning {
        return ().into_any();
    }
    let have = look::available(view);
    if have.is_empty() && !in_production {
        return ().into_any();
    }
    let me = ctx.me();
    let has_cubes = have.iter().any(|(h, _)| matches!(h, Held::Cube(_)));
    let pieces = have.iter().map(|&(held, n)| piece_button(ctx, held, n)).collect_view();

    let empire = has_cubes.then(|| {
        let can = {
            let ctx = ctx.clone();
            move || look::pick_for_empire(&ctx.view, ctx.ui().held.get()).is_some()
        };
        let click = {
            let ctx = ctx.clone();
            move |_| {
                let chosen = ctx.ui().held.get_untracked();
                if let Some(action) = look::pick_for_empire(&ctx.view, chosen).and_then(look::empire_action) {
                    ctx.act(action);
                }
            }
        };
        view! {
            <button class="empire-target" disabled=move || !can() on:click=click>
                <span class="target-name">"Put on your Empire"</span>
                {pips(me.empire_cubes, 5)}
            </button>
        }
    });

    // What you placed this step stays yours to take back until you press Done.
    let undo = look::undo_title(view).map(|title| {
        let ctx2 = ctx.clone();
        view! {
            <button class="undo" title=title.clone() aria-label=title on:click=move |_| ctx2.act(Action::Undo)>
                "Undo"
            </button>
        }
    });
    let done = in_production.then(|| {
        let ctx2 = ctx.clone();
        let click = move |_| ctx2.act(Action::Ready);
        let (label, disabled) = look::done_button(view);
        view! {
            <button class="primary" disabled=disabled on:click=click>
                {label}
            </button>
        }
    });
    let cue = look::placing_cue(view).map(|cue| view! { <p class="placing-cue">{cue}</p> });

    view! {
        <section class="panel tray">
            <div class="tray-pieces" role="group" aria-label="Pieces you can place">
                {if have.is_empty() {
                    view! { <span class="empty">"Nothing to place right now."</span> }.into_any()
                } else {
                    pieces.into_any()
                }}
            </div>
            <div class="tray-actions">{cue}{empire}{undo}{done}</div>
        </section>
    }
    .into_any()
}

// ----- your buildings ------------------------------------------------------------------------

/// One space of a building: a free one lights up when what you hold fits.
fn live_socket(ctx: &Ctx, card: CardId, slot: Slot) -> AnyView {
    let space = slot.space;
    if slot.filled {
        return view! {
            <span class=format!("socket filled {}", space.class()) title=format!("{} placed", space.name())>
                {face(space.res(), space.token(), space.letter())}
            </span>
        }
        .into_any();
    }
    let class = {
        let ctx = ctx.clone();
        move || {
            let fits = look::pick_for_space(&ctx.view, ctx.ui().held.get(), space).is_some();
            format!("socket free {}{}", space.class(), if fits { " fits" } else { "" })
        }
    };
    let disabled = {
        let ctx = ctx.clone();
        move || look::pick_for_space(&ctx.view, ctx.ui().held.get(), space).is_none()
    };
    let click = {
        let ctx = ctx.clone();
        move |_| {
            let chosen = ctx.ui().held.get_untracked();
            let action = look::pick_for_space(&ctx.view, chosen, space)
                .and_then(|held| look::slot_action(held, space, card));
            if let Some(action) = action {
                ctx.act(action);
            }
        }
    };
    view! {
        <button
            class=class
            disabled=disabled
            on:click=click
            aria-label=format!("Free {} space on {}", space.name(), card.def().name)
        >
            {face(space.res(), space.token(), space.letter())}
        </button>
    }
    .into_any()
}

fn building_card(ctx: &Ctx, building: &Building) -> AnyView {
    let card = building.card;
    let def = card.def();
    let (placed, needed) = look::progress(&def.cost, &building.filled);
    let sockets = look::slots(&def.cost, &building.filled)
        .into_iter()
        .map(|slot| live_socket(ctx, card, slot))
        .collect_view();
    let scrap = {
        let ui = ctx.ui();
        move |_| ui.cube_from.set(Some(CubeFrom::Scrap(card)))
    };
    let summary = look::card_summary(def);
    view! {
        <li>
            <div class=card_class(def, "building") role="group" aria-label=summary.clone() title=summary>
                {card_body(def, Some(view! { <span class="sockets">{sockets}</span> }.into_any()))}
                <span class="card-foot">
                    <span class="progress">{format!("{placed} of {needed} placed")}</span>
                    <button class="link" on:click=scrap>
                        "Scrap"
                    </button>
                </span>
            </div>
        </li>
    }
    .into_any()
}

fn buildings_panel(ctx: &Ctx) -> AnyView {
    let me = ctx.me();
    let cards = me.buildings.iter().map(|b| building_card(ctx, b)).collect_view();
    view! {
        <section class="panel buildings">
            <h2>"Under construction"</h2>
            {if me.buildings.is_empty() {
                view! {
                    <p class="empty">
                        "Nothing yet. Cards you build in the planning phase wait here for cubes."
                    </p>
                }
                .into_any()
            } else {
                view! { <ul class="cards building-list">{cards}</ul> }.into_any()
            }}
        </section>
    }
    .into_any()
}

// ----- your Empire ------------------------------------------------------------------------------

fn stock_line(p: &PlayerView) -> AnyView {
    view! {
        <span class="stats stock-line" role="group" aria-label="What you hold">
            <span class="stat" title="Cubes on the Empire: five make a Krystallium">
                "Empire"
                {pips(p.empire_cubes, 5)}
            </span>
            <span class="stat" title="Krystallium">
                {cube("krystallium", "K")}
                <b>{p.krystallium}</b>
            </span>
            <span class="stat" title="Generals">
                {token_cube(Token::General)}
                <b>{p.generals}</b>
            </span>
            <span class="stat" title="Financiers">
                {token_cube(Token::Financier)}
                <b>{p.financiers}</b>
            </span>
        </span>
    }
    .into_any()
}

/// An Empire's round crest. `alt` is empty where its name is written beside
/// it; otherwise it is the name, which a pointer over it shows too.
fn crest(empire: usize, alt: &'static str) -> AnyView {
    let title = (!alt.is_empty()).then_some(alt);
    view! { <img class="crest" src=look::crest_art(empire) alt=alt title=title width="96" height="96" decoding="async" /> }
        .into_any()
}

fn points(p: &PlayerView) -> AnyView {
    view! {
        <div class="points">
            <span class="points-num">{p.points}</span>
            <span class="points-word">{if p.points == 1 { "point" } else { "points" }}</span>
        </div>
    }
    .into_any()
}

fn empire_panel(ctx: &Ctx) -> AnyView {
    let me = ctx.me();
    let counts = look::kind_counts(&me.empire);
    let columns = Kind::ALL
        .iter()
        .zip(counts)
        .filter(|(_, n)| *n > 0)
        .map(|(&kind, n)| {
            let cards = me
                .empire
                .iter()
                .filter(|id| id.def().kind == kind)
                .map(|id| view! { <li>{card_static(id.def(), false, "")}</li> })
                .collect_view();
            view! {
                <section class="kind-col">
                    <h3 class="subhead">
                        {kind_icon(kind)}
                        {kind.name()}
                        <span class="count">{n}</span>
                    </h3>
                    <ul class="cards finished">{cards}</ul>
                </section>
            }
        })
        .collect_view();

    view! {
        <section class="panel empire">
            <div class="empire-head">
                <img
                    class="empire-art"
                    src=look::empire_art(me.empire_card)
                    alt=""
                    width="1200"
                    height="400"
                    loading="lazy"
                    decoding="async"
                />
                <div>
                    <div class="empire-title">{crest(me.empire_card, "")}<h2>"Your Empire"</h2></div>
                    <p class="empire-name">{look::empire_name(me.empire_card)}</p>
                    <p class="empire-points">{look::empire_points(me.empire_card)}</p>
                </div>
                {points(me)}
            </div>
            <div class="empire-stats">
                <div class="stat-block">
                    <span class="row-label">"Produces each round"</span>
                    {production_line(me.production)}
                </div>
                <div class="stat-block">
                    <span class="row-label">"You hold"</span>
                    {stock_line(me)}
                </div>
            </div>
            {if me.empire.is_empty() {
                view! { <p class="empty">"No finished cards yet. Fill every space of a building to finish it."</p> }
                    .into_any()
            } else {
                view! { <div class="kinds">{columns}</div> }.into_any()
            }}
        </section>
    }
    .into_any()
}

// ----- the other players ---------------------------------------------------------------------------

fn mini_building(b: &Building) -> AnyView {
    let def = b.card.def();
    let (placed, needed) = look::progress(&def.cost, &b.filled);
    let pct = (placed * 100).checked_div(needed).unwrap_or(100);
    view! {
        <li class=format!("mini {}", look::kind_class(def.kind)) title=look::card_summary(def)>
            <span class="mini-name">{def.name}</span>
            <span class="mini-progress" style=format!("--done:{pct}%")></span>
            <span class="mini-count">{format!("{placed}/{needed}")}</span>
        </li>
    }
    .into_any()
}

fn rival(ctx: &Ctx, seat: usize) -> AnyView {
    let p = &ctx.view.players[seat];
    let offline = ctx.connected.get(seat).is_some_and(|c| !c);
    let waiting = look::waiting_on(&ctx.view).contains(&seat);
    let over = ctx.view.phase == Phase::Over;
    let status = if over {
        ""
    } else if waiting {
        "deciding"
    } else {
        "done"
    };
    let buildings = p.buildings.iter().map(mini_building).collect_view();
    let chips = p
        .empire
        .iter()
        .map(|id| {
            let def = id.def();
            view! { <li class=format!("chip {}", look::kind_class(def.kind)) title=look::card_summary(def)>{def.name}</li> }
        })
        .collect_view();
    view! {
        <li class="rival">
            <div class="rival-head">
                {crest(p.empire_card, "")}
                <div class="rival-who">
                    <h3>
                        {ctx.name(seat)}
                        {offline.then(|| view! { <span class="tag tag-off">"offline"</span> })}
                    </h3>
                    <p class="empire-name" title=look::empire_points(p.empire_card)>{look::empire_name(p.empire_card)}</p>
                </div>
                {points(p)}
            </div>
            {(!status.is_empty()).then(|| view! { <p class={format!("rival-status {status}")}>{status}</p> })}
            {production_line(p.production)}
            {stock_line(p)}
            {(!p.buildings.is_empty()).then(|| view! { <ul class="minis">{buildings}</ul> })}
            {(!p.empire.is_empty()).then(|| view! { <ul class="chips">{chips}</ul> })}
        </li>
    }
    .into_any()
}

fn rivals_panel(ctx: &Ctx) -> AnyView {
    let rows = (0..ctx.view.players.len())
        .filter(|&seat| seat != ctx.view.you)
        .map(|seat| rival(ctx, seat))
        .collect_view();
    view! {
        <section class="panel rivals">
            <h2>"Other players"</h2>
            <ul class="rival-list">{rows}</ul>
        </section>
    }
    .into_any()
}

fn log_panel(ctx: &Ctx) -> AnyView {
    let lines = ctx
        .view
        .log
        .iter()
        .rev()
        .take(12)
        .map(|event| view! { <li>{look::describe_event(event, &ctx.names, ctx.view.you)}</li> })
        .collect_view();
    view! {
        <section class="panel log-panel">
            <h2>"What happened"</h2>
            {if ctx.view.log.is_empty() {
                view! { <p class="empty">"Nothing yet."</p> }.into_any()
            } else {
                view! { <ol class="log">{lines}</ol> }.into_any()
            }}
        </section>
    }
    .into_any()
}

// ----- where a recycled cube goes ---------------------------------------------------------------------

/// The dialog that asks where the cube of a recycled card or a scrapped
/// building goes (see `Ui::cube_from`).
fn cube_picker(ctx: &Ctx) -> AnyView {
    let ctx = ctx.clone();
    (move || {
        let ui = ctx.ui();
        let from = ui.cube_from.get().filter(|from| from.valid(&ctx.view))?;
        let res = from.res();
        let def = from.card().def();
        let scrapping = matches!(from, CubeFrom::Scrap(_));
        let title = if scrapping { format!("Scrap {}", def.name) } else { format!("Recycle {}", def.name) };

        let targets = look::cube_targets(&ctx.view, from)
            .into_iter()
            .map(|to| {
                let detail = match to {
                    wonderful_core::Target::Empire => format!("{} of 5 cubes", ctx.me().empire_cubes),
                    wonderful_core::Target::Card(id) => {
                        let left = ctx.me().buildings.iter().find(|b| b.card == id).map_or(0, |b| b.remaining().res[res.index()]);
                        format!("needs {left} more {}", res.name())
                    }
                };
                let click = {
                    let ctx = ctx.clone();
                    move |_| {
                        ctx.act(from.action(to));
                        ctx.ui().cube_from.set(None);
                    }
                };
                view! {
                    <li>
                        <button on:click=click>
                            <span class="target-name">{look::target_name(to)}</span>
                            <span class="target-detail">{detail}</span>
                        </button>
                    </li>
                }
            })
            .collect_view();

        Some(
            view! {
                <div class="modal-bg">
                    <div class="modal panel" role="dialog" aria-modal="true" aria-labelledby="cube-picker-title">
                        <h2 id="cube-picker-title">{title}</h2>
                        <p class="modal-text">
                            "It gives you a cube: " {res_cube(res)}
                            {if scrapping {
                                format!(" {}, which goes on your Empire.", res.name())
                            } else {
                                format!(" {}. Where does it go?", res.name())
                            }}
                        </p>
                        {scrapping
                            .then(|| view! { <p class="warning">"Everything placed on this building is lost."</p> })}
                        <ul class="targets">{targets}</ul>
                        <button class="link" on:click=move |_| ui.cube_from.set(None)>
                            "Cancel"
                        </button>
                    </div>
                </div>
            }
            .into_any(),
        )
    })
    .into_any()
}

// ----- the end of the game ----------------------------------------------------------------------------

fn results_panel(ctx: &Ctx) -> AnyView {
    let app = ctx.app;
    let you = ctx.view.you;
    let rows = look::results(&ctx.view, &ctx.names)
        .into_iter()
        .map(|r| {
            view! {
                <tr class={if r.winner { "winner" } else { "" }}>
                    <th scope="row">
                        {crest(ctx.view.players[r.seat].empire_card, look::empire_name(ctx.view.players[r.seat].empire_card))}
                        <span class="rank-name">{r.name.clone()}</span>
                        {(r.seat == you).then(|| view! { <span class="tag">"you"</span> })}
                        {r.winner.then(|| view! { <span class="tag tag-win">"winner"</span> })}
                    </th>
                    <td data-label="Card points">{r.score.gross}</td>
                    <td data-label="Bonuses">{r.score.combo}</td>
                    <td data-label="Generals">{r.score.generals}</td>
                    <td data-label="Financiers">{r.score.financiers}</td>
                    <td class="total">{r.score.total}</td>
                    <td class="cards-done" data-label="Finished cards">{r.score.cards}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <section class="panel results">
            <h2 class="verdict">{look::verdict(&ctx.view, &ctx.names)}</h2>
            <div class="table-wrap">
                <table class="standings">
                    <thead>
                        <tr>
                            <th scope="col">"Player"</th>
                            <th scope="col">"Card points"</th>
                            <th scope="col">"Bonuses"</th>
                            <th scope="col">"Generals"</th>
                            <th scope="col">"Financiers"</th>
                            <th scope="col">"Total"</th>
                            <th scope="col">"Finished cards"</th>
                        </tr>
                    </thead>
                    <tbody>{rows}</tbody>
                </table>
            </div>
            <p class="note">
                "Bonuses are the points that depend on card types and characters. A tie goes to whoever finished the most cards, then to whoever holds the most characters."
            </p>
            <div class="panel-actions">
                <button class="primary" on:click=move |_| leave(app)>
                    "Leave room"
                </button>
            </div>
        </section>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::tachys::view::RenderHtml;
    use wonderful_core::{catalogue, State};

    const NAMES: [&str; 7] = ["Ann", "Bob", "Cy", "Di", "Ed", "Flo", "Gus"];

    /// Runs `f` inside a fresh reactive owner, with an executor registered so
    /// that creating effects doesn't panic (the tasks are never polled).
    fn in_owner<R>(f: impl FnOnce() -> R) -> R {
        let _ = any_spawner::Executor::init_futures_executor();
        Owner::new().with(f)
    }

    fn names(n: usize) -> Vec<String> {
        NAMES[..n].iter().map(|s| s.to_string()).collect()
    }

    /// The board for `seat` as the browser would first paint it.
    fn render_with(app: App, state: &State, seat: usize) -> String {
        let n = state.seats();
        game_screen(app, state.view_for(seat), names(n), vec![true; n], "ABCD".into()).to_html()
    }

    fn render(state: &State, seat: usize) -> String {
        in_owner(|| render_with(App::new(), state, seat))
    }

    fn id(name: &str) -> CardId {
        CardId(catalogue().iter().position(|c| c.name == name).unwrap() as u16)
    }

    /// What a simple player would do next, if anything: the game is played
    /// to its end with these moves so that every state gets drawn.
    fn next_move(state: &State, seat: usize) -> Option<Action> {
        let p = &state.players[seat];
        match state.phase {
            Phase::Draft => p.picked.is_none().then(|| Action::Draft { card: p.hand[0] }),
            Phase::Planning => match p.drafted.first() {
                Some(&card) if p.buildings.len() < 3 => Some(Action::Build { card }),
                Some(&card) => Some(Action::Recycle { card, to: wonderful_core::Target::Empire }),
                None => (!p.ready).then_some(Action::Ready),
            },
            Phase::Production { step } => {
                if p.choose {
                    return Some(Action::Choose { token: Token::General });
                }
                let step_res = Res::ALL.get(step as usize).copied();
                if let Some(res) = step_res {
                    if p.pool > 0 {
                        let to = p
                            .buildings
                            .iter()
                            .find(|b| b.remaining().res[res.index()] > 0)
                            .map_or(wonderful_core::Target::Empire, |b| wonderful_core::Target::Card(b.card));
                        return Some(Action::Place { piece: wonderful_core::Piece::Cube(res), target: to });
                    }
                }
                (!p.ready).then_some(Action::Ready)
            }
            Phase::Over => None,
        }
    }

    /// Plays on until nobody has anything left to do, drawing the board for
    /// every seat after each move (or only when the phase changes, if `sparse`).
    fn play_and_draw(state: &mut State, sparse: bool, mut check: impl FnMut(&State, usize, &str)) -> usize {
        let n = state.seats();
        let mut moves = 0;
        let mut seen = (state.round, state.phase);
        for seat in 0..n {
            check(state, seat, &render(state, seat));
        }
        loop {
            let mut moved = false;
            for seat in 0..n {
                if let Some(action) = next_move(state, seat) {
                    state.apply(seat, action.clone()).unwrap_or_else(|e| panic!("{action:?}: {e:?}"));
                    moved = true;
                    moves += 1;
                    let now = (state.round, state.phase);
                    if !sparse || now != seen {
                        for s in 0..n {
                            check(state, s, &render(state, s));
                        }
                    }
                    seen = now;
                }
            }
            assert!(moves < 5000, "the game doesn't end");
            if !moved {
                break;
            }
        }
        moves
    }

    #[test]
    fn a_whole_game_can_be_drawn_from_every_seat_at_every_step() {
        for players in 2..=5 {
            let mut state = State::new(players, 40 + players as u64);
            let moves = play_and_draw(&mut state, players != 3, |state, seat, html| {
                assert!(html.contains("ABCD") && html.contains("Leave game"), "{:?}", state.phase);
                assert!(html.contains("It&#39;s a Wonderful World") || html.contains("It's a Wonderful World"));
                assert!(html.contains(&format!("{}", state.round)));
                assert!(html.contains("Your Empire") && html.contains(look::empire_name(state.empires[seat])));
                // Every other player is on the table; you are "You", not a name.
                for (other, name) in NAMES.iter().enumerate().take(state.seats()) {
                    let listed = html.contains(&format!("<h3>{name}"));
                    assert_eq!(listed, other != seat, "{name} seen from {seat}");
                }
                // (The list of who is still deciding goes away once the game is over.)
                assert_eq!(html.contains("</span>You<span class=\"sr\">"), state.phase != Phase::Over);
                match state.phase {
                    Phase::Draft if state.players[seat].picked.is_none() => {
                        assert!(html.contains("Pick a card") && html.contains("pickable"));
                    }
                    Phase::Draft => assert!(html.contains("Your pick is in")),
                    Phase::Planning => assert!(html.contains("Done planning") || html.contains("Waiting for the others")),
                    Phase::Production { .. } => assert!(html.contains("Production:")),
                    Phase::Over => assert!(html.contains("Leave room") && html.contains("standings")),
                }
            });
            assert!(moves > 100, "{players} players: only {moves} moves");
            assert_eq!(state.phase, Phase::Over, "{players} players");
        }
    }

    #[test]
    fn the_final_standings_name_the_winner_and_every_score() {
        let mut state = State::new(3, 9);
        play_and_draw(&mut state, true, |_, _, _| {});
        assert_eq!(state.phase, Phase::Over);
        for seat in 0..3 {
            let html = render(&state, seat);
            let verdict = look::verdict(&state.view_for(seat), &names(3));
            assert!(html.contains(&verdict.replace('\'', "&#39;")) || html.contains(&verdict), "{verdict}");
            assert!(html.contains("Game over"));
            for s in 0..3 {
                assert!(html.contains(&format!("<td class=\"total\">{}</td>", state.scores[s].total)));
            }
            // Each player's Empire crest by their name.
            let table = &html[html.find("class=\"standings\"").unwrap()..];
            let table = &table[..table.find("</table>").unwrap()];
            for s in 0..3 {
                assert!(table.contains(&format!("src=\"{}\"", look::crest_art(state.empires[s]))), "seat {s}");
            }
            // One winner mark per winner, and nothing left to build.
            assert_eq!(html.matches("tag tag-win").count(), state.winners().len());
            assert!(!html.contains("Under construction"));
            assert!(html.contains("Leave room"));
        }
    }

    #[test]
    fn the_draft_shows_your_hand_then_your_pick() {
        let mut state = State::new(3, 5);
        let hand = state.players[0].hand.clone();
        let before = render(&state, 0);
        assert!(before.contains("Pick a card") && !before.contains("Your pick is in"));
        assert_eq!(before.matches("class=\"card kind-").count(), hand.len(), "one pickable card each");
        assert!(before.contains("Pick a card (1 of 7)"));
        assert!(!before.contains("Kept so far"));
        for &card in &hand {
            assert!(before.contains(card.def().name), "{}", card.def().name);
        }

        state.apply(0, Action::Draft { card: hand[2] }).unwrap();
        let after = render(&state, 0);
        assert!(after.contains("Your pick is in") && after.contains("Your pick"));
        assert!(after.contains("Passing on to Bob") && after.contains("Waiting for Bob and Cy"));
        assert!(after.contains("disabled"), "the others can't be picked any more");
        // The picked card shows once, apart from the rest.
        assert_eq!(after.matches(" picked\"").count(), 1);

        // Bob hasn't picked: his own view still offers a choice and hides nothing of Ann's pick.
        let bob = render(&state, 1);
        assert!(bob.contains("Pick a card") && !bob.contains("Your pick is in"));
        assert!(bob.contains("Ann"));

        // Once everybody has picked, the pick is kept, still showing what recycling it gives.
        for seat in 1..3 {
            let card = state.players[seat].hand[0];
            state.apply(seat, Action::Draft { card }).unwrap();
        }
        let html = render(&state, 0);
        let kept = &html[html.find("Kept so far").expect("the kept cards")..];
        let kept = &kept[..kept.find("</ul>").unwrap()];
        assert!(kept.contains(hand[2].def().name) && kept.contains("Recycles into"), "{kept}");
        // And what it costs to build, one cube per space, as on the card in hand.
        let spaces = look::slots(&hand[2].def().cost, &Cost::default()).len();
        assert!(kept.contains("class=\"card-cost\""), "{kept}");
        assert_eq!(kept.matches("class=\"socket req ").count(), spaces, "{kept}");
    }

    #[test]
    fn planning_offers_build_or_recycle_for_each_card() {
        let mut state = State::new(3, 6);
        while state.phase == Phase::Draft {
            for seat in 0..3 {
                if let Some(a) = next_move(&state, seat) {
                    state.apply(seat, a).unwrap();
                }
            }
        }
        assert_eq!(state.phase, Phase::Planning);
        let html = render(&state, 0);
        assert!(html.contains("Plan your 7 cards"));
        assert_eq!(html.matches(">Build<").count(), 7);
        assert_eq!(html.matches("Recycle for ").count(), 7);
        assert!(html.contains("Done planning") && html.contains("disabled"), "not yet");
        assert!(html.contains("Nothing yet."), "no building yet");

        let card = state.players[0].drafted[0];
        state.apply(0, Action::Build { card }).unwrap();
        let html = render(&state, 0);
        assert!(html.contains("Plan your 6 cards") && !html.contains("Nothing yet."));
        assert!(html.contains("Under construction") && html.contains("0 of "));
        assert!(html.contains("Scrap"));

        // Everything decided: just the button.
        for card in state.players[0].drafted.clone() {
            state.apply(0, Action::Build { card }).unwrap();
        }
        let html = render(&state, 0);
        assert!(!html.contains("Plan your") && html.contains("Done planning"));
        assert!(!html.contains(">Build<"));
        state.apply(0, Action::Ready).unwrap();
        let html = render(&state, 0);
        assert!(html.contains("Waiting for the others"));
    }

    /// Seat 0 in the production phase of a game in which everybody has
    /// built what was drafted.
    /// Seat `i` plays Empire `i` (Noram States first).
    fn production_state() -> State {
        let mut state = State::new(3, 11);
        state.empires = vec![0, 1, 2];
        for _ in 0..2000 {
            if matches!(state.phase, Phase::Production { .. }) {
                break;
            }
            for seat in 0..3 {
                if state.phase == Phase::Planning {
                    for card in state.players[seat].drafted.clone() {
                        state.apply(seat, Action::Build { card }).unwrap();
                    }
                    if state.phase == Phase::Planning && !state.players[seat].ready {
                        state.apply(seat, Action::Ready).unwrap();
                    }
                } else if let Some(a) = next_move(&state, seat) {
                    state.apply(seat, a).unwrap();
                }
            }
        }
        assert!(matches!(state.phase, Phase::Production { step: 0 }));
        state
    }

    #[test]
    fn production_shows_the_step_the_race_and_what_to_place() {
        let mut state = production_state();
        // Seats 1 and 2 also make Materials at the start of the game: take
        // them away, so that seat 0 is alone in front.
        for seat in [1, 2] {
            state.players[seat].produced = 0;
            state.players[seat].pool = 0;
            state.players[seat].ready = true;
        }
        let html = render(&state, 0);
        assert!(html.contains("Production: Materials"));
        for step in ["Materials", "Energy", "Science", "Gold", "Exploration", "Wrap-up"] {
            assert!(html.contains(step), "{step}");
        }
        assert!(html.contains("track-step now") && html.contains("aria-current=\"step\""));
        // Seat 0 (Noram States) makes three Materials cubes and nobody else
        // does: it leads the race and takes a Financier.
        // (The log may say "...tied: nobody takes a Financier" for earlier
        // races, so the prize is looked for in the race itself.)
        assert!(html.contains("Materials produced") && html.contains("race-row lead"));
        assert!(html.contains("class=\"race-prize\">takes a Financier<"));
        assert!(html.contains("Put on your Empire") && html.contains("Done with Materials: 3 cubes to your Empire")
            && html.contains("Placing Materials — press Done when finished"));
        assert!(html.contains("piece selected"), "the first piece is the one used");
        assert!(html.contains("Materials cube"));
        assert!(!html.contains("tied: no character"));

        // Level with someone else: nobody takes the character.
        state.players[2].produced = state.players[0].produced;
        let tied = render(&state, 0);
        assert!(tied.contains("tied: no character") && !tied.contains("race-row lead") && !tied.contains("race-prize"));

        // Seat 1 produces nothing in that step: nothing to place.
        let idle = render(&state, 1);
        assert!(idle.contains("Nothing to place right now."));
        assert!(idle.contains("Waiting for the others"));
        assert!(!idle.contains("Put on your Empire"));
    }

    #[test]
    fn placing_waits_for_done_and_can_be_undone() {
        let mut state = production_state();
        let card = id("Recycling Plant"); // 2 Materials
        state.players[0].buildings = vec![wonderful_core::Building { card, filled: Cost::default() }];
        state.players[0].pool = 3;
        state.players[0].ready = false;
        assert!(!render(&state, 0).contains("class=\"undo\""), "nothing to take back yet");
        let cube = wonderful_core::Piece::Cube(Res::Materials);
        let to = wonderful_core::Target::Card(card);
        state.apply(0, Action::Place { piece: cube, target: to }).unwrap();
        let html = render(&state, 0);
        assert!(html.contains("Take back the Materials cube from Recycling Plant"));
        // Full: built at once, and undo stops there.
        state.apply(0, Action::Place { piece: cube, target: to }).unwrap();
        let html = render(&state, 0);
        assert!(!html.contains("class=\"undo\""));
        assert!(html.contains("Done with Materials: 1 cube to your Empire<"));
        state.apply(0, Action::Place { piece: wonderful_core::Piece::Cube(Res::Materials), target: wonderful_core::Target::Empire })
            .unwrap();
        let html = render(&state, 0);
        assert!(html.contains("Done with Materials<") && html.contains("Placing Materials — press Done when finished"));
        assert!(html.contains("Take back the Materials cube from your Empire"));
    }

    #[test]
    fn free_spaces_light_up_for_what_you_hold() {
        let mut state = production_state();
        // Give seat 0 a building that wants Energy and Gold, and some pieces.
        let card = id("Industrial Complex"); // 3 Materials, 1 Energy
        state.players[0].buildings = vec![wonderful_core::Building { card, filled: Cost::default() }];
        state.players[0].pool = 2;
        state.players[0].ready = false;
        state.players[0].krystallium = 1;

        let html = in_owner(|| render_with(App::new(), &state, 0));
        // Two cubes of Materials: the Materials spaces are the only ones that fit.
        assert_eq!(html.matches("socket free res-materials fits").count(), 3, "{html}");
        assert_eq!(html.matches("socket free res-energy fits").count(), 0);
        assert_eq!(html.matches("socket free res-energy\"").count(), 1);
        assert!(html.contains("Krystallium") && html.contains("piece-name"));

        // With the Krystallium picked, it fits every space.
        let html = in_owner(|| {
            let app = App::new();
            app.games.wonderful.ui.held.set(Some(Held::Krystallium));
            render_with(app, &state, 0)
        });
        assert_eq!(html.matches("socket free res-energy fits").count(), 1);
        assert_eq!(html.matches("socket free res-materials fits").count(), 3);
        assert!(html.contains("aria-pressed=\"true\""));
        // ...but it doesn't go on the Empire.
        assert!(html.contains("Put on your Empire"));
    }

    #[test]
    fn a_character_space_takes_a_character_and_the_science_winner_chooses() {
        let mut state = production_state();
        let card = id("Giant Tower"); // 2 Materials, 3 Gold, 1 Financier
        state.players[0].buildings = vec![wonderful_core::Building { card, filled: Cost::default() }];
        state.players[0].financiers = 1;
        state.players[0].pool = 0;
        state.players[0].ready = true;
        let html = render(&state, 0);
        assert!(html.contains("socket free financier fits"), "{html}");

        state.players[0].choose = true;
        state.players[0].ready = false;
        let html = render(&state, 0);
        assert!(html.contains("Take a General") && html.contains("Take a Financier"));
        assert!(html.contains("Take a character first"));
        assert!(html.contains("disabled"));
        assert!(!render(&state, 1).contains("Take a General"));
    }

    #[test]
    fn the_recycle_dialog_asks_where_the_cube_goes() {
        let mut state = State::new(3, 6);
        while state.phase == Phase::Draft {
            for seat in 0..3 {
                if let Some(a) = next_move(&state, seat) {
                    state.apply(seat, a).unwrap();
                }
            }
        }
        let card = state.players[0].drafted[1];
        let built = state.players[0].drafted[0];
        state.apply(0, Action::Build { card: built }).unwrap();

        // Nothing is asked until a card is chosen.
        assert!(!render(&state, 0).contains("modal-bg"));

        let html = in_owner(|| {
            let app = App::new();
            app.games.wonderful.ui.cube_from.set(Some(CubeFrom::Recycle(card)));
            render_with(app, &state, 0)
        });
        assert!(html.contains("modal-bg") && html.contains("role=\"dialog\""));
        assert!(html.contains(&format!("Recycle {}", card.def().name)) || html.contains("Recycle"));
        assert!(html.contains("your Empire") && html.contains("0 of 5 cubes"));
        assert!(html.contains("Cancel") && !html.contains("is lost"));

        // Scrapping warns about what is lost and only confirms: the cube goes to the Empire.
        let html = in_owner(|| {
            let app = App::new();
            app.games.wonderful.ui.cube_from.set(Some(CubeFrom::Scrap(built)));
            render_with(app, &state, 0)
        });
        assert!(html.contains("Everything placed on this building is lost."));
        assert!(html.contains(&format!("Scrap {}", built.def().name)));
        assert!(html.contains("which goes on your Empire"));
        assert_eq!(html.matches("target-detail").count(), 1, "only the Empire");

        // A choice that no longer applies (the card is gone) shows nothing.
        let html = in_owner(|| {
            let app = App::new();
            app.games.wonderful.ui.cube_from.set(Some(CubeFrom::Recycle(built)));
            render_with(app, &state, 0)
        });
        assert!(!html.contains("modal-bg"));
    }

    #[test]
    fn the_other_players_show_what_they_have_built() {
        let mut state = State::new(4, 12);
        state.players[2].empire = vec![id("Recycling Plant"), id("Zeppelin")];
        state.players[2].buildings = vec![wonderful_core::Building { card: id("Juggernaut"), filled: Cost::default() }];
        state.players[2].krystallium = 2;
        let html = in_owner(|| {
            let n = state.seats();
            game_screen(
                App::new(),
                state.view_for(0),
                names(n),
                vec![true, true, false, true],
                "ABCD".into(),
            )
            .to_html()
        });
        assert_eq!(html.matches("class=\"rival\"").count(), 3);
        assert!(html.contains("Cy") && html.contains("tag tag-off"), "Cy is offline");
        assert_eq!(html.matches("tag tag-off").count(), 1);
        assert!(html.contains("Recycling Plant") && html.contains("Zeppelin") && html.contains("Juggernaut"));
        assert!(html.contains("0/7"), "progress of the Juggernaut");
        assert!(html.contains("deciding"));
    }

    #[test]
    fn nothing_of_the_others_hands_is_drawn() {
        let mut state = State::new(3, 12);
        state.players[0].hand = vec![id("Neuroscience"), id("Atlantis")];
        state.players[1].hand = vec![id("Time Travel"), id("Aquaculture")];
        state.players[2].hand = vec![id("Museum"), id("Zeppelin")];
        state.players[1].picked = Some(id("Casino City"));
        state.players[1].drafted = vec![id("Roswell")];
        let html = render(&state, 0);
        assert!(html.contains("Neuroscience") && html.contains("Atlantis"));
        for hidden in ["Time Travel", "Aquaculture", "Museum", "Zeppelin", "Casino City", "Roswell"] {
            assert!(!html.contains(hidden), "{hidden} leaked");
        }
    }

    #[test]
    fn the_empire_groups_finished_cards_by_type() {
        let mut state = State::new(3, 12);
        state.empires = vec![0, 1, 2]; // Noram States for seat 0
        state.players[0].empire =
            vec![id("Recycling Plant"), id("Nuclear Plant"), id("Zeppelin"), id("Wind Turbines"), id("Neuroscience")];
        let html = render(&state, 0);
        assert!(html.contains("Your Empire") && html.contains("Noram States") && html.contains("1 VP per Financier"));
        assert_eq!(html.matches("class=\"kind-col\"").count(), 3, "Structure, Vehicle and Research");
        assert!(html.contains("Produces each round") && html.contains("You hold"));
        assert!(!html.contains("No finished cards yet."));
        let bare = render(&State::new(3, 12), 0);
        assert!(bare.contains("No finished cards yet."));
    }

    #[test]
    fn every_card_shows_its_picture_with_its_name_over_it() {
        let mut state = State::with_options(3, 1, wonderful_core::Options { ascension: true });
        let (hand, kept, building, finished) = (id("Secret Society"), id("Neuroscience"), id("Juggernaut"), id("Pandora's Box"));
        state.players[0].hand = vec![hand];
        state.players[0].drafted = vec![kept];
        state.players[0].buildings = vec![Building { card: building, filled: Cost::default() }];
        state.players[0].empire = vec![finished];
        let html = render(&state, 0);
        // A picture under every card's header: in the hand, kept, being built, finished.
        assert_eq!(html.matches("class=\"card-art\"").count(), html.matches("class=\"card-head\"").count());
        for card in [hand, kept, building, finished] {
            let def = card.def();
            let at = html.find(&format!("src=\"{}\"", look::card_art(def))).expect(def.name);
            let band = &html[html[..at].rfind("class=\"card-art\"").unwrap()..];
            let band = &band[..band.find("</span>").unwrap() + "</span>".len()];
            assert!(band.contains("alt=\"\"") && band.contains("loading=\"lazy\""), "{band}");
            let named = |name: &str| band.contains(&format!("class=\"card-name\">{name}</span>"));
            assert!(named(def.name) || named(&def.name.replace('\'', "&#39;")), "the name over the picture: {band}");
        }
    }

    #[test]
    fn every_card_has_the_kept_cards_look() {
        // The hand, the pick, the kept cards, the planned ones and the buildings
        // all draw the cost as a column down the left, beside the picture, as
        // the printed card does: the layout is the card's own, not one section's.
        let css = include_str!("wonderful.css");
        assert!(!css.contains(".cards.kept .card-cost"), "the cost column is not only for kept cards");
        for rule in [".wonderful .card .card-cost {", ".wonderful .card .card-cost .sockets {"] {
            assert!(css.contains(rule), "{rule}");
        }
        let card = &css[css.find("/* Every card shows its cost as a column").expect("the shared layout")..];
        assert!(card[..card.find('}').unwrap()].contains("grid-template-columns: auto minmax(0, 1fr)"));
        // The picture's shape is one variable, so that it is easy to resize.
        assert!(css.contains("aspect-ratio: var(--art-ratio"));

        let mut state = State::new(3, 1);
        let (hand, kept, building) = (id("Secret Society"), id("Neuroscience"), id("Juggernaut"));
        state.players[0].hand = vec![hand];
        state.players[0].drafted = vec![kept];
        state.players[0].buildings = vec![Building { card: building, filled: Cost::default() }];
        let html = render(&state, 0);
        // Each card carries its cost between its picture and its rows.
        for card in [hand, kept, building] {
            let name = card.def().name;
            let at = html.find(&format!("class=\"card-name\">{name}</span>")).expect(name);
            let rest = &html[at..];
            let cost = rest.find("class=\"card-cost\"").expect(name);
            assert!(cost < rest.find("class=\"card-rows\"").unwrap(), "{name}");
        }
    }

    #[test]
    fn the_picture_fades_out_under_the_rows() {
        // A player asked for a bigger picture, fading out at its foot, with
        // what the card makes and gives when built over the fade.
        let css = include_str!("wonderful.css");
        // The rule for that selector alone, not one it shares with others.
        let rule = |sel: &str| {
            let at = css
                .match_indices(&format!("{sel} {{"))
                .map(|(at, _)| at)
                .find(|&at| !css[..at].trim_end().ends_with(','))
                .expect(sel);
            css[at..at + css[at..].find('}').unwrap()].to_string()
        };
        assert!(css.contains("aspect-ratio: var(--art-ratio, 4 / 3)"), "a taller picture");
        assert!(rule(".wonderful .card-art img").contains("mask-image: linear-gradient(to bottom"));
        let art = rule(".wonderful .card .card-art");
        let rows = rule(".wonderful .card .card-rows");
        assert!(art.contains("grid-row: 2 / span 2") && rows.contains("grid-row: 3"), "the rows lie over the picture");
        assert!(rows.contains("align-self: end") && rows.contains("text-shadow"), "{rows}");
        assert!(rule(".wonderful .card-art .card-name").contains("inset: 0 0 auto"), "the name over the picture's head");
    }

    #[test]
    fn the_empires_show_their_pictures() {
        let mut state = State::new(3, 12);
        state.empires = vec![0, 1, 2];
        let html = render(&state, 0);
        // Yours: its board's picture behind the heading, and its crest.
        let head = &html[html.find("class=\"empire-head\"").unwrap()..];
        let head = &head[..head.find("empire-stats").unwrap()];
        assert!(head.contains(&format!("src=\"{}\"", look::empire_art(0))), "{head}");
        assert!(head.contains(&format!("src=\"{}\"", look::crest_art(0))), "{head}");
        assert!(head.contains("Noram States") && head.contains("1 VP per Financier"));
        // The others': a crest by their Empire's name.
        for (seat, name) in [(1, "Bob"), (2, "Cy")] {
            let h3 = html.find(&format!("<h3>{name}")).unwrap();
            let start = html[..h3].rfind("class=\"rival\"").unwrap();
            let rival = &html[start..start + html[start..].find("</li>").unwrap()];
            assert!(rival.contains(&format!("src=\"{}\"", look::crest_art(seat))), "{rival}");
        }
        assert!(!html.contains(&look::empire_art(1)), "only your own board's picture");
    }

    #[test]
    fn the_log_lists_what_happened_newest_first() {
        let mut state = State::new(3, 4);
        let fresh = render(&state, 0);
        assert!(fresh.contains("Round 1 of 4 begins."));
        state.log = vec![
            wonderful_core::Event::RoundStarted(1),
            wonderful_core::Event::Completed { seat: 1, card: id("Zeppelin") },
            wonderful_core::Event::Supremacy { res: Res::Gold, seats: vec![0] },
        ];
        let html = render(&state, 0);
        let newest = html.find("You produced the most Gold").unwrap();
        let older = html.find("Bob finished Zeppelin.").unwrap();
        assert!(newest < older);
        // Only the latest twelve lines are listed.
        state.log = (0..24).map(|_| wonderful_core::Event::RoundStarted(1)).collect();
        assert_eq!(render(&state, 0).matches("begins.").count(), 12);
    }

    #[test]
    fn every_card_of_the_catalogue_can_be_drawn() {
        let mut state = State::new(3, 1);
        state.players[0].hand = (0..catalogue().len() as u16).map(CardId).collect();
        let html = render(&state, 0);
        for card in catalogue() {
            assert!(html.contains(card.name.replace('\'', "&#39;").as_str()) || html.contains(card.name), "{}", card.name);
        }
        for kind in Kind::ALL {
            assert!(html.contains(kind.name()));
        }
    }

    #[test]
    fn player_names_are_escaped() {
        let state = State::new(2, 3);
        let html = in_owner(|| {
            game_screen(
                App::new(),
                state.view_for(0),
                vec!["<b>Ann</b>".into(), "Bob & co".into()],
                vec![true, true],
                "ABCD".into(),
            )
            .to_html()
        });
        assert!(!html.contains("<b>Ann</b>"), "an unescaped name");
        assert!(html.contains("Bob &amp; co"));
    }

    // ----- Corruption & Ascension ---------------------------------------------------

    #[test]
    fn the_expansions_cards_carry_its_tag_and_their_corruption() {
        let mut state = State::with_options(3, 1, wonderful_core::Options { ascension: true });
        state.players[0].hand = vec![id("Border Patrol"), id("Immortality"), id("Zeppelin")];
        let html = render(&state, 0);
        // Two of the three are the expansion's.
        assert_eq!(html.matches("class=\"card-deck\"").count(), 2, "{html}");
        // Border Patrol makes a Gold and corrupts an Exploration.
        assert!(html.contains("cube corrupt res-exploration"));
        assert!(html.contains("9 VP per Research + Project"));
    }

    #[test]
    fn cubes_and_worth_show_the_card_type_symbols() {
        let mut state = State::with_options(3, 1, wonderful_core::Options { ascension: true });
        state.players[0].hand = vec![id("Border Patrol"), id("Immortality")];
        let html = render(&state, 0);
        // Resource cubes carry the symbol of the card type of their colour, no letter.
        let gold = html.find("cube res-gold\"").expect("a Gold cube");
        let gold = &html[gold..gold + html[gold..].find("</span>").unwrap()];
        assert!(gold.contains("kind-icon"), "{gold}");
        assert!(!gold.contains(">G<"), "{gold}");
        // Worth draws the types as named symbols, with the words in its tooltip.
        let worth = html.find("title=\"9 VP per Research + Project\" class=\"worth-body\"").expect("Immortality's worth");
        let worth = &html[worth..worth + html[worth..].find("class=\"row-label\"").unwrap_or(html.len() - worth)];
        for kind in [Kind::Research, Kind::Project] {
            assert!(worth.contains(&format!("aria-label=\"{}\"", kind.name())), "{worth}");
            assert!(worth.contains(&format!("kind-badge {}", look::kind_class(kind))), "{worth}");
        }
    }

    #[test]
    fn generals_and_financiers_wear_the_printed_token_over_their_letter() {
        let mut state = State::new(2, 1);
        state.players[0].hand = vec![id("Giant Tower")]; // 1 Financier among its costs
        let html = render(&state, 0);
        let img = |token| format!("<img src=\"{}\" alt=\"\" onerror=\"this.remove()\" class=\"token-art\">", look::token_art(token));
        // The cost's space: the letter stays underneath, for when the picture fails.
        let space = html.find("socket req financier\"").expect("a Financier space");
        let space = &html[space..space + html[space..].find("</span>").unwrap()];
        assert!(space.contains("Fin") && space.contains(&img(Token::Financier)), "{space}");
        // Every player's tally of Generals, and the three at stake on the round's track.
        assert_eq!(html.matches(&img(Token::General)).count(), 2 + 3, "{html}");
    }

    #[test]
    fn every_cards_header_shows_what_recycling_it_gives() {
        for ascension in [false, true] {
            let mut state = State::with_options(3, 1, wonderful_core::Options { ascension });
            let (hand, kept, building, finished) =
                ([id("Zeppelin"), id("Border Patrol")], id("Neuroscience"), id("Juggernaut"), id("Recycling Plant"));
            state.players[0].hand = hand.to_vec();
            state.players[0].drafted = vec![kept];
            state.players[0].buildings = vec![wonderful_core::Building { card: building, filled: Cost::default() }];
            state.players[0].empire = vec![finished];
            let html = render(&state, 0);
            for card in [hand[0], hand[1], kept, building, finished] {
                let def = card.def();
                let name = html.find(&format!("class=\"card-name\">{}<", def.name)).expect(def.name);
                let head = &html[html[..name].rfind("class=\"card-head\"").unwrap()..name];
                let res = def.recycle.name();
                let article = if res.starts_with(['A', 'E', 'I', 'O', 'U']) { "an" } else { "a" };
                assert!(head.contains(&format!("title=\"Recycles into {article} {res} cube\"")), "{}: {head}", def.name);
                assert!(head.contains(&format!("cube {}", look::res_class(def.recycle))), "{}: {head}", def.name);
                // Next to the expansion's mark, when the card has one.
                assert_eq!(head.contains("card-deck"), def.ascension, "{}", def.name);
            }
            // The header says it all: no line at the foot of the card repeats it.
            assert!(!html.contains("Recycles to"));
        }
    }

    #[test]
    fn the_round_ring_shows_which_way_the_hands_go() {
        let mut state = State::new(3, 2);
        let html = render(&state, 0);
        assert!(html.contains("class=\"round-ring next\""));
        assert!(html.contains(&look::round_art(true)));
        assert!(html.contains("Round 1 of 4, cards pass to the next player"));
        state.round = 2;
        let html = render(&state, 0);
        assert!(html.contains("class=\"round-ring previous\""));
        assert!(html.contains(&look::round_art(false)) && !html.contains(&look::round_art(true)));
    }

    #[test]
    fn the_rounds_track_sits_beside_the_ring_with_a_cursor_on_the_step_played() {
        let current = |state: &State| {
            let html = render(state, 0);
            let track = &html[html.find("class=\"track\"").expect("a track")..];
            let track = &track[..track.find("</ol>").unwrap()];
            assert!(html.find("class=\"round\"").unwrap() < html.find("class=\"track\"").unwrap());
            assert!(html.find("class=\"track\"").unwrap() < html.find("class=\"phase\"").unwrap());
            assert_eq!(track.matches("class=\"track-step").count(), 8, "{track}");
            assert_eq!(track.matches("aria-current=\"step\"").count(), 1, "{track}");
            let now = track.find("track-step now").expect("a cursor");
            (track.matches("track-step done").count(), track[now..].split("</li>").next().unwrap().to_string(), html)
        };
        let mut state = production_state();
        state.phase = Phase::Draft;
        let (done, now, _) = current(&state);
        assert_eq!(done, 0);
        assert!(now.contains(">Draft<"), "{now}");
        state.phase = Phase::Planning;
        assert!(current(&state).1.contains(">Planning<"));
        state.phase = Phase::Production { step: 3 };
        let (done, now, html) = current(&state);
        assert_eq!(done, 5);
        assert!(now.contains("res-gold") && now.contains("Most Gold: a Financier"), "{now}");
        // The production panel no longer lists the steps a second time.
        assert!(!html.contains("class=\"steps\"") && !html.contains("class=\"step "));
    }

    #[test]
    fn the_production_steps_show_the_character_at_stake_then_who_took_it() {
        let awards = |state: &State| {
            let html = render(state, 0);
            let at = |class: &str| html.matches(&format!("class=\"step-award{class}\"")).count();
            (at(""), at(" won"), at(" nobody"), html)
        };
        let mut state = production_state();
        // Nothing played yet: five resources, each with its character, none taken.
        state.awards.clear();
        let (open, won, nobody, html) = awards(&state);
        assert_eq!((open, won, nobody), (5, 0, 0));
        assert!(html.contains("title=\"Most Materials: a Financier\""));
        assert!(html.contains("title=\"Most Energy: a General\""));
        assert!(html.contains("title=\"Most Science: a General or a Financier\""));
        assert!(html.contains("title=\"Most Exploration: a General\""));
        // Three steps played: Materials to us, Energy tied, Science to seats 1 and 2.
        state.phase = Phase::Production { step: 3 };
        state.awards = vec![vec![0], vec![], vec![1, 2]];
        let (open, won, nobody, html) = awards(&state);
        assert_eq!((open, won, nobody), (2, 2, 1));
        assert!(html.contains("title=\"a Financier: You\""));
        assert!(html.contains("<span class=\"award-who\">nobody</span>"));
        assert!(html.contains("a General or a Financier: Bob &amp; Cy"));
    }

    #[test]
    fn from_six_players_the_race_has_two_winners() {
        let ascension = wonderful_core::Options { ascension: true };
        let mut state = State::with_options(6, 3, ascension);
        // Straight to a production step, with made-up numbers.
        state.phase = Phase::Production { step: 0 };
        for (p, made) in state.players.iter_mut().zip([4, 1, 3, 0, 2, 0]) {
            p.produced = made;
            p.pool = made;
        }
        let html = render(&state, 0);
        assert_eq!(html.matches("race-row lead").count(), 2, "seats 0 and 2");
        assert_eq!(html.matches("class=\"race-prize\"").count(), 2);
        assert!(!html.contains("tied: no character"));
    }

    #[test]
    fn whole_games_with_the_expansion_can_be_drawn_for_six_and_seven() {
        let ascension = wonderful_core::Options { ascension: true };
        for players in [6, 7] {
            let mut state = State::with_options(players, 50 + players as u64, ascension);
            let moves = play_and_draw(&mut state, true, |state, seat, html| {
                assert!(html.contains("Your Empire") && html.contains(look::empire_name(state.empires[seat])));
                if state.phase == Phase::Over {
                    assert!(html.contains("standings"));
                }
            });
            assert!(moves > 100, "{players} players: only {moves} moves");
            assert_eq!(state.phase, Phase::Over, "{players} players");
        }
    }
}
