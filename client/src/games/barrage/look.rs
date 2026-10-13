//! The pure functions behind the Barrage screens: names and texts, the
//! companies' colours, what each action space does, and what a click on the
//! map means. Native tests at the bottom.

use barrage_core::{
    advanced_effect, production_energy, Action, Build, Company, Event, Kind, Officer, Phase, Pick, Production,
    ProductionOption, Reward, Section, Spot, Target, Task, Tile, Todo, View, Why, BASINS, CONDUITS, CONTRACTS,
    POWERHOUSES,
};

/// The colour of a company's pieces, and the ink that reads on it.
pub fn company_colors(company: Company) -> (&'static str, &'static str) {
    match company {
        Company::Usa => ("#c8323c", "#fff6ef"),
        Company::Germany => ("#2b2a2e", "#f4efe6"),
        Company::Italy => ("#2f8f4e", "#f4fff6"),
        Company::France => ("#f1ede3", "#2b2a2e"),
    }
}

/// A seat's colour: its company's once chosen.
pub fn seat_colors(view: &View, seat: usize) -> (&'static str, &'static str) {
    match view.players.get(seat).and_then(|p| p.company) {
        Some(c) => company_colors(c),
        None => ("#8a7a5c", "#fff"),
    }
}

/// The printed map board's picture (see docs/barrage.md, "The pictures").
pub const BOARD_ART: &str = "/barrage-art/board.webp";

/// A piece's picture, as the rulebook draws it: a company's, or a neutral
/// Dam's (`None`).
pub fn piece_art(company: Option<Company>, kind: Kind) -> String {
    let owner = company.map(company_slug).unwrap_or("neutral");
    format!("/barrage-art/pieces/{owner}-{}.webp", kind.name().to_lowercase())
}

/// A company's Engineer figure.
pub fn engineer_art(company: Company) -> String {
    format!("/barrage-art/pieces/{}-engineer.webp", company_slug(company))
}

fn company_slug(company: Company) -> &'static str {
    match company {
        Company::Usa => "usa",
        Company::Germany => "germany",
        Company::Italy => "italy",
        Company::France => "france",
    }
}

/// A token's picture: `drop`, `credit`, `excavator`, `mixer`.
pub fn token_art(token: &str) -> String {
    format!("/barrage-art/pieces/{token}.webp")
}

/// A neutral dam's wood.
pub const NEUTRAL: &str = "#c9a46a";

/// The colour of a conduit's pipe by its value, as printed: grey, green,
/// yellow, orange, red.
pub fn value_color(value: u8) -> &'static str {
    match value {
        1 => "#9a9a9a",
        2 => "#3fae4a",
        3 => "#e3c429",
        4 => "#ee7d22",
        _ => "#d4312c",
    }
}

/// "You" for our own seat, the seat's name otherwise.
pub fn who(seat: usize, names: &[String], you: usize) -> String {
    if seat == you {
        "You".to_string()
    } else {
        name(seat, names)
    }
}

/// A seat's own name, ours included.
pub fn name(seat: usize, names: &[String]) -> String {
    names.get(seat).cloned().unwrap_or_else(|| format!("Player {}", seat + 1))
}

fn plural(n: impl Into<u64>) -> &'static str {
    if n.into() == 1 {
        ""
    } else {
        "s"
    }
}

pub fn basin_name(basin: usize) -> &'static str {
    BASINS[basin].name
}

/// Where a piece stands, in words.
pub fn target_text(target: Target) -> String {
    let b = basin_name(target.basin());
    match target {
        Target::Base(d) | Target::Elevation(d) => {
            format!("{b}, {} dam space", if d % 2 == 0 { "upper (red)" } else { "lower" })
        }
        Target::Conduit(c) => format!("{b}, Conduit {} to {}", CONDUITS[c].value, basin_name(CONDUITS[c].to)),
        Target::Powerhouse(p) => format!("{b}{}", if POWERHOUSES[p].paid { ", red space" } else { "" }),
    }
}

