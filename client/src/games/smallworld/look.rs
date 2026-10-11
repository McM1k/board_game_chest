//! The pure functions behind the Small World screens: names and texts, the
//! colours of the seats, which tools are in hand and what a click on a
//! region does. Native tests at the bottom.

use smallworld_core::{Action, Event, Foe, Map, Phase, Power, Race, RegionState, View};

/// One colour per seat, for its tokens and its name.
pub const SEAT_COLORS: [&str; 5] = ["#d6453a", "#2f78d4", "#e6a321", "#2f9e57", "#8c57cf"];

pub fn seat_color(seat: usize) -> &'static str {
    SEAT_COLORS[seat % SEAT_COLORS.len()]
}

/// A race's short name, on its tokens.
pub fn race_tag(race: Race) -> &'static str {
    match race {
        Race::Amazons => "Ama",
        Race::Dwarves => "Dwa",
        Race::Elves => "Elf",
        Race::Ghouls => "Gho",
        Race::Giants => "Gia",
        Race::Halflings => "Hal",
        Race::Humans => "Hum",
        Race::Orcs => "Orc",
        Race::Ratmen => "Rat",
        Race::Skeletons => "Ske",
        Race::Sorcerers => "Sor",
        Race::Tritons => "Tri",
        Race::Trolls => "Tro",
        Race::Wizards => "Wiz",
    }
}

/// "Merchant Skeletons": the power, then the race, as the rules call combos.
pub fn combo_name(race: Race, power: Power) -> String {
    format!("{} {}", power.name(), race.name())
}

/// A region's name: its terrain and its number on the board, from 1.
pub fn region_name(board: usize, region: usize) -> String {
    let map = Map::for_players(board);
    let terrain = map.regions.get(region).map_or("Region", |r| r.terrain.name());
    format!("{terrain} {}", region + 1)
}

/// A seat's name, "You" for ourselves.
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

/// "Your" or "Ann's", to start a sentence.
fn whose(seat: usize, names: &[String], you: usize) -> String {
    if seat == you {
        "Your".to_string()
    } else {
        format!("{}'s", name(seat, names))
    }
}

/// "your" or "Ann's", inside a sentence.
fn whose_in(seat: usize, names: &[String], you: usize) -> String {
    if seat == you {
        "your".to_string()
    } else {
        whose(seat, names, you)
    }
}

/// What a click on the map does: one tool at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Conquer,
    Abandon,
    Dragon,
    Enchant,
    Put,
    Take,
    Camp,
    Uncamp,
    Fortify,
    Hero,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Conquer => "Conquer",
            Tool::Abandon => "Abandon",
            Tool::Dragon => "Dragon",
            Tool::Enchant => "Enchant",
            Tool::Put => "Add a token",
            Tool::Take => "Take a token",
            Tool::Camp => "Pitch a camp",
            Tool::Uncamp => "Strike a camp",
            Tool::Fortify => "Fortress",
            Tool::Hero => "Hero",
        }
    }

    /// What clicking a highlighted region does, for the tool's tooltip.
    pub fn hint(self) -> &'static str {
        match self {
            Tool::Conquer => "Conquer a highlighted region; the number is what it costs.",
            Tool::Abandon => "Empty one of your regions and take its tokens back in hand.",
            Tool::Dragon => "Conquer with a single token: the Dragon makes the region immune.",
            Tool::Enchant => "Replace a lone enemy token next to you with a Sorcerer.",
            Tool::Put => "Put a token from your hand in one of your regions.",
            Tool::Take => "Take a token back in hand (one has to stay).",
            Tool::Camp => "Pitch an encampment: +1 defense.",
            Tool::Uncamp => "Strike an encampment, to pitch it elsewhere.",
            Tool::Fortify => "Build this turn's fortress: +1 defense, +1 coin a turn.",
            Tool::Hero => "Put a Hero here, or take it off: the region is immune.",
        }
    }
}

