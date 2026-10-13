//! Small World's rules in short, behind the Rules button (from Days of
//! Wonder's 2015 rulebook and the expansions' rule sheets, listed in
//! `docs/small-world.md`).

use crate::rules::{Rules, Section};

pub const RULES: Rules = Rules {
    intro: "Lead fantasy races, one after another, to conquer a world too small for all of them. The most coins win.",
    sections: &[
        Section {
            title: "Setup",
            points: &[
                "The board fits the number of players. Lost Tribes hold some regions.",
                "Everyone starts with 5 coins. Six race and power combos are face up in a column.",
                "The game lasts 10 turns with 2 or 3 players, 9 with 4, 8 with 5.",
            ],
        },
        Section {
            title: "Your turn",
            points: &[
                "No active race? Pick a combo: the top one is free; each lower one costs 1 coin, left on each combo above it. Take the coins left on yours. Banner + badge = your tokens.",
                "Conquer: a region costs 2 tokens, +1 for a Mountain, a fortress, a lair or an encampment, +1 per token on it. A new race enters from the board's edge, then conquers next to its regions.",
                "Defenders lose one token and redeploy the rest after your turn.",
                "Short of tokens for one last conquest? Roll the reinforcement die (0 to 3). Your conquests end either way.",
                "Later turns: keep one token per region and take the rest back to conquer more. Or instead put your race in decline: it stays on the board without its power, and you pick a new race next turn. Only one race in decline at a time.",
                "End by redeploying your tokens among your regions.",
            ],
        },
        Section {
            title: "Scoring",
            points: &[
                "At the end of your turn: 1 coin per region held by your active race and your race in decline, plus the bonuses of races and powers.",
                "Coins stay hidden until the end.",
            ],
        },
        Section {
            title: "End of game",
            points: &["After the last turn, the most coins win; a tie goes to the most tokens on the map."],
        },
        Section {
            title: "Expansions (the host ticks them)",
            points: &[
                "Their races and powers join the stacks: Cursed!, Grand Dames, Be Not Afraid..., Royal Bonus, A Spider's Web.",
                "Sky Islands (3 to 6 players): floating islands beside the ground board, reached by the Beanstalk and the Stairway to Heaven; an island held whole earns a coin. Its own races and powers come with it.",
            ],
        },
    ],
    links: &[
        ("Rulebook (Days of Wonder)", "https://cdn1.daysofwonder.com/smallworld/en/img/sw_rules_2015_en.pdf"),
        ("Cursed!", "https://cdn1.daysofwonder.com/smallworld/en/img/sw_cu_rules_en.pdf"),
        ("Grand Dames", "https://cdn1.daysofwonder.com/smallworld/en/img/sw_gd_rules_en.pdf"),
        ("Be Not Afraid...", "https://cdn1.daysofwonder.com/smallworld/en/img/sw_bna_en.pdf"),
        ("Royal Bonus", "https://cdn1.daysofwonder.com/smallworld/en/img/sw_royal_en.pdf"),
        ("A Spider's Web", "https://cdn1.daysofwonder.com/smallworld/en/img/sw_spider_en.pdf"),
        ("Sky Islands", "https://cdn1.daysofwonder.com/smallworld/en/img/sw_si_en.pdf"),
    ],
};