pub fn rewards_text(rewards: &[Reward]) -> String {
    rewards.iter().map(|r| r.text()).collect::<Vec<_>>().join(", ")
}

/// A Contract in words: "5 energy: 4 VP, 2 Excavators".
pub fn contract_text(id: u8) -> String {
    let c = CONTRACTS[id as usize];
    format!("{} energy: {}", c.energy, rewards_text(c.rewards))
}

/// The CSS class of a Contract's colour.
pub fn contract_class(id: u8) -> &'static str {
    match CONTRACTS[id as usize].tier {
        barrage_core::Tier::Starting => "contract starting",
        barrage_core::Tier::Green => "contract green",
        barrage_core::Tier::Yellow => "contract yellow",
        barrage_core::Tier::Red => "contract red",
        barrage_core::Tier::National => "contract national",
    }
}

/// A tile's name and, for an Advanced one, what it does.
pub fn tile_text(tile: Tile) -> String {
    match tile {
        Tile::Advanced(id) => format!("{}: {}", tile.name(), advanced_effect(id)),
        Tile::Krylov => "Krylov's tile: copies a tile on your wheel, special effect included.".into(),
        Tile::Basic(k) => format!("{}: builds {}.", tile.name(), k.plural()),
    }
}

/// What an action space does, for its label.
pub fn spot_label(view: &View, spot: Spot) -> String {
    let r = spot.row as usize;
    match spot.section {
        Section::Turbine => {
            let bonus = barrage_core::TURBINE_BONUS[r];
            match bonus {
                0 => "Produce".into(),
                b if b > 0 => format!("Produce +{b}"),
                b => format!("Produce \u{2212}{}", -b),
            }
        }
        Section::Water => ["Place 2 drops on Headstreams", "Place 1 drop that flows now"][r.min(1)].into(),
        Section::Workshop => ["Turn your wheel 1", "2 Credits: turn your wheel 2", "5 Credits: turn your wheel 3"][r.min(2)].into(),
        Section::Shop => {
            ["2 Credits: 1 Excavator", "4 Credits: 1 Machinery of your choice", "5 Credits: 1 Excavator and 1 Concrete Mixer"]
                [r.min(2)]
            .into()
        }
        Section::Contracts => ["Take 1 Contract", "Take 1 Contract", "1 Credit: take 2 Contracts"][r.min(2)].into(),
        Section::Patent => match view.patents.get(r).copied().flatten() {
            Some(t) => format!("5 Credits: {}", Tile::Advanced(t).name()),
            None => "Patent Office (sold)".into(),
        },
        Section::Bank => "1 Credit per Engineer".into(),
        Section::Sekibo => "Copy an Executive Officer".into(),
    }
}

/// The action a click on a space that needs nothing more sends (`None`:
/// the rest of the move is chosen first).
pub fn spot_action(spot: Spot) -> Option<Action> {
    match spot.section {
        Section::Workshop => Some(Action::Workshop { space: spot }),
        Section::Shop if spot.row != 1 => Some(Action::Buy { space: spot, mixer: false }),
        Section::Patent => Some(Action::Patent { slot: spot.row }),
        _ => None,
    }
}

/// Drops a water placement takes now: for the space picked, or a reward.
pub fn water_wanted(view: &View, spot: Option<Spot>) -> Option<(u8, bool)> {
    if let Some((n, now)) = view.moves.water {
        return Some((n, now));
    }
    match spot {
        Some(s) if s.section == Section::Water => Some(if s.row == 0 { (2, false) } else { (1, true) }),
        _ => None,
    }
}

/// The ways to take `n` Machineries of your choice: Excavators first.
pub fn machinery_splits(n: u8) -> Vec<(u8, u8)> {
    (0..=n).rev().map(|e| (e, n - e)).collect()
}

