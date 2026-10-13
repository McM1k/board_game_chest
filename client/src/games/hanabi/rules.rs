//! Hanabi's rules in short, behind the Rules button (from R&R Games'
//! rulebook and `hanabi-core`'s variants).

use crate::rules::{Rules, Section};

pub const RULES: Rules = Rules {
    intro: "Together, build the fireworks: each color from 1 up to 5. You see everyone's cards but your own.",
    sections: &[
        Section {
            title: "Setup",
            points: &[
                "Each color has three 1s, two 2s, 3s and 4s, and one 5.",
                "Hands: 5 cards with 2 or 3 players, 4 with more. Hold them facing the others.",
                "8 clue tokens and 3 fuses.",
            ],
        },
        Section {
            title: "Your turn",
            points: &[
                "Do one thing:",
                "Give a clue (spends a clue token): tell one player all their cards of one color, or of one number.",
                "Discard a card (not with all 8 clue tokens): get a clue token back, and draw.",
                "Play a card: if it is the next number of its color's firework it goes on it, otherwise it is discarded and burns a fuse. Then draw.",
                "Finishing a firework with its 5 gives a clue token back.",
            ],
        },
        Section {
            title: "Scoring",
            points: &["Your score is the total of the highest card in each firework: 25 is perfect with five colors."],
        },
        Section {
            title: "End of game",
            points: &[
                "The last fuse burns: the game is lost, scoring 0.",
                "All fireworks are complete: a perfect score.",
                "The deck runs out: everyone plays one more turn.",
            ],
        },
        Section {
            title: "Variants (the host picks)",
            points: &[
                "Multicolor: a sixth suit, touched by every color clue but never named itself.",
                "Black powder: a suit no color clue touches, built from 5 down to 1.",
                "Extra colors, six-card suits (up to 6), and \"short\" suits with one card of each number.",
                "Hanabii mode: six colors of six cards, up to 6 players; only red, yellow and blue can be named, and they touch the colors mixed from them (red touches orange and purple).",
            ],
        },
    ],
    links: &[("Hanabi rulebook (R&R Games)", "https://www.rnrgames.com/Content/RRGames/images/ProductRules/hanabiRules.pdf")],
};
