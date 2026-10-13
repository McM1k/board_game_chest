//! Catan's rules in short, behind the Rules button (from the 2020 rulebook,
//! as played here: see the README's "Rules implemented").

use crate::rules::{Rules, Section};

pub const RULES: Rules = Rules {
    intro: "Settle the island: collect resources, build roads, settlements and cities, and be the first to 10 victory points.",
    sections: &[
        Section {
            title: "Setup",
            points: &[
                "The island is 19 hexes: each makes a resource (Brick, Lumber, Wool, Grain, Ore) on its number; the desert holds the robber.",
                "Each player places a settlement and a road, then again in reverse order. The second settlement collects one of each resource around it.",
                "Settlements go on corners, never next to another settlement (the distance rule).",
            ],
        },
        Section {
            title: "Your turn",
            points: &[
                "Roll the dice: every hex with that number gives its resource to each settlement (1) and city (2) on it.",
                "A 7: anyone with more than 7 cards discards half. You move the robber (its hex makes nothing) and steal a card from someone next to it.",
                "Trade with the other players, or with the bank at 4:1 (3:1 or 2:1 at a port you have built on).",
                "Build: road = Brick + Lumber; settlement = Brick + Lumber + Wool + Grain; city (replaces a settlement) = 2 Grain + 3 Ore; development card = Ore + Wool + Grain.",
                "Play at most one development card per turn, and never on the turn you bought it: Knight (move the robber), Road Building, Year of Plenty, Monopoly.",
            ],
        },
        Section {
            title: "Scoring",
            points: &[
                "Settlement 1 point, city 2.",
                "Longest Road (5 roads or more in a row) 2 points; Largest Army (3 Knights or more) 2 points. A longer road or a bigger army takes them.",
                "Victory Point cards: 1 point each.",
            ],
        },
        Section {
            title: "End of game",
            points: &["The first player to reach 10 points on their turn wins."],
        },
    ],
    links: &[("Catan rulebook (catan.com)", "https://www.catan.com/sites/default/files/2021-06/catan_base_rules_2020_200707.pdf")],
};