pub fn splits_text(excavators: u8, mixers: u8) -> String {
    match (excavators, mixers) {
        (e, 0) => format!("{e} Excavator{}", plural(e)),
        (0, m) => format!("{m} Concrete Mixer{}", plural(m)),
        (e, m) => format!("{e} Excavator{} + {m} Mixer{}", plural(e), plural(m)),
    }
}

/// The energy a production option makes with `drops`, with what the
/// Turbine row adds (`row`: `None` for a production in the todo list).
pub fn option_energy(view: &View, o: &ProductionOption, drops: u8, row: Option<u8>) -> i32 {
    let row_bonus = row.map_or(0, |r| barrage_core::TURBINE_BONUS[r as usize]);
    production_energy(drops, o.value, view.moves.fiesler, view.moves.bonus + row_bonus)
}

/// Credits left for a production once the space is paid for.
pub fn credits_after(view: &View, spot: Option<Spot>) -> u32 {
    let credits = view.players[view.you].credits;
    let fee = spot.and_then(|s| s.def()).map_or(0, |(d, _)| if d.fee { 3 } else { 0 });
    credits.saturating_sub(fee)
}

/// A production option in words.
pub fn option_text(view: &View, names: &[String], o: &ProductionOption) -> String {
    let dam = view.dams[o.dam].expect("a dam");
    let owner = match dam.owner {
        None => "neutral Dam".to_string(),
        Some(_) => "your Dam".to_string(),
    };
    let conduit = match view.conduits[o.conduit] {
        Some(s) if s == view.you => format!("your Conduit {}", o.value),
        Some(s) => format!("{}'s Conduit {} (1 Credit a drop)", name(s, names), o.value),
        None => "a Conduit".into(),
    };
    format!(
        "{} {owner} ({} drop{}) \u{2192} {conduit} \u{2192} Powerhouse in {}",
        basin_name(barrage_core::dam_basin(o.dam)),
        dam.water,
        plural(dam.water),
        basin_name(POWERHOUSES[o.powerhouse].basin)
    )
}

/// The production a click sends.
pub fn production(o: &ProductionOption, drops: u8) -> Production {
    Production { powerhouse: o.powerhouse, conduit: o.conduit, dam: o.dam, drops }
}

/// The tiles that can build something now, with the tile copied (Anton
/// Krylov), each once, in the order of the supply.
pub fn tile_choices(view: &View) -> Vec<(Option<Tile>, Option<Tile>)> {
    let mut out: Vec<(Option<Tile>, Option<Tile>)> = Vec::new();
    for o in &view.moves.builds {
        let key = (o.build.tile, o.build.copy);
        if !out.contains(&key) {
            out.push(key);
        }
    }
    out
}

/// The name of a tile choice: "Base", "Krylov's tile as Conduit II".
pub fn choice_name(tile: Option<Tile>, copy: Option<Tile>) -> String {
    match (tile, copy) {
        (Some(t), None) => t.name(),
        (Some(t), Some(c)) => format!("{} as {}", t.name(), c.name()),
        (None, Some(c)) => format!("Copy of {}", c.name()),
        (None, None) => String::new(),
    }
}

/// The construction a click on `target` makes with the tile chosen, paying
/// a Conduit with Concrete Mixers if `mixers` (and that's possible).
pub fn build_for(view: &View, choice: (Option<Tile>, Option<Tile>), mixers: bool, target: Target) -> Option<Build> {
    let fits = |o: &&barrage_core::BuildOption| (o.build.tile, o.build.copy) == choice && o.build.target == target;
    let options: Vec<_> = view.moves.builds.iter().filter(fits).collect();
    options.iter().find(|o| o.build.mixers == mixers).or(options.first()).map(|o| o.build)
}

/// What a click on a piece of the map does now: a free building, or a
/// construction with the tile in hand.
pub fn map_click(view: &View, choice: Option<(Option<Tile>, Option<Tile>)>, mixers: bool, target: Target) -> Option<Action> {
    if view.moves.places.contains(&target) {
        return Some(Action::Place { target });
    }
    build_for(view, choice?, mixers, target).map(Action::Build)
}