/// The tools on offer right now, the usual one first. None when it isn't
/// our move.
pub fn tools(view: &View) -> Vec<Tool> {
    if view.actor != Some(view.you) {
        return Vec::new();
    }
    let m = &view.moves;
    let mut tools = Vec::new();
    match view.phase {
        Phase::Conquest { .. } => {
            tools.push(Tool::Conquer);
            for (tool, list) in [(Tool::Dragon, &m.dragon), (Tool::Enchant, &m.enchant), (Tool::Abandon, &m.abandon)] {
                if !list.is_empty() {
                    tools.push(tool);
                }
            }
        }
        Phase::Redeploy => {
            tools.push(Tool::Put);
            for (tool, list) in [
                (Tool::Take, &m.take),
                (Tool::Camp, &m.camp),
                (Tool::Uncamp, &m.uncamp),
                (Tool::Fortify, &m.fortify),
                (Tool::Hero, &m.hero),
            ] {
                if !list.is_empty() {
                    tools.push(tool);
                }
            }
        }
        Phase::Reinforce => tools.push(Tool::Put),
        Phase::Pick | Phase::Over => {}
    }
    tools
}

/// The tool a click uses: the one picked, if it's still on offer, else
/// the usual one for the phase.
pub fn effective_tool(view: &View, chosen: Tool) -> Option<Tool> {
    let tools = tools(view);
    if tools.contains(&chosen) {
        Some(chosen)
    } else {
        tools.first().copied()
    }
}

/// What a click on a region does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    /// Send this move.
    Act(Action),
    /// Not enough tokens: ask whether to roll the die for it.
    Gamble(usize),
    Nothing,
}

pub fn click(view: &View, tool: Tool, region: usize) -> Click {
    let m = &view.moves;
    let has = |list: &[usize]| list.contains(&region);
    let act = |ok: bool, action: Action| if ok { Click::Act(action) } else { Click::Nothing };
    match tool {
        Tool::Conquer => {
            if m.conquer.iter().any(|&(r, _)| r == region) {
                Click::Act(Action::Conquer { region })
            } else if m.gamble.iter().any(|&(r, _)| r == region) {
                Click::Gamble(region)
            } else {
                Click::Nothing
            }
        }
        Tool::Abandon => act(has(&m.abandon), Action::Abandon { region }),
        Tool::Dragon => act(has(&m.dragon), Action::Dragon { region }),
        Tool::Enchant => act(has(&m.enchant), Action::Enchant { region }),
        Tool::Put => act(has(&m.put), Action::Put { region }),
        Tool::Take => act(has(&m.take), Action::Take { region }),
        Tool::Camp => act(has(&m.camp), Action::Camp { region }),
        Tool::Uncamp => act(has(&m.uncamp), Action::Uncamp { region }),
        Tool::Fortify => act(has(&m.fortify), Action::Fortify { region }),
        Tool::Hero => act(has(&m.hero), Action::Hero { region }),
    }
}

/// How a region is marked for the tool in hand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mark {
    /// Click it: the badge says what it costs, if anything.
    Target(Option<String>),
    /// Only with the die: what it costs.
    Gamble(String),
}

/// The regions the tool does something on, and how to mark them.
pub fn marks(view: &View, tool: Tool) -> Vec<(usize, Mark)> {
    let m = &view.moves;
    let plain = |list: &[usize]| list.iter().map(|&r| (r, Mark::Target(None))).collect();
    match tool {
        Tool::Conquer => m
            .conquer
            .iter()
            .map(|&(r, cost)| (r, Mark::Target(Some(cost.to_string()))))
            .chain(m.gamble.iter().map(|&(r, cost)| (r, Mark::Gamble(format!("{cost}?")))))
            .collect(),
        Tool::Abandon => plain(&m.abandon),
        Tool::Dragon => m.dragon.iter().map(|&r| (r, Mark::Target(Some("1".into())))).collect(),
        Tool::Enchant => plain(&m.enchant),
        Tool::Put => m.put.iter().map(|&r| (r, Mark::Target(Some("+".into())))).collect(),
        Tool::Take => m.take.iter().map(|&r| (r, Mark::Target(Some("\u{2212}".into())))).collect(),
        Tool::Camp => plain(&m.camp),
        Tool::Uncamp => plain(&m.uncamp),
        Tool::Fortify => plain(&m.fortify),
        Tool::Hero => plain(&m.hero),
    }
}

