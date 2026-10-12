//! The pure functions behind the Small World screens: names and texts, the
//! colours of the seats, which tools are in hand and what a click on a
//! region does. Native tests at the bottom.

use smallworld_core::{Action, Board, Event, Foe, How, Loot, Phase, Power, Race, RegionMove, Sky, View, Why, CURSED_SKIP};

/// One colour per seat, for its tokens and its name.
pub const SEAT_COLORS: [&str; 6] = ["#d6453a", "#2f78d4", "#e6a321", "#2f9e57", "#8c57cf", "#d0679d"];

pub fn seat_color(seat: usize) -> &'static str {
    SEAT_COLORS[seat % SEAT_COLORS.len()]
}

/// Where the pictures are served: `art/` next to this file, which Trunk
/// copies into the build (see `client/index.html`). They come from Days of
/// Wonder's rule sheets, and the base game's banners from scans.
pub const ART: &str = "/smallworld-art";

/// A race's portrait: the medallion from its banner.
pub fn race_art(race: Race) -> String {
    format!("{ART}/races/{}.webp", format!("{race:?}").to_lowercase())
}

/// The printed ground board for `players` players (2 to 5), photographed.
pub fn board_art(players: usize) -> String {
    format!("{ART}/boards/{}.webp", players.clamp(2, 5))
}

/// A printed side of the Sky Islands.
pub fn sky_art(side: Sky) -> String {
    let name = match side {
        Sky::Three => "sky-three",
        Sky::Two => "sky-two",
    };
    format!("{ART}/boards/{name}.webp")
}

/// A race's banner, which a power's badge slots against.
pub fn banner_art(race: Race) -> String {
    format!("{ART}/banners/{}.webp", format!("{race:?}").to_lowercase())
}

/// A power's badge.
pub fn power_art(power: Power) -> String {
    format!("{ART}/powers/{}.webp", format!("{power:?}").to_lowercase())
}

/// "Merchant Skeletons": the power, then the race, as the rules call combos
/// ("Were-Ratmen" in one word).
pub fn combo_name(race: Race, power: Power) -> String {
    let p = power.name();
    if p.ends_with('-') {
        format!("{p}{}", race.name())
    } else {
        format!("{p} {}", race.name())
    }
}

/// A region's name: its terrain and its number on the board, from 1 ("Sky"
/// in front for the Sky Islands).
pub fn region_name(board: &Board, region: usize) -> String {
    match board.regions.get(region) {
        Some(r) if r.sky => format!("Sky {} {}", r.terrain.name(), region + 1),
        Some(r) => format!("{} {}", r.terrain.name(), region + 1),
        None => format!("Region {}", region + 1),
    }
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

/// The tool in hand decides what a click on the map does: a kind of move
/// on a region. Conquering covers the die too.
pub type Tool = RegionMove;

pub fn tool_label(tool: Tool) -> &'static str {
    match tool {
        RegionMove::Conquer | RegionMove::Gamble => "Conquer",
        RegionMove::Hire => "Mercenaries",
        RegionMove::Fireball => "Fireball",
        RegionMove::Behemoth => "Behemoth",
        RegionMove::Catapult => "Catapult",
        RegionMove::Dragon => "Dragon",
        RegionMove::Lightning => "Lightning",
        RegionMove::Zeppelin => "Zeppelin",
        RegionMove::Enchant => "Enchant",
        RegionMove::Wendigo => "Clear a forest",
        RegionMove::Fire => "Fire a cannon",
        RegionMove::Abandon => "Abandon",
        RegionMove::Tower => "Ivory Tower",
        RegionMove::Put => "Add a token",
        RegionMove::Take => "Take a token",
        RegionMove::Camp => "Pitch a camp",
        RegionMove::Uncamp => "Strike a camp",
        RegionMove::Fortify => "Fortress",
        RegionMove::Hero => "Hero",
        RegionMove::Pot => "Pot of Gold",
        RegionMove::Winter => "Winter",
        RegionMove::Lava => "Lava",
        RegionMove::Herd => "Place a Behemoth",
        RegionMove::Cannon => "Cannon",
    }
}