/// The targets a click can act on now, with what each costs in words.
pub fn targets(view: &View, choice: Option<(Option<Tile>, Option<Tile>)>, mixers: bool) -> Vec<(Target, String)> {
    if !view.moves.places.is_empty() {
        return view.moves.places.iter().map(|&t| (t, if t.paid() { "3\u{a2}".into() } else { "free".into() })).collect();
    }
    let Some(choice) = choice else { return Vec::new() };
    let mut out: Vec<(Target, String)> = Vec::new();
    for o in &view.moves.builds {
        if (o.build.tile, o.build.copy) != choice || out.iter().any(|(t, _)| *t == o.build.target) {
            continue;
        }
        let b = build_for(view, choice, mixers, o.build.target).unwrap_or(o.build);
        let cost = view.moves.builds.iter().find(|x| x.build == b).map_or(o.cost, |x| x.cost);
        out.push((o.build.target, cost_text(cost.excavators, cost.mixers, cost.credits)));
    }
    out
}

/// "4E 2M 3¢", short, for the map.
pub fn cost_text(excavators: u32, mixers: u32, credits: u32) -> String {
    let mut parts = Vec::new();
    if excavators > 0 {
        parts.push(format!("{excavators}E"));
    }
    if mixers > 0 {
        parts.push(format!("{mixers}M"));
    }
    if credits > 0 {
        parts.push(format!("{credits}\u{a2}"));
    }
    if parts.is_empty() {
        "free".into()
    } else {
        parts.join(" ")
    }
}

/// What a waiting decision is about.
pub fn todo_text(todo: &Todo) -> String {
    match todo.task {
        Task::Reward(r) => match r {
            Reward::Machinery(n) => format!("take {n} Machiner{} of their choice", if n == 1 { "y" } else { "ies" }),
            Reward::Water(n) => format!("place {n} drop{} on the Headstreams", plural(n)),
            Reward::WaterNow(n) => format!("place {n} drop{} that flow now", plural(n)),
            other => format!("build {}", other.text().trim_start_matches("a free ")),
        },
        Task::Contract { energy } => format!("fulfill a Contract with {energy} energy"),
        Task::Discard => "discard a Contract".into(),
        Task::Produce { plain: true, .. } => "produce a second time".into(),
        Task::Produce { .. } => "produce with a +2 bonus".into(),
    }
}

/// The title of the status band.
pub fn phase_title(view: &View, names: &[String]) -> String {
    match view.phase {
        Phase::Setup => match view.actor {
            Some(s) if s == view.you => "You pick your company".into(),
            Some(s) => format!("{} picks a company", name(s, names)),
            None => "Setup".into(),
        },
        Phase::Over => match view.winners.as_slice() {
            [w] if *w == view.you => "You win!".into(),
            [w] => format!("{} wins", name(*w, names)),
            _ => "A shared win".into(),
        },
        _ => match view.actor {
            Some(s) if s == view.you => "Your move".into(),
            Some(s) => format!("{}'s move", name(s, names)),
            None => String::new(),
        },
    }
}

/// The hint under the title.
pub fn hint(view: &View, names: &[String]) -> String {
    let you = view.actor == Some(view.you);
    match (view.phase, &view.todo) {
        (Phase::Setup, _) if you => {
            "Pick a Company with its Executive Officer, and a Starting Contract. The first to pick plays last.".into()
        }
        (Phase::Setup, _) => "Companies and Starting Contracts are picked in seat order.".into(),
        (Phase::Over, _) => "The game is over: the most VP win, then the most energy in the last round.".into(),
        (_, Some(t)) if you => format!("Now {}.", todo_text(t).replace("their", "your")),
        (_, Some(t)) => format!("{} has to {}.", name(t.seat, names), todo_text(t)),
        (_, None) if you => {
            let p = &view.players[view.you];
            if p.engineers == 0 {
                "You have no Engineers left: pass, or build with a tile that needs none.".into()
            } else {
                "Place Engineers: build with a Technology tile on your Company board, or take a space of the Management board.".into()
            }
        }
        _ => "Each turn, a player places Engineers on one action space.".into(),
    }
}