/// The tokens in hand of the race that acts now (the Ghouls in decline in
/// their phase), for the action panel.
pub fn hand(view: &View) -> u8 {
    let p = &view.players[view.you];
    match view.phase {
        Phase::Conquest { ghouls: true } => p.declined.iter().find(|a| a.race == Race::Ghouls).map_or(0, |a| a.hand),
        Phase::Redeploy | Phase::Reinforce => p.active.iter().chain(&p.declined).map(|a| a.hand).sum(),
        _ => p.active.map_or(0, |a| a.hand),
    }
}

/// The big line above the board: whose turn, doing what.
pub fn phase_title(view: &View, names: &[String]) -> String {
    let current = who(view.current, names, view.you);
    let you = view.current == view.you;
    let (be, pick) = if you { ("are", "pick") } else { ("is", "picks") };
    match view.phase {
        Phase::Pick => format!("{current} {pick} a race"),
        Phase::Conquest { ghouls: true } => format!("{} Ghouls in decline are conquering", whose(view.current, names, view.you)),
        Phase::Conquest { ghouls: false } => format!("{current} {be} conquering"),
        Phase::Redeploy => format!("{current} {be} redeploying"),
        Phase::Reinforce => match view.actor {
            Some(seat) => format!("{} {} lost tokens back", who(seat, names, view.you), if seat == view.you { "put" } else { "puts" }),
            None => String::new(),
        },
        Phase::Over => "The game is over".to_string(),
    }
}

/// The line under it: what to do now, or whom we wait for.
pub fn hint(view: &View, names: &[String]) -> String {
    if view.phase == Phase::Over {
        return match view.winners.as_slice() {
            [w] if *w == view.you => "You win!".to_string(),
            [w] => format!("{} wins.", name(*w, names)),
            ws => format!("Shared win: {}.", ws.iter().map(|&w| who(w, names, view.you)).collect::<Vec<_>>().join(", ")),
        };
    }
    let Some(actor) = view.actor else { return String::new() };
    if actor != view.you {
        return format!("Waiting for {}.", name(actor, names));
    }
    let m = &view.moves;
    match view.phase {
        Phase::Pick => {
            "Pick a race and power from the column. The top one is free; each one lower costs a coin, left on the ones above it.".to_string()
        }
        Phase::Conquest { ghouls } => {
            let n = hand(view);
            let mut s = if ghouls {
                format!("Your Ghouls in decline conquer first, with {n} in hand.")
            } else {
                format!("{n} tokens in hand. Click a region to conquer it; the number is its cost.")
            };
            if !m.gamble.is_empty() {
                s.push_str(" Short of tokens? Click a region marked \u{201c}?\u{201d} to try the reinforcement die: it ends your conquests.");
            }
            if m.decline {
                s.push_str(" Or put your race in decline instead of conquering.");
            }
            s
        }
        Phase::Redeploy => match &m.end_turn {
            Some(why) => why.clone(),
            None => "Move your tokens around if you like, then end your turn to score.".to_string(),
        },
        Phase::Reinforce => format!("Put your {} tokens back in your regions.", hand(view)),
        Phase::Over => String::new(),
    }
}

/// One log line.
pub fn describe_event(event: &Event, names: &[String], you: usize, board: usize) -> String {
    let w = |seat: usize| who(seat, names, you);
    let region = |r: usize| region_name(board, r);
    match *event {
        Event::TurnStarted(turn) => format!("Turn {turn} begins."),
        Event::Picked { seat, race, power, paid, gained } => {
            let mut s = format!("{} took the {}", w(seat), combo_name(race, power));
            if paid > 0 {
                s.push_str(&format!(" for {paid} coin{}", plural(paid)));
            }
            if gained > 0 {
                s.push_str(&format!(", pocketing {gained} coin{}", plural(gained)));
            }
            s.push('.');
            s
        }
        Event::Conquered { seat, race, region: r, tokens, foe, dragon } => {
            let by = if dragon { " with the Dragon" } else { "" };
            let from = match foe {
                None => String::new(),
                Some(Foe::LostTribe) => " from a Lost Tribe".to_string(),
                Some(Foe::Race { seat: s, race: r }) => format!(" from {} {}", whose_in(s, names, you), r.name()),
            };
            format!("{} {} took {}{from}{by} ({tokens} token{}).", whose(seat, names, you), race.name(), region(r), plural(tokens as u32))
        }
        Event::Gambled { seat, region: r, die, won } => format!(
            "{} rolled {die} for {}: {}.",
            w(seat),
            region(r),
            if won { "conquered" } else { "not enough" }
        ),
        Event::Rolled { seat, die } => format!("{} rolled {die} before conquering.", w(seat)),
        Event::Enchanted { seat, region: r, victim, race } => {
            format!("{} Sorcerers enchanted {} {} in {}.", whose(seat, names, you), whose_in(victim, names, you), race.name(), region(r))
        }
        Event::Abandoned { seat, region: r } => format!("{} abandoned {}.", w(seat), region(r)),
        Event::Raised { seat, tokens } => format!("{} Skeletons raised {tokens} more.", whose(seat, names, you)),
        Event::Declined { seat, race } => format!("{} {} went into decline.", whose(seat, names, you), race.name()),
        Event::Vanished { seat, race } => format!("{} {} in decline left the map.", whose(seat, names, you), race.name()),
        Event::Scored { seat, coins } => format!("{} scored {coins} coin{}.", w(seat), plural(coins)),
        Event::Peace { seat, ally } => format!("{} made peace with {}.", w(seat), w(ally)),
        Event::GameOver => "The game is over.".to_string(),
    }
}

