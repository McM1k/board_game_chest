//! Barrage's rules in short, behind the Rules button (from the 2019 English
//! rulebook, as `docs/barrage.md` and the README have them).

use crate::rules::{Rules, Section};

pub const RULES: Rules = Rules {
    intro: "Run an energy company in the Alps: build dams, conduits and powerhouses, hold back the water and turn it into energy for contracts. Five rounds.",
    sections: &[
        Section {
            title: "Setup",
            points: &[
                "In turn, each player picks a Company (with its Executive Officer) and a Starting Contract. Whoever picks first plays last.",
                "Each round has its Bonus tile; one Objective tile scores at the end.",
            ],
        },
        Section {
            title: "A round",
            points: &[
                "Water drops arrive on the Headstreams, and everyone takes the incomes uncovered on their Company board.",
                "In turn order, place Engineers on one action space per turn (Company board or Management board, paying its Credits) until everyone has passed.",
                "Then the Headstreams' water flows down, and the round is scored.",
            ],
        },
        Section {
            title: "Your turn",
            points: &[
                "Build (your Company board): put a Technology tile and the Excavators or Concrete Mixers it needs on the Construction Wheel, turn it, and place the piece. They come back once the wheel has turned all the way.",
                "Base: 5, 4 or 3 Excavators (Mountains, Hills, Plains). Elevation: 4, 3 or 2 Mixers. Conduit: 2 Excavators per production value. Powerhouse: 2 Mixers + 1 per Powerhouse built. Red spaces cost 3 Credits more.",
                "Produce (Turbine Station): drops from your Dam or a neutral one run through a Conduit to your Powerhouse; each makes the Conduit's value. Someone else's Conduit costs 1 Credit per drop. One Contract may be fulfilled with that energy.",
                "Management board: Water Management, the Bank (1 Credit per Engineer), the Workshop, the Machinery Shop, the Contract Office and the Patent Office (Advanced tiles).",
                "Water runs down one drop at a time: past Powerhouses, into the upper Dam, then the lower one, then on.",
            ],
        },
        Section {
            title: "Scoring",
            points: &[
                "Each round, by the energy you produced: 6 and 2 VP to the two best producers (at least 1 energy); Credits from the Energy Track (none: −3 VP); the round's Bonus tile from 6 energy, 4 VP less per section short of the round's.",
                "Contracts and building rows give VP and incomes as you go.",
            ],
        },
        Section {
            title: "End of game",
            points: &[
                "After round 5: the Objective tile (15, 10, 5 VP), 1 VP per 5 Credits and Machineries, 1 VP per drop in your Dams.",
                "The most VP win; a tie goes to the most energy in the last round.",
            ],
        },
    ],
    links: &[("Barrage rulebook (2019, English)", "https://www.boardgame.bg/barrage%20rules.pdf")],
};