fn why_text(why: Why) -> &'static str {
    match why {
        Why::FirstEnergy => "the most energy",
        Why::SecondEnergy => "the second most energy",
        Why::NoEnergy => "producing nothing",
        Why::Bonus => "the Bonus tile",
        Why::Objective => "the Objective",
        Why::Resources => "Credits and Machineries left",
        Why::Water => "water in their Dams",
        Why::Tile => "an Advanced tile",
    }
}

/// One line of the log.
pub fn describe_event(event: &Event, names: &[String], you: usize) -> String {
    let w = |s: usize| who(s, names, you);
    let their = |s: usize| if s == you { "your" } else { "their" };
    match event {
        Event::Round(r) => format!("Round {r} begins."),
        Event::Chose { seat, company, officer, contract } => format!(
            "{} took {} with {}, and the Starting Contract {}.",
            w(*seat),
            company.name(),
            officer.name(),
            contract_text(*contract)
        ),
        Event::Built { seat, target, tile } => {
            let with = tile.map_or(" (a reward)".to_string(), |t| format!(" with {}", t.name()));
            format!("{} built a {} \u{2014} {}{with}.", w(*seat), target.kind().name(), target_text(*target))
        }
        Event::Produced { seat, production, energy, paid_to } => {
            let paid = paid_to.map_or(String::new(), |o| format!(", paying {}", w(o)));
            format!(
                "{} produced {energy} energy: {} drop{} from {} to {}{paid}.",
                w(*seat),
                production.drops,
                plural(production.drops),
                basin_name(barrage_core::dam_basin(production.dam)),
                basin_name(POWERHOUSES[production.powerhouse].basin)
            )
        }
        Event::Generated { seat, energy } => format!("{} produced {energy} energy with a tile.", w(*seat)),
        Event::Fulfilled { seat, contract } => format!("{} fulfilled a Contract ({}).", w(*seat), contract_text(*contract)),
        Event::TookContracts { seat, contracts } => {
            let list: Vec<String> = contracts.iter().map(|&c| contract_text(c)).collect();
            format!("{} took {}.", w(*seat), list.join(" and "))
        }
        Event::Discarded { seat, contract } => format!("{} discarded {}.", w(*seat), contract_text(*contract)),
        Event::Watered { seat, drops, now } => {
            let n: u8 = drops.iter().sum();
            let at: Vec<String> =
                drops.iter().enumerate().filter(|(_, &d)| d > 0).map(|(h, &d)| format!("{d} on {}", ["A", "B", "C", "D"][h])).collect();
            let when = if *now { ", flowing at once" } else { "" };
            format!("{} placed {n} drop{} ({}){when}.", w(*seat), plural(n), at.join(", "))
        }
        Event::Bank { seat, credits } => format!("{} took {credits} Credit{} at the Bank.", w(*seat), plural(*credits)),
        Event::Rotated { seat, turns } => format!("{} turned {} wheel {turns}.", w(*seat), their(*seat)),
        Event::Bought { seat, excavators, mixers } => {
            format!("{} got {}.", w(*seat), splits_text(*excavators as u8, *mixers as u8))
        }
        Event::Patent { seat, tile } => format!("{} bought {}.", w(*seat), tile.name()),
        Event::Copied { seat, officer } => format!("{} copied {} for the turn.", w(*seat), officer.name()),
        Event::Passed { seat } => format!("{} passed.", w(*seat)),
        Event::Gained { seat, reward } => format!("{} got {}.", w(*seat), reward.text()),
        Event::Flowed { seat, energy } => {
            format!("{} got {energy} energy from water through {} Powerhouses.", w(*seat), their(*seat))
        }
        Event::WaterFlow => "The Headstreams' water flowed down.".into(),
        Event::Scored { seat, vp, why } if *vp < 0 => {
            format!("{} lost {} VP for {}.", w(*seat), -vp, why_text(*why).replace("their", their(*seat)))
        }
        Event::Scored { seat, vp, why } => {
            format!("{} scored {vp} VP for {}.", w(*seat), why_text(*why).replace("their", their(*seat)))
        }
        Event::Credits { seat, credits } => format!("{} earned {credits} Credits from the Energy Track.", w(*seat)),
        Event::GameOver => "The game is over.".into(),
    }
}