fn plural(n: u32) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// A region's tooltip: what it is and what stands there.
pub fn region_title(view: &View, region: usize, names: &[String]) -> String {
    let def = &Map::for_players(view.board).regions[region];
    let r: &RegionState = &view.regions[region];
    let mut parts = vec![region_name(view.board, region)];
    for (has, what) in [(def.mine(), "mine"), (def.magic(), "magic source"), (def.cavern(), "cavern")] {
        if has {
            parts.push(what.to_string());
        }
    }
    if r.lost_tribe {
        parts.push("Lost Tribe".to_string());
    }
    if let Some(t) = r.tokens {
        let owner = view.players.iter().position(|p| p.active.iter().chain(&p.declined).any(|a| a.race == t.race));
        let owner = owner.map(|s| format!(" ({})", who(s, names, view.you))).unwrap_or_default();
        let decline = if t.declined { " in decline" } else { "" };
        parts.push(format!("{} {}{decline}{owner}", t.count, t.race.name()));
    }
    for (has, what) in [
        (r.lair, "Troll's Lair"),
        (r.fortress, "fortress"),
        (r.hole, "Hole-in-the-Ground"),
        (r.hero, "Hero"),
        (r.dragon, "Dragon"),
    ] {
        if has {
            parts.push(what.to_string());
        }
    }
    if r.encampments > 0 {
        parts.push(format!("{} encampment{}", r.encampments, plural(r.encampments as u32)));
    }
    parts.join(" \u{b7} ")
}

/// The seat whose tokens stand in a region, for their colour.
pub fn region_owner(view: &View, region: usize) -> Option<usize> {
    let race = view.regions[region].tokens?.race;
    view.players.iter().position(|p| p.active.iter().chain(&p.declined).any(|a| a.race == race))
}