/// What clicking a highlighted region does, for the tool's tooltip.
pub fn tool_hint(tool: Tool) -> &'static str {
    match tool {
        RegionMove::Conquer | RegionMove::Gamble => "Conquer a highlighted region; the number is what it costs.",
        RegionMove::Hire => "Pay a coin to conquer with 2 tokens less.",
        RegionMove::Fireball => "Throw Fireballs (2 tokens each) to conquer for fewer tokens.",
        RegionMove::Behemoth => "Attack with a Behemoth, worth as many tokens as your Swamps.",
        RegionMove::Catapult => "Conquer a region two away with 1 token less; the Catapult protects where it fires from.",
        RegionMove::Dragon => "Conquer with a single token: the Dragon makes the region immune.",
        RegionMove::Lightning => "A Lightning Bolt takes a Mountain with a single token.",
        RegionMove::Zeppelin => "Roll the die and conquer anywhere: each pip is a token; a blank burns the region.",
        RegionMove::Enchant => "Replace a lone enemy token next to you with a Sorcerer.",
        RegionMove::Wendigo => "Clear a Forest region anywhere of its tokens.",
        RegionMove::Fire => "This Cannon fires: conquests next to it cost 2 tokens less.",
        RegionMove::Abandon => "Empty one of your regions and take its tokens back in hand.",
        RegionMove::Tower => "Go into decline, stacking your Ivory Tower in this region.",
        RegionMove::Put => "Put a token from your hand in one of your regions.",
        RegionMove::Take => "Take a token back in hand (one has to stay).",
        RegionMove::Camp => "Pitch an encampment: +1 defense.",
        RegionMove::Uncamp => "Strike an encampment, to pitch it elsewhere.",
        RegionMove::Fortify => "Build this turn's fortress: +1 defense, +1 coin a turn.",
        RegionMove::Hero => "Put a Hero here, or take it off: the region is immune.",
        RegionMove::Pot => "Put a Pot of Gold here, or take it off: a coin at your next turn if it's still yours.",
        RegionMove::Winter => "Put a Winter marker here: +1 defense, 1 coin less for anyone else.",
        RegionMove::Lava => "Pour Lava here: whoever is there takes their tokens back, and nobody may enter until your next turn.",
        RegionMove::Herd => "Stand a Behemoth here, or take it back in hand.",
        RegionMove::Cannon => "Put a Cannon here, or take it off to move it.",
    }
}

/// The tools on offer right now, the usual one first. None when it isn't
/// our move.
pub fn tools(view: &View) -> Vec<Tool> {
    if view.actor != Some(view.you) {
        return Vec::new();
    }
    let mut tools: Vec<Tool> = Vec::new();
    match view.phase {
        Phase::Conquest { .. } => tools.push(RegionMove::Conquer),
        Phase::Redeploy | Phase::Reinforce => tools.push(RegionMove::Put),
        Phase::Pick | Phase::Over => return tools,
    }
    for kind in RegionMove::ALL {
        let kind = if kind == RegionMove::Gamble { RegionMove::Conquer } else { kind };
        if !tools.contains(&kind) && view.moves.regions.iter().any(|t| t.kind == kind) {
            tools.push(kind);
        }
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
    if tool == RegionMove::Conquer && !m.can(RegionMove::Conquer, region) && m.can(RegionMove::Gamble, region) {
        return Click::Gamble(region);
    }
    if m.can(tool, region) {
        Click::Act(tool.action(region))
    } else {
        Click::Nothing
    }
}

/// How a region is marked for the tool in hand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mark {
    /// Click it: the badge says what it costs, if anything.
    Target(Option<String>),
    /// Only with the die: what it costs.
    Gamble(String),
    /// Out of reach for now (or too dear): what it would cost, to plan ahead.
    Price(String),
}