/// The seats, best first: VP, then energy in the last round.
pub fn standings(view: &View) -> Vec<usize> {
    let mut seats: Vec<usize> = (0..view.players.len()).collect();
    seats.sort_by_key(|&s| std::cmp::Reverse((view.players[s].vp, view.players[s].energy)));
    seats
}

/// Engineers the next construction takes from the Company board, and its
/// fee (`None`: no space left this round).
pub fn next_construction(view: &View, seat: usize) -> Option<(u8, bool)> {
    let c = view.players[seat].constructions as usize;
    barrage_core::CONSTRUCTION.get(c).map(|&e| (e, c == 3))
}

/// The officer abilities a Mahiri Sekibo can copy, in words.
pub fn officer_line(officer: Officer) -> String {
    format!("{}: {}", officer.name(), officer.ability())
}

/// Whether the picks for the Contract Office are complete for `spot`.
pub fn picks_ready(spot: Spot, picks: &[Pick]) -> bool {
    let most = barrage_core::CONTRACTS_TAKEN[spot.row as usize];
    !picks.is_empty() && picks.len() <= most
}

/// Toggles a face-up Contract in the picks, keeping at most `most`.
pub fn toggle_pick(picks: &[Pick], pick: Pick, most: usize) -> Vec<Pick> {
    if picks.contains(&pick) {
        picks.iter().copied().filter(|&p| p != pick).collect()
    } else if picks.len() < most {
        let mut out = picks.to_vec();
        out.push(pick);
        out
    } else {
        let mut out = picks[1..].to_vec();
        out.push(pick);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use barrage_core::{Dam, State, Target};

    fn names() -> Vec<String> {
        ["Ann", "Bob", "Cy", "Di"].iter().map(|s| s.to_string()).collect()
    }

    fn started(players: usize, seed: u64) -> State {
        let mut s = State::new(players, seed);
        for seat in 0..players {
            s.apply(seat, Action::Choose { pair: seat, contract: seat }).unwrap();
        }
        s
    }

    #[test]
    fn spaces_say_what_they_do() {
        let s = started(3, 1);
        let v = s.view_for(0);
        assert_eq!(spot_label(&v, Spot::new(Section::Turbine, 0, false)), "Produce +2");
        assert_eq!(spot_label(&v, Spot::new(Section::Turbine, 2, true)), "Produce");
        assert_eq!(spot_label(&v, Spot::new(Section::Turbine, 4, false)), "Produce \u{2212}2");
        assert!(spot_label(&v, Spot::new(Section::Patent, 1, false)).starts_with("5 Credits: "));
        assert_eq!(spot_action(Spot::new(Section::Shop, 1, true)), None, "a choice to make first");
        assert!(spot_action(Spot::new(Section::Shop, 0, true)).is_some());
        assert_eq!(spot_action(Spot::new(Section::Patent, 2, false)), Some(Action::Patent { slot: 2 }));
        assert_eq!(spot_action(Spot::new(Section::Turbine, 2, false)), None);
    }

    #[test]
    fn pieces_have_their_pictures() {
        assert_eq!(piece_art(Some(Company::Usa), Kind::Base), "/barrage-art/pieces/usa-base.webp");
        assert_eq!(piece_art(Some(Company::France), Kind::Powerhouse), "/barrage-art/pieces/france-powerhouse.webp");
        assert_eq!(piece_art(None, Kind::Elevation), "/barrage-art/pieces/neutral-elevation.webp");
        assert_eq!(token_art("drop"), "/barrage-art/pieces/drop.webp");
        assert_eq!(engineer_art(Company::Italy), "/barrage-art/pieces/italy-engineer.webp");
        assert_eq!(BOARD_ART, "/barrage-art/board.webp");
    }

    #[test]
    fn machinery_can_be_taken_any_way() {
        assert_eq!(machinery_splits(2), [(2, 0), (1, 1), (0, 2)]);
        assert_eq!(splits_text(1, 1), "1 Excavator + 1 Mixer");
        assert_eq!(splits_text(0, 3), "3 Concrete Mixers");
    }

    #[test]
    fn a_click_on_the_map_builds_with_the_tile_in_hand() {
        let s = started(2, 2);
        let seat = s.actor().unwrap();
        let v = s.view_for(seat);
        let choices = tile_choices(&v);
        assert!(choices.contains(&(Some(Tile::Basic(Kind::Base)), None)));
        let base = (Some(Tile::Basic(Kind::Base)), None);
        let free = (0..20).find(|&d| v.dams[d].is_none() && d % 2 == 1 && BASINS[d / 2].area == barrage_core::Area::Plains).unwrap();
        let target = Target::Base(free);
        let action = map_click(&v, Some(base), false, target).unwrap();
        assert!(matches!(action, Action::Build(b) if b.target == target));
        assert_eq!(map_click(&v, None, false, target), None);
        let marked = targets(&v, Some(base), false);
        assert!(marked.iter().any(|(t, cost)| *t == target && cost == "3E"));
        // Red spaces cost the 3 Credits too.
        assert!(marked.iter().any(|(t, cost)| *t == Target::Base(free - 1) && cost == "3E 3\u{a2}"));
    }

    #[test]
    fn productions_are_described_with_their_energy() {
        let mut s = started(2, 3);
        let seat = s.actor().unwrap();
        s.dams = vec![None; 20];
        s.dams[1] = Some(Dam { owner: None, level: 1, water: 1 });
        s.conduits[0] = Some(1 - seat);
        s.powerhouses[6] = Some(seat);
        let v = s.view_for(seat);
        let o = v.moves.productions[0];
        assert_eq!(option_energy(&v, &o, 1, Some(0)), 7);
        assert_eq!(option_energy(&v, &o, 1, None), 5);
        let text = option_text(&v, &names(), &o);
        assert!(text.contains("M1 neutral Dam (1 drop)") && text.contains("Conduit 5 (1 Credit a drop)") && text.contains("P1"), "{text}");
    }

    #[test]
    fn contract_picks_keep_the_latest() {
        let a = Pick { tier: 0, slot: 0 };
        let b = Pick { tier: 1, slot: 1 };
        let c = Pick { tier: 2, slot: 0 };
        assert_eq!(toggle_pick(&[], a, 1), [a]);
        assert_eq!(toggle_pick(&[a], b, 1), [b]);
        assert_eq!(toggle_pick(&[a], b, 2), [a, b]);
        assert_eq!(toggle_pick(&[a, b], c, 2), [b, c]);
        assert_eq!(toggle_pick(&[a, b], a, 2), [b]);
        assert!(picks_ready(Spot::new(Section::Contracts, 2, false), &[a, b]));
        assert!(!picks_ready(Spot::new(Section::Contracts, 0, false), &[a, b]));
    }

    #[test]
    fn every_event_reads() {
        let s = started(4, 4);
        for e in &s.log {
            let line = describe_event(e, &names(), 0);
            assert!(line.ends_with('.') || line.ends_with('!'), "{line}");
        }
        let line = describe_event(&Event::Scored { seat: 1, vp: -3, why: Why::NoEnergy }, &names(), 0);
        assert_eq!(line, "Bob lost 3 VP for producing nothing.");
        assert_eq!(contract_text(54), "15 energy: 9 VP, 3 drops that flow at once");
    }
}