/// The final standings: seats from the best, with coins and tokens.
pub fn standings(view: &View) -> Vec<(usize, u32, u32)> {
    let mut rows: Vec<(usize, u32, u32)> = view
        .players
        .iter()
        .enumerate()
        .map(|(seat, p)| (seat, p.coins.unwrap_or(0), p.on_map))
        .collect();
    rows.sort_by(|a, b| (b.1, b.2).cmp(&(a.1, a.2)));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use smallworld_core::{Phase, State};

    fn names() -> Vec<String> {
        ["Ann", "Bob", "Cy"].iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_tools_follow_the_phase_and_whose_move_it_is() {
        let mut s = State::new(3, 4);
        let v = s.view_for(0);
        assert!(tools(&v).is_empty(), "picking happens in the column");
        assert!(tools(&s.view_for(1)).is_empty());
        s.apply(0, Action::Pick { offer: 0 }).unwrap();
        let v = s.view_for(0);
        assert_eq!(tools(&v)[0], Tool::Conquer);
        assert!(tools(&s.view_for(1)).is_empty(), "not their turn");
        // A tool that isn't on offer falls back to the usual one.
        assert_eq!(effective_tool(&v, Tool::Take), Some(Tool::Conquer));
        s.apply(0, Action::EndConquests).unwrap();
        let v = s.view_for(0);
        assert_eq!(effective_tool(&v, Tool::Conquer), Some(Tool::Put));
    }

    #[test]
    fn a_click_conquers_or_asks_about_the_die() {
        let mut s = State::new(2, 4);
        s.apply(0, Action::Pick { offer: 0 }).unwrap();
        let v = s.view_for(0);
        let (r, cost) = v.moves.conquer[0];
        assert_eq!(click(&v, Tool::Conquer, r), Click::Act(Action::Conquer { region: r }));
        assert!(marks(&v, Tool::Conquer).contains(&(r, Mark::Target(Some(cost.to_string())))));
        // An inland region is out of reach at first.
        let inland = (0..v.regions.len()).find(|&r| !Map::for_players(2).border(r)).unwrap();
        assert_eq!(click(&v, Tool::Conquer, inland), Click::Nothing);
        // Down to one token: border regions need the die.
        s.players[0].active.as_mut().unwrap().hand = 1;
        let v = s.view_for(0);
        let (g, _) = v.moves.gamble[0];
        assert_eq!(click(&v, Tool::Conquer, g), Click::Gamble(g));
        assert!(matches!(marks(&v, Tool::Conquer).iter().find(|(r, _)| *r == g), Some((_, Mark::Gamble(_)))));
    }

    #[test]
    fn texts_name_the_seats_and_regions() {
        let mut s = State::new(3, 4);
        let v = s.view_for(0);
        assert_eq!(phase_title(&v, &names()), "You pick a race");
        assert_eq!(phase_title(&s.view_for(1), &names()), "Ann picks a race");
        assert!(hint(&v, &names()).starts_with("Pick a race"));
        assert_eq!(hint(&s.view_for(2), &names()), "Waiting for Ann.");
        s.apply(0, Action::Pick { offer: 0 }).unwrap();
        let v = s.view_for(1);
        assert_eq!(phase_title(&v, &names()), "Ann is conquering");
        let picked = v.log.iter().rev().find(|e| matches!(e, Event::Picked { .. })).unwrap();
        let line = describe_event(picked, &names(), 1, 3);
        let a = v.players[0].active.unwrap();
        assert_eq!(line, format!("Ann took the {}.", combo_name(a.race, a.power)));
        assert_eq!(describe_event(picked, &names(), 0, 3), format!("You took the {}.", combo_name(a.race, a.power)));
        assert_eq!(region_name(2, 12), "Hill 13");
        let taken = Event::Conquered {
            seat: 1,
            race: Race::Orcs,
            region: 12,
            tokens: 3,
            foe: Some(Foe::Race { seat: 0, race: Race::Elves }),
            dragon: false,
        };
        assert_eq!(describe_event(&taken, &names(), 0, 2), "Bob's Orcs took Hill 13 from your Elves (3 tokens).");
        assert_eq!(describe_event(&taken, &names(), 1, 2), "Your Orcs took Hill 13 from Ann's Elves (3 tokens).");
        assert_eq!(combo_name(Race::Skeletons, Power::Merchant), "Merchant Skeletons");
        let r = v.moves.conquer.first().map(|&(r, _)| r);
        assert!(r.is_none(), "seat 1 sees no moves");
        let r = s.view_for(0).moves.conquer[0].0;
        s.apply(0, Action::Conquer { region: r }).unwrap();
        let v = s.view_for(1);
        let title = region_title(&v, r, &names());
        assert!(title.contains(a.race.name()) && title.contains("(Ann)"), "{title}");
        assert_eq!(region_owner(&v, r), Some(0));
        let line = describe_event(v.log.last().unwrap(), &names(), 1, 3);
        assert!(line.starts_with(&format!("Ann's {} took {}", a.race.name(), region_name(3, r))), "{line}");
    }

    #[test]
    fn standings_go_by_coins_then_tokens() {
        let mut s = State::new(3, 1);
        s.players[0].coins = 10;
        s.players[1].coins = 12;
        s.players[2].coins = 10;
        s.phase = Phase::Over;
        let v = s.view_for(0);
        assert_eq!(standings(&v).iter().map(|r| r.0).collect::<Vec<_>>(), [1, 0, 2]);
        assert_eq!(hint(&v, &names()), "Bob wins.");
    }
}