/// The regions the tool does something on, and how to mark them.
pub fn marks(view: &View, tool: Tool) -> Vec<(usize, Mark)> {
    let m = &view.moves;
    let mut marks: Vec<(usize, Mark)> = Vec::new();
    for t in &m.regions {
        let mark = match (tool, t.kind) {
            (RegionMove::Conquer, RegionMove::Conquer) => Mark::Target(Some(t.cost.to_string())),
            (RegionMove::Conquer, RegionMove::Gamble) if !m.can(RegionMove::Conquer, t.region) => {
                Mark::Gamble(format!("{}?", t.cost))
            }
            (RegionMove::Zeppelin, RegionMove::Zeppelin) => Mark::Gamble(format!("{}?", t.cost)),
            (
                RegionMove::Hire
                | RegionMove::Fireball
                | RegionMove::Behemoth
                | RegionMove::Catapult
                | RegionMove::Dragon
                | RegionMove::Lightning,
                kind,
            ) if kind == tool => Mark::Target(Some(t.cost.to_string())),
            (RegionMove::Put, RegionMove::Put) => Mark::Target(Some("+".into())),
            (RegionMove::Take, RegionMove::Take) => Mark::Target(Some("\u{2212}".into())),
            (tool, kind) if tool == kind => Mark::Target(None),
            _ => continue,
        };
        marks.push((t.region, mark));
    }
    // Conquering: the rest of the map is priced too, to plan ahead.
    if tool == RegionMove::Conquer {
        for &(region, cost) in &m.prices {
            if !marks.iter().any(|(r, _)| *r == region) {
                marks.push((region, Mark::Price(cost.to_string())));
            }
        }
    }
    marks
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

/// What the combo at `offer` costs us: a coin per combo above, 3 for a
/// Cursed one, nothing after a Racketeering decline.
pub fn pick_cost(view: &View, offer: usize) -> u32 {
    if view.players[view.you].free_pick {
        return 0;
    }
    view.offers[..offer.min(view.offers.len())]
        .iter()
        .map(|o| if o.power == Power::Cursed { CURSED_SKIP } else { 1 })
        .sum()
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
                let then = match view.players[view.you].active {
                    Some(a) => format!(", then your {}, with {}", combo_name(a.race, a.power), a.hand),
                    None => String::new(),
                };
                format!("Your Ghouls in decline conquer first, with {n} in hand{then}.")
            } else {
                format!("{n} tokens in hand. Click a region to conquer it; the number is its cost (faded: out of reach for now).")
            };
            if !m.gamble().is_empty() {
                let whose = if ghouls { "the Ghouls'" } else { "your" };
                s.push_str(&format!(
                    " Short of tokens? Click a region marked \u{201c}?\u{201d} to try the reinforcement die: it ends {whose} conquests."
                ));
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

fn why_name(why: Why) -> &'static str {
    match why {
        Why::Ransacking => "Ransacking",
        Why::Corrupt => "Corrupt",
        Why::Scarecrows => "Scarecrows",
        Why::Slingmen => "a sling shot",
        Why::Loot => "the Skags' loot",
        Why::Pot => "a Pot of Gold",
        Why::Pact => "Trade Pacts",
        Why::Historian => "Historian",
        Why::Racketeering => "Racketeering",
        Why::Escargots => "the Escargots' regions",
    }
}

/// One log line.
pub fn describe_event(event: &Event, names: &[String], you: usize, board: &Board) -> String {
    let w = |seat: usize| who(seat, names, you);
    let region = |r: usize| region_name(board, r);
    let coins = |n: u32| format!("{n} coin{}", plural(n));
    match *event {
        Event::TurnStarted(turn) => format!("Turn {turn} begins."),
        Event::Picked { seat, race, power, paid, gained } => {
            let mut s = format!("{} took the {}", w(seat), combo_name(race, power));
            if paid > 0 {
                s.push_str(&format!(" for {}", coins(paid)));
            }
            if gained > 0 {
                s.push_str(&format!(", pocketing {}", coins(gained)));
            }
            s.push('.');
            s
        }
        Event::Conquered { seat, race, region: r, tokens, foe, how } => {
            let by = match how {
                How::Tokens => "",
                How::Dragon => " with a Dragon",
                How::Hire => " with mercenaries",
                How::Fireball => " with Fireballs",
                How::Behemoth => " with a Behemoth",
                How::Catapult => " with the Catapult",
                How::Lightning => " with a Lightning Bolt",
                How::Zeppelin => " by Zeppelin",
                How::Sling => " with slings",
            };
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
        Event::Zeppelin { seat, region: r, die: 0 } => format!("{} Zeppelin crashed on {}, which burns.", whose(seat, names, you), region(r)),
        Event::Zeppelin { seat, region: r, die } => format!("{} Zeppelin rolled {die} over {}.", whose(seat, names, you), region(r)),
        Event::Enchanted { seat, region: r, victim, race } => {
            format!("{} Sorcerers enchanted {} {} in {}.", whose(seat, names, you), whose_in(victim, names, you), race.name(), region(r))
        }
        Event::Abandoned { seat, region: r } => format!("{} abandoned {}.", w(seat), region(r)),
        Event::Raised { seat, race, tokens } => format!("{} {} got {tokens} more.", whose(seat, names, you), race.name()),
        Event::Declined { seat, race } => format!("{} {} went into decline.", whose(seat, names, you), race.name()),
        Event::Vanished { seat, race } => format!("{} {} in decline left the map.", whose(seat, names, you), race.name()),
        Event::Revived { seat, race } => format!("{} {} came back to life.", whose(seat, names, you), race.name()),
        Event::Scored { seat, coins: n } => format!("{} scored {}.", w(seat), coins(n)),
        Event::Paid { from: None, to, coins: n, why } => format!("{} got {} for {}.", w(to), coins(n), why_name(why)),
        Event::Paid { from: Some(from), to, coins: n, why } => {
            format!("{} paid {} {} for {}.", w(from), if to == you { "you".to_string() } else { name(to, names) }, coins(n), why_name(why))
        }
        Event::Peace { seat, ally } => format!("{} made peace with {}.", w(seat), w(ally)),
        Event::SkagAttack { seat, region: r } => format!("A Skag Attack drove {} off {}.", if seat == you { "you".to_string() } else { name(seat, names) }, region(r)),
        Event::Lava { seat, region: r } => format!("{} poured Lava on {}.", w(seat), region(r)),
        Event::Wendigo { seat, region: r } => format!("{} Wendigos cleared {}.", whose(seat, names, you), region(r)),
        Event::Copied { seat, power: Some(p) } => format!("{} copied {}.", w(seat), p.name()),
        Event::Copied { seat, power: None } => format!("{} copied nothing.", w(seat)),
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
pub fn region_title(view: &View, board: &Board, region: usize, names: &[String]) -> String {
    let def = &board.regions[region];
    let r = &view.regions[region];
    let mut parts = vec![region_name(board, region)];
    for (has, what) in [(def.mine(), "mine"), (def.magic(), "magic source"), (def.cavern(), "cavern")] {
        if has {
            parts.push(what.to_string());
        }
    }
    for (i, &(g, l)) in board.access.iter().enumerate() {
        let name = if i == 0 { "the Beanstalk" } else { "the Stairway" };
        if g == region || l == region {
            parts.push(format!("{name} ({} and {})", region_name(board, g), region_name(board, l)));
        }
    }
    if r.lost_tribe {
        parts.push("Lost Tribe".to_string());
    }
    if let Some(t) = r.tokens {
        let owner = region_owner(view, region).map(|s| format!(" ({})", who(s, names, view.you))).unwrap_or_default();
        let decline = if t.declined { " in decline" } else { "" };
        parts.push(format!("{} {}{decline}{owner}", t.count, t.race.name()));
    }
    if let Some(c) = r.captive {
        parts.push(format!("1 {} kept by the Scavengers", c.name()));
    }
    for (has, what) in [
        (r.lair, "Troll's Lair"),
        (r.fortress, "fortress"),
        (r.hole, "Hole-in-the-Ground"),
        (r.hero, "Hero"),
        (r.dragon, "Dragon"),
        (r.pot, "Pot of Gold"),
        (r.catapult, "Catapult"),
        (r.winter, "Winter"),
        (r.lava.is_some(), "Lava"),
        (r.burning.is_some(), "burning"),
    ] {
        if has {
            parts.push(what.to_string());
        }
    }
    if r.encampments > 0 {
        parts.push(format!("{} encampment{}", r.encampments, plural(r.encampments as u32)));
    }
    if r.behemoths > 0 {
        parts.push(format!("{} Behemoth{}", r.behemoths, plural(r.behemoths as u32)));
    }
    match r.cannon {
        Some(true) => parts.push("Cannon, firing".to_string()),
        Some(false) => parts.push("Cannon".to_string()),
        None => {}
    }
    if r.loot {
        match view.loot.iter().find(|(l, _)| *l == region) {
            Some((_, Loot::Coins(n))) => parts.push(format!("Loot: {}", if *n == 1 { "1 coin".to_string() } else { format!("{n} coins") })),
            Some((_, Loot::SkagAttack)) => parts.push("Loot: a Skag Attack".to_string()),
            None => parts.push("Loot, face down".to_string()),
        }
    }
    parts.join(" \u{b7} ")
}

/// The seat whose tokens stand in a region, for their colour.
pub fn region_owner(view: &View, region: usize) -> Option<usize> {
    let race = view.regions[region].tokens?.race;
    owner_of(view, race)
}

/// The seat playing a race.
pub fn owner_of(view: &View, race: Race) -> Option<usize> {
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
    use smallworld_core::{Options, Phase, Sky, State};

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
        assert_eq!(tools(&v)[0], RegionMove::Conquer);
        assert!(tools(&s.view_for(1)).is_empty(), "not their turn");
        // A tool that isn't on offer falls back to the usual one.
        assert_eq!(effective_tool(&v, RegionMove::Take), Some(RegionMove::Conquer));
        s.apply(0, Action::EndConquests).unwrap();
        let v = s.view_for(0);
        assert_eq!(effective_tool(&v, RegionMove::Conquer), Some(RegionMove::Put));
    }

    #[test]
    fn a_click_conquers_or_asks_about_the_die() {
        let mut s = State::new(2, 4);
        s.apply(0, Action::Pick { offer: 0 }).unwrap();
        let v = s.view_for(0);
        let (r, cost) = v.moves.conquer()[0];
        assert_eq!(click(&v, RegionMove::Conquer, r), Click::Act(Action::Conquer { region: r }));
        assert!(marks(&v, RegionMove::Conquer).contains(&(r, Mark::Target(Some(cost.to_string())))));
        // An inland region is out of reach at first.
        let board = v.map();
        let inland = (0..v.regions.len()).find(|&r| !board.border(r)).unwrap();
        assert_eq!(click(&v, RegionMove::Conquer, inland), Click::Nothing);
        // Down to one token: border regions need the die.
        s.players[0].active.as_mut().unwrap().hand = 1;
        let v = s.view_for(0);
        let (g, _) = v.moves.gamble()[0];
        assert_eq!(click(&v, RegionMove::Conquer, g), Click::Gamble(g));
        assert!(matches!(marks(&v, RegionMove::Conquer).iter().find(|(r, _)| *r == g), Some((_, Mark::Gamble(_)))));
    }

    #[test]
    fn regions_out_of_reach_show_their_price_too() {
        let mut s = State::new(2, 4);
        s.apply(0, Action::Pick { offer: 0 }).unwrap();
        let v = s.view_for(0);
        let marks = marks(&v, RegionMove::Conquer);
        let m = &v.moves;
        let far: Vec<(usize, u8)> = m
            .prices
            .iter()
            .copied()
            .filter(|&(r, _)| !m.can(RegionMove::Conquer, r) && !m.can(RegionMove::Gamble, r))
            .collect();
        assert!(!far.is_empty(), "the inland regions can't be entered yet");
        for (r, cost) in far {
            assert!(marks.contains(&(r, Mark::Price(cost.to_string()))), "{r}");
            assert_eq!(click(&v, RegionMove::Conquer, r), Click::Nothing);
        }
        // What can be conquered now keeps its own mark, once.
        let (r, cost) = m.conquer()[0];
        assert_eq!(marks.iter().filter(|(m, _)| *m == r).count(), 1);
        assert!(marks.contains(&(r, Mark::Target(Some(cost.to_string())))));
        // The other tools don't show prices.
        assert!(super::marks(&v, RegionMove::Abandon).is_empty());
    }

    #[test]
    fn every_race_and_power_has_its_picture() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/games/smallworld/art");
        let file = |url: String| dir.join(url.strip_prefix(&format!("{ART}/")).unwrap());
        for race in Race::ALL {
            assert!(file(race_art(race)).is_file(), "{race:?}: {}", race_art(race));
            assert!(file(banner_art(race)).is_file(), "{race:?}: {}", banner_art(race));
        }
        for power in Power::ALL {
            assert!(file(power_art(power)).is_file(), "{power:?}: {}", power_art(power));
        }
        for players in 2..=5 {
            assert!(file(board_art(players)).is_file(), "{}", board_art(players));
        }
        for side in [Sky::Three, Sky::Two] {
            assert!(file(sky_art(side)).is_file(), "{}", sky_art(side));
        }
        assert_eq!(race_art(Race::StormGiants), "/smallworld-art/races/stormgiants.webp");
        assert_eq!(power_art(Power::HordesOf), "/smallworld-art/powers/hordesof.webp");
    }

    #[test]
    fn a_power_brings_its_own_tool() {
        let mut s = State::with_options(2, 4, Options { sky: None, ..Options::all() });
        s.offers[0].power = Power::Zeppelined;
        s.apply(0, Action::Pick { offer: 0 }).unwrap();
        let v = s.view_for(0);
        assert!(tools(&v).contains(&RegionMove::Zeppelin));
        let marks = marks(&v, RegionMove::Zeppelin);
        assert_eq!(marks.len(), v.moves.list(RegionMove::Zeppelin).len());
        assert!(marks.iter().all(|(_, m)| matches!(m, Mark::Gamble(_))));
        assert_eq!(tool_label(RegionMove::Zeppelin), "Zeppelin");
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
        let board = v.map();
        assert_eq!(phase_title(&v, &names()), "Ann is conquering");
        let picked = v.log.iter().rev().find(|e| matches!(e, Event::Picked { .. })).unwrap();
        let line = describe_event(picked, &names(), 1, &board);
        let a = v.players[0].active.unwrap();
        assert_eq!(line, format!("Ann took the {}.", combo_name(a.race, a.power)));
        assert_eq!(describe_event(picked, &names(), 0, &board), format!("You took the {}.", combo_name(a.race, a.power)));
        assert_eq!(region_name(&State::new(2, 1).board, 12), "Hill 13");
        assert_eq!(combo_name(Race::Skeletons, Power::Merchant), "Merchant Skeletons");
        assert_eq!(combo_name(Race::Ratmen, Power::Were), "Were-Ratmen");
        assert_eq!(combo_name(Race::Orcs, Power::HordesOf), "Hordes of Orcs");
        let taken = Event::Conquered {
            seat: 1,
            race: Race::Orcs,
            region: 12,
            tokens: 3,
            foe: Some(Foe::Race { seat: 0, race: Race::Elves }),
            how: How::Tokens,
        };
        let two = State::new(2, 1).board;
        assert_eq!(describe_event(&taken, &names(), 0, &two), "Bob's Orcs took Hill 13 from your Elves (3 tokens).");
        assert_eq!(describe_event(&taken, &names(), 1, &two), "Your Orcs took Hill 13 from Ann's Elves (3 tokens).");
        let paid = Event::Paid { from: Some(1), to: 0, coins: 2, why: Why::Pact };
        assert_eq!(describe_event(&paid, &names(), 0, &two), "Bob paid you 2 coins for Trade Pacts.");
        let r = s.view_for(0).moves.conquer()[0].0;
        s.apply(0, Action::Conquer { region: r }).unwrap();
        let v = s.view_for(1);
        let title = region_title(&v, &board, r, &names());
        assert!(title.contains(a.race.name()) && title.contains("(Ann)"), "{title}");
        assert_eq!(region_owner(&v, r), Some(0));
        let line = describe_event(v.log.last().unwrap(), &names(), 1, &board);
        assert!(line.starts_with(&format!("Ann's {} took {}", a.race.name(), region_name(&board, r))), "{line}");
    }

    #[test]
    fn the_sky_regions_are_named_and_the_access_points_shown() {
        let s = State::with_options(3, 2, Options { sky: Some(Sky::Three), ..Options::default() });
        let v = s.view_for(0);
        let board = v.map();
        assert_eq!(board.len(), 23 + 9);
        assert!(region_name(&board, 23).starts_with("Sky Farmland 24"));
        let (ground, landing) = board.access[0];
        let title = region_title(&v, &board, ground, &names());
        assert!(title.contains("the Beanstalk") && title.contains(&region_name(&board, landing)), "{title}");
    }

    #[test]
    fn a_cursed_combo_costs_three_to_skip() {
        let mut s = State::with_options(2, 1, Options { cursed: true, ..Options::default() });
        s.offers[0].power = Power::Cursed;
        let v = s.view_for(0);
        assert_eq!(pick_cost(&v, 2), CURSED_SKIP + 1);
        assert_eq!(v.moves.pick.iter().find(|(i, _)| *i == 2), Some(&(2, CURSED_SKIP + 1)));
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
