//! The 14 races and 20 special powers of the base game, with the number of
//! tokens each banner and badge is worth and what each one does (in the
//! words of the rules, shortened).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Race {
    Amazons,
    Dwarves,
    Elves,
    Ghouls,
    Giants,
    Halflings,
    Humans,
    Orcs,
    Ratmen,
    Skeletons,
    Sorcerers,
    Tritons,
    Trolls,
    Wizards,
}

impl Race {
    pub const ALL: [Race; 14] = [
        Race::Amazons,
        Race::Dwarves,
        Race::Elves,
        Race::Ghouls,
        Race::Giants,
        Race::Halflings,
        Race::Humans,
        Race::Orcs,
        Race::Ratmen,
        Race::Skeletons,
        Race::Sorcerers,
        Race::Tritons,
        Race::Trolls,
        Race::Wizards,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Race::Amazons => "Amazons",
            Race::Dwarves => "Dwarves",
            Race::Elves => "Elves",
            Race::Ghouls => "Ghouls",
            Race::Giants => "Giants",
            Race::Halflings => "Halflings",
            Race::Humans => "Humans",
            Race::Orcs => "Orcs",
            Race::Ratmen => "Ratmen",
            Race::Skeletons => "Skeletons",
            Race::Sorcerers => "Sorcerers",
            Race::Tritons => "Tritons",
            Race::Trolls => "Trolls",
            Race::Wizards => "Wizards",
        }
    }

    /// The number on the banner: tokens taken with the race (Amazons' four
    /// extra conquest-only tokens included).
    pub fn tokens(self) -> u8 {
        match self {
            Race::Amazons => 10,
            Race::Dwarves => 3,
            Race::Elves | Race::Giants | Race::Halflings | Race::Skeletons | Race::Tritons => 6,
            Race::Ghouls | Race::Humans | Race::Orcs | Race::Sorcerers | Race::Trolls | Race::Wizards => 5,
            Race::Ratmen => 8,
        }
    }

    /// How many tokens of the race the box holds: nobody can ever have more
    /// on the board and in hand.
    pub fn supply(self) -> u8 {
        match self {
            Race::Amazons => 15,
            Race::Dwarves => 8,
            Race::Elves | Race::Giants | Race::Halflings | Race::Tritons => 11,
            Race::Ghouls | Race::Humans | Race::Orcs | Race::Trolls | Race::Wizards => 10,
            Race::Ratmen => 13,
            Race::Skeletons => 20,
            Race::Sorcerers => 18,
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Race::Amazons => "+4 tokens for conquest only: they are set aside after each redeployment and come back at your next turn.",
            Race::Dwarves => "+1 coin per Mine region, even in decline.",
            Race::Elves => "Lose no token when an enemy conquers one of your regions.",
            Race::Ghouls => "All tokens stay when going into decline, and keep conquering at the start of your turns while in decline.",
            Race::Giants => "Conquer regions next to a Mountain you hold with 1 token less.",
            Race::Halflings => "Enter the map anywhere; your first 2 regions get a Hole-in-the-Ground and are immune.",
            Race::Humans => "+1 coin per Farmland region.",
            Race::Orcs => "+1 coin per non-empty region conquered this turn.",
            Race::Ratmen => "No power: just many tokens.",
            Race::Skeletons => "+1 token for every 2 non-empty regions conquered this turn.",
            Race::Sorcerers => "Once per turn per opponent, replace a lone active enemy token next to you with one of yours.",
            Race::Tritons => "Conquer coastal regions (next to a sea or the lake) with 1 token less.",
            Race::Trolls => "A Troll's Lair in each region adds 1 to its defense, and stays in decline.",
            Race::Wizards => "+1 coin per Magic region.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Power {
    Alchemist,
    Berserk,
    Bivouacking,
    Commando,
    Diplomat,
    DragonMaster,
    Flying,
    Forest,
    Fortified,
    Heroic,
    Hill,
    Merchant,
    Mounted,
    Pillaging,
    Seafaring,
    Spirit,
    Stout,
    Swamp,
    Underworld,
    Wealthy,
}

impl Power {
    pub const ALL: [Power; 20] = [
        Power::Alchemist,
        Power::Berserk,
        Power::Bivouacking,
        Power::Commando,
        Power::Diplomat,
        Power::DragonMaster,
        Power::Flying,
        Power::Forest,
        Power::Fortified,
        Power::Heroic,
        Power::Hill,
        Power::Merchant,
        Power::Mounted,
        Power::Pillaging,
        Power::Seafaring,
        Power::Spirit,
        Power::Stout,
        Power::Swamp,
        Power::Underworld,
        Power::Wealthy,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Power::Alchemist => "Alchemist",
            Power::Berserk => "Berserk",
            Power::Bivouacking => "Bivouacking",
            Power::Commando => "Commando",
            Power::Diplomat => "Diplomat",
            Power::DragonMaster => "Dragon Master",
            Power::Flying => "Flying",
            Power::Forest => "Forest",
            Power::Fortified => "Fortified",
            Power::Heroic => "Heroic",
            Power::Hill => "Hill",
            Power::Merchant => "Merchant",
            Power::Mounted => "Mounted",
            Power::Pillaging => "Pillaging",
            Power::Seafaring => "Seafaring",
            Power::Spirit => "Spirit",
            Power::Stout => "Stout",
            Power::Swamp => "Swamp",
            Power::Underworld => "Underworld",
            Power::Wealthy => "Wealthy",
        }
    }

    /// The number on the badge: tokens it adds to the race's.
    pub fn tokens(self) -> u8 {
        match self {
            Power::Merchant => 2,
            Power::Fortified => 3,
            Power::Alchemist
            | Power::Berserk
            | Power::Commando
            | Power::Forest
            | Power::Hill
            | Power::Stout
            | Power::Swamp
            | Power::Wealthy => 4,
            Power::Bivouacking
            | Power::Diplomat
            | Power::DragonMaster
            | Power::Flying
            | Power::Heroic
            | Power::Mounted
            | Power::Pillaging
            | Power::Seafaring
            | Power::Spirit
            | Power::Underworld => 5,
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Power::Alchemist => "+2 coins at the end of each turn, until you go into decline.",
            Power::Berserk => "Roll the reinforcement die before any conquest, not just the last one.",
            Power::Bivouacking => "5 encampments to place in your regions when you redeploy: each adds 1 to its defense.",
            Power::Commando => "Conquer any region with 1 token less.",
            Power::Diplomat => "At the end of your turn, make peace with an opponent you didn't attack: they can't attack you until your next turn.",
            Power::DragonMaster => "Once per turn, conquer a region with a single token, whatever defends it; the Dragon makes it immune.",
            Power::Flying => "Conquer any region of the map, not only next to yours (except the seas and the lake).",
            Power::Forest => "+1 coin per Forest region.",
            Power::Fortified => "Once per turn, build a fortress in one of your regions: +1 to its defense, +1 coin a turn while active (6 at most).",
            Power::Heroic => "At the end of your turn, your 2 Heroes make 2 of your regions immune.",
            Power::Hill => "+1 coin per Hill region.",
            Power::Merchant => "+1 coin per region.",
            Power::Mounted => "Conquer Hills and Farmlands with 1 token less.",
            Power::Pillaging => "+1 coin per non-empty region conquered this turn.",
            Power::Seafaring => "Conquer the seas and the lake as empty regions, and keep them in decline.",
            Power::Spirit => "In decline, this race doesn't count toward your one race in decline: you can have two.",
            Power::Stout => "You may go into decline at the end of a turn of conquests, after scoring.",
            Power::Swamp => "+1 coin per Swamp region.",
            Power::Underworld => "Conquer Caverns with 1 token less; all Caverns are next to each other for you.",
            Power::Wealthy => "+7 coins, once, at the end of your first turn.",
        }
    }
}
