//! It's a Wonderful World's rules in short, behind the Rules button (from
//! La Boîte de Jeu's rulebooks, as `docs/wonderful-world.md` and the README
//! have them).

use crate::rules::{Rules, Section};

pub const RULES: Rules = Rules {
    intro: "Build an empire in four rounds of drafting cards. Everybody plays at the same time: there are no turns.",
    sections: &[
        Section {
            title: "Setup",
            points: &[
                "Each player gets an Empire at random, which produces a few cubes every round.",
            ],
        },
        Section {
            title: "A round",
            points: &[
                "Draft: you are dealt 7 cards (10 with two players, the last 3 discarded). Keep one, pass the rest: to the next player in rounds 1 and 3, to the previous one in rounds 2 and 4. Repeat until you have kept 7.",
                "Planning: build each kept card (it goes under construction) or recycle it for its cube, placed on a card under construction or your Empire. Scrapping a card under construction loses what is on it; its cube goes on your Empire.",
                "Production, in order: Materials, Energy, Science, Gold, Exploration. Your Empire and finished cards make that resource; place the cubes on cards under construction or on your Empire. Cubes you don't place are lost.",
                "Whoever makes strictly the most of a resource gets a character: Financier for Materials and Gold, General for Energy and Exploration, their choice for Science. A tie gives nothing.",
                "A card is finished when every space on it is filled; some give a one-off bonus.",
                "Five cubes on your Empire make a Krystallium: it replaces any resource (not a character) and is the only thing that fills a Krystallium space.",
            ],
        },
        Section {
            title: "Scoring",
            points: &[
                "The points printed on your finished cards and Empire, including those per card of a type or per character.",
                "1 point per Financier and per General you hold.",
                "Cards still under construction and Krystallium score nothing.",
            ],
        },
        Section {
            title: "End of game",
            points: &[
                "After round 4, the most points win. Ties: most finished cards, then most characters.",
            ],
        },
        Section {
            title: "Corruption & Ascension (the host's choice)",
            points: &[
                "Up to 7 players. Hands mix both decks (8 + 4 cards with two players, 6 + 3 with three or four, 6 + 2 with five or six, 5 + 2 with seven); you still keep 7.",
                "Corruption: some cards take away production of a resource (never below zero).",
                "Duos: some cards score per pair of two card types, or a type and a character.",
                "Two more Empires. From six players, the two best producers each take the character.",
            ],
        },
    ],
    links: &[
        ("Rulebook (La Boîte de Jeu)", "https://www.laboitedejeu.fr/wp-content/uploads/2021/01/ITS-EN-STD-rules-web.pdf"),
        ("Corruption & Ascension rulebook", "https://www.laboitedejeu.fr/wp-content/uploads/2021/02/corruptionascension_rules_EN.pdf"),
    ],
};
