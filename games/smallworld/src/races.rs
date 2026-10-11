//! The races and special powers: the base game's 14 and 20, and those of the
//! expansions the host can add in the lobby (see [`Pack`]), with the number
//! of tokens each banner and badge is worth and what each one does (in the
//! words of the rules, shortened).

use serde::{Deserialize, Serialize};

/// Where a race or power comes from: the base game, or one of the
/// expansions the host can switch on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Pack {
    Base,
    Cursed,
    GrandDames,
    BeNotAfraid,
    RoyalBonus,
    SpidersWeb,
    SkyIslands,
}

impl Pack {
    /// The expansions, in the order the lobby lists them.
    pub const EXPANSIONS: [Pack; 6] =
        [Pack::Cursed, Pack::GrandDames, Pack::BeNotAfraid, Pack::RoyalBonus, Pack::SpidersWeb, Pack::SkyIslands];

    pub fn name(self) -> &'static str {
        match self {
            Pack::Base => "Small World",
            Pack::Cursed => "Cursed!",
            Pack::GrandDames => "Grand Dames of Small World",
            Pack::BeNotAfraid => "Be Not Afraid...",
            Pack::RoyalBonus => "Royal Bonus",
            Pack::SpidersWeb => "A Spider's Web",
            Pack::SkyIslands => "Sky Islands",
        }
    }
}

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
    // Cursed!
    Goblins,
    Kobolds,
    // Grand Dames of Small World
    Gypsies,
    Priestesses,
    WhiteLadies,
    // Be Not Afraid...
    Barbarians,
    Homunculi,
    Leprechauns,
    Pixies,
    Pygmies,
    // Royal Bonus
    Fauns,
    Igors,
    Shrubmen,
    // A Spider's Web
    IceWitches,
    Skags,
    Slingmen,
    // Sky Islands
    Escargots,
    Scarecrows,
    Khans,
    Scavengers,
    StormGiants,
    Drakons,
    Wendigos,
}

impl Race {
    pub const ALL: [Race; 37] = [
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
        Race::Goblins,
        Race::Kobolds,
        Race::Gypsies,
        Race::Priestesses,
        Race::WhiteLadies,
        Race::Barbarians,
        Race::Homunculi,
        Race::Leprechauns,
        Race::Pixies,
        Race::Pygmies,
        Race::Fauns,
        Race::Igors,
        Race::Shrubmen,
        Race::IceWitches,
        Race::Skags,
        Race::Slingmen,
        Race::Escargots,
        Race::Scarecrows,
        Race::Khans,
        Race::Scavengers,
        Race::StormGiants,
        Race::Drakons,
        Race::Wendigos,
    ];

    pub fn pack(self) -> Pack {
        match self {
            Race::Goblins | Race::Kobolds => Pack::Cursed,
            Race::Gypsies | Race::Priestesses | Race::WhiteLadies => Pack::GrandDames,
            Race::Barbarians | Race::Homunculi | Race::Leprechauns | Race::Pixies | Race::Pygmies => Pack::BeNotAfraid,
            Race::Fauns | Race::Igors | Race::Shrubmen => Pack::RoyalBonus,
            Race::IceWitches | Race::Skags | Race::Slingmen => Pack::SpidersWeb,
            Race::Escargots
            | Race::Scarecrows
            | Race::Khans
            | Race::Scavengers
            | Race::StormGiants
            | Race::Drakons
            | Race::Wendigos => Pack::SkyIslands,
            _ => Pack::Base,
        }
    }

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
            Race::Goblins => "Goblins",
            Race::Kobolds => "Kobolds",
            Race::Gypsies => "Gypsies",
            Race::Priestesses => "Priestesses",
            Race::WhiteLadies => "White Ladies",
            Race::Barbarians => "Barbarians",
            Race::Homunculi => "Homunculi",
            Race::Leprechauns => "Leprechauns",
            Race::Pixies => "Pixies",
            Race::Pygmies => "Pygmies",
            Race::Fauns => "Fauns",
            Race::Igors => "Igors",
            Race::Shrubmen => "Shrubmen",
            Race::IceWitches => "Ice Witches",
            Race::Skags => "Skags",
            Race::Slingmen => "Slingmen",
            Race::Escargots => "Escargots",
            Race::Scarecrows => "Scarecrows",
            Race::Khans => "Khans",
            Race::Scavengers => "Scavengers",
            Race::StormGiants => "Storm Giants",
            Race::Drakons => "Drakons",
            Race::Wendigos => "Wendigos",
        }
    }

    /// The number on the banner: tokens taken with the race (Amazons' four
    /// extra conquest-only tokens included).
    pub fn tokens(self) -> u8 {
        match self {
            Race::WhiteLadies => 2,
            Race::Dwarves => 3,
            Race::Priestesses | Race::Igors => 4,
            Race::Ghouls
            | Race::Humans
            | Race::Orcs
            | Race::Sorcerers
            | Race::Trolls
            | Race::Wizards
            | Race::Homunculi
            | Race::Fauns
            | Race::IceWitches
            | Race::Slingmen
            | Race::Khans => 5,
            Race::Elves
            | Race::Giants
            | Race::Halflings
            | Race::Skeletons
            | Race::Tritons
            | Race::Goblins
            | Race::Gypsies
            | Race::Leprechauns
            | Race::Pygmies
            | Race::Shrubmen
            | Race::Skags
            | Race::Scavengers
            | Race::StormGiants
            | Race::Drakons
            | Race::Wendigos => 6,
            Race::Ratmen => 8,
            Race::Barbarians => 9,
            Race::Amazons => 10,
            Race::Kobolds | Race::Pixies => 11,
            Race::Escargots | Race::Scarecrows => 12,
        }
    }

    /// How many tokens of the race the box holds: nobody can ever have more
    /// on the board and in hand. A race that never gains tokens has its
    /// banner's number plus five (the most a badge adds); see
    /// `docs/small-world.md` for the counts that were estimated.
    pub fn supply(self) -> u8 {
        match self {
            Race::Amazons => 15,
            Race::Skeletons => 20,
            Race::Sorcerers => 18,
            Race::Homunculi => 11,
            Race::Pygmies => 15,
            Race::Fauns | Race::Igors => 15,
            Race::Wendigos => 17,
            other => other.tokens() + 5,
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
            Race::Tritons => "Conquer coastal regions (next to a sea or a lake) with 1 token less.",
            Race::Trolls => "A Troll's Lair in each region adds 1 to its defense, and stays in decline.",
            Race::Wizards => "+1 coin per Magic region.",
            Race::Goblins => "Conquer regions held by a race in decline with 1 token less.",
            Race::Kobolds => "Never fewer than 2 Kobolds in a region (but 1 per region in decline, as usual).",
            Race::Gypsies => "+1 coin for each region you abandon; you can't take it back this turn.",
            Race::Priestesses => "In decline, one Priestess per region is stacked in a single Ivory Tower, worth 1 coin per Priestess each turn.",
            Race::WhiteLadies => "In decline, immune to everyone's conquests and powers.",
            Race::Barbarians => "No redeploying: tokens left in hand sit out until your next turn.",
            Race::Homunculi => "Every time someone takes a combo below yours, it gets a Homunculus token as well as a coin.",
            Race::Leprechauns => "Put a Pot of Gold in your regions when you redeploy: each one still there at your next turn is a coin; whoever takes the region gets it instead.",
            Race::Pixies => "Only one Pixie stays in each region after redeploying; the rest sit out until your next turn.",
            Race::Pygmies => "Every time you lose a Pygmy, roll the die and get that many new ones.",
            Race::Fauns => "+1 token for each region of an active race you conquer this turn; your victims get one too.",
            Race::Igors => "Collect every token lost in conquests; at the start of your turn, trade them for new Igors (as many as there are players for one).",
            Race::Shrubmen => "Your Forest regions are immune, even in decline.",
            Race::IceWitches => "A Winter marker for each Magic Source you hold, to put in or next to your regions: +1 defense, and 1 coin less to anyone else holding it.",
            Race::Skags => "A face-down Loot marker in each region you conquer: whoever takes the region gets its coins, or a Skag Attack that stops them. Yours at decline or the end.",
            Race::Slingmen => "Conquer a region two away that none of yours touches (not over a sea), and get a coin for it.",
            Race::Escargots => "Your regions' coins come at the start of your turn, not at the end.",
            Race::Scarecrows => "Whoever conquers one of your regions gets a coin.",
            Race::Khans => "+1 coin per Hill or Farmland region, 1 coin less for every other region.",
            Race::Scavengers => "Regions you take from a race in decline keep its token: it adds to the defense and still scores for its owner.",
            Race::StormGiants => "Two Lightning Bolts a turn: each takes a Mountain with a single token, whatever defends it. May start in the Sky Islands.",
            Race::Drakons => "Turn up to 3 tokens a turn into Dragons, each conquering a region with one token, which stays immune until your next turn.",
            Race::Wendigos => "Once a turn, clear a Forest region anywhere of its tokens, and get a new Wendigo.",
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
    // Cursed!
    Cursed,
    HordesOf,
    Marauding,
    Ransacking,
    Were,
    // Grand Dames of Small World
    Historian,
    PeaceLoving,
    // Be Not Afraid...
    Barricade,
    Catapult,
    Corrupt,
    Imperial,
    Mercenary,
    // Royal Bonus
    Aquatic,
    Behemoth,
    Fireball,
    // A Spider's Web
    Copycat,
    Lava,
    SoulTouch,
    // Sky Islands
    Goldsmith,
    Zeppelined,
    Airborne,
    Haggling,
    Gunner,
    Exploring,
    Racketeering,
}

impl Power {
    pub const ALL: [Power; 45] = [
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
        Power::Cursed,
        Power::HordesOf,
        Power::Marauding,
        Power::Ransacking,
        Power::Were,
        Power::Historian,
        Power::PeaceLoving,
        Power::Barricade,
        Power::Catapult,
        Power::Corrupt,
        Power::Imperial,
        Power::Mercenary,
        Power::Aquatic,
        Power::Behemoth,
        Power::Fireball,
        Power::Copycat,
        Power::Lava,
        Power::SoulTouch,
        Power::Goldsmith,
        Power::Zeppelined,
        Power::Airborne,
        Power::Haggling,
        Power::Gunner,
        Power::Exploring,
        Power::Racketeering,
    ];

    pub fn pack(self) -> Pack {
        match self {
            Power::Cursed | Power::HordesOf | Power::Marauding | Power::Ransacking | Power::Were => Pack::Cursed,
            Power::Historian | Power::PeaceLoving => Pack::GrandDames,
            Power::Barricade | Power::Catapult | Power::Corrupt | Power::Imperial | Power::Mercenary => Pack::BeNotAfraid,
            Power::Aquatic | Power::Behemoth | Power::Fireball => Pack::RoyalBonus,
            Power::Copycat | Power::Lava | Power::SoulTouch => Pack::SpidersWeb,
            Power::Goldsmith
            | Power::Zeppelined
            | Power::Airborne
            | Power::Haggling
            | Power::Gunner
            | Power::Exploring
            | Power::Racketeering => Pack::SkyIslands,
            _ => Pack::Base,
        }
    }

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
            Power::Cursed => "Cursed",
            Power::HordesOf => "Hordes of",
            Power::Marauding => "Marauding",
            Power::Ransacking => "Ransacking",
            Power::Were => "Were-",
            Power::Historian => "Historian",
            Power::PeaceLoving => "Peace-Loving",
            Power::Barricade => "Barricade",
            Power::Catapult => "Catapult",
            Power::Corrupt => "Corrupt",
            Power::Imperial => "Imperial",
            Power::Mercenary => "Mercenary",
            Power::Aquatic => "Aquatic",
            Power::Behemoth => "Behemoth",
            Power::Fireball => "Fireball",
            Power::Copycat => "Copycat",
            Power::Lava => "Lava",
            Power::SoulTouch => "Soul-Touch",
            Power::Goldsmith => "Goldsmith",
            Power::Zeppelined => "Zeppelined",
            Power::Airborne => "Airborne",
            Power::Haggling => "Haggling",
            Power::Gunner => "Gunner",
            Power::Exploring => "Exploring",
            Power::Racketeering => "Racketeering",
        }
    }

    /// The number on the badge: tokens it adds to the race's.
    pub fn tokens(self) -> u8 {
        match self {
            Power::Cursed => 0,
            Power::Merchant => 2,
            Power::Fortified | Power::Aquatic | Power::Copycat | Power::Airborne | Power::Gunner => 3,
            Power::Alchemist
            | Power::Berserk
            | Power::Commando
            | Power::Forest
            | Power::Hill
            | Power::Stout
            | Power::Swamp
            | Power::Wealthy
            | Power::Ransacking
            | Power::Were
            | Power::PeaceLoving
            | Power::Barricade
            | Power::Catapult
            | Power::Corrupt
            | Power::Imperial
            | Power::Mercenary
            | Power::Behemoth
            | Power::SoulTouch
            | Power::Goldsmith => 4,
            Power::Bivouacking
            | Power::Diplomat
            | Power::DragonMaster
            | Power::Flying
            | Power::Heroic
            | Power::Mounted
            | Power::Pillaging
            | Power::Seafaring
            | Power::Spirit
            | Power::Underworld
            | Power::HordesOf
            | Power::Marauding
            | Power::Historian
            | Power::Fireball
            | Power::Lava
            | Power::Zeppelined
            | Power::Haggling
            | Power::Exploring
            | Power::Racketeering => 5,
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
            Power::Flying => "Conquer any region of the map, not only next to yours (except the seas and the lakes).",
            Power::Forest => "+1 coin per Forest region.",
            Power::Fortified => "Once per turn, build a fortress in one of your regions: +1 to its defense, +1 coin a turn while active (6 at most).",
            Power::Heroic => "At the end of your turn, your 2 Heroes make 2 of your regions immune.",
            Power::Hill => "+1 coin per Hill region.",
            Power::Merchant => "+1 coin per region.",
            Power::Mounted => "Conquer Hills and Farmlands with 1 token less.",
            Power::Pillaging => "+1 coin per non-empty region conquered this turn.",
            Power::Seafaring => "Conquer the seas and the lakes as empty regions, and keep them in decline.",
            Power::Spirit => "In decline, this race doesn't count toward your one race in decline: you can have two.",
            Power::Stout => "You may go into decline at the end of a turn of conquests, after scoring.",
            Power::Swamp => "+1 coin per Swamp region.",
            Power::Underworld => "Conquer Caverns with 1 token less; all Caverns are next to each other for you.",
            Power::Wealthy => "+7 coins, once, at the end of your first turn.",
            Power::Cursed => "No power, and no tokens: anyone who skips this combo pays 3 coins on it, not 1.",
            Power::HordesOf => "2 Hordes tokens on top, used like your own; they vanish when you go into decline.",
            Power::Marauding => "When your conquests are over, take your troops back in hand (one stays in each region) and conquer again.",
            Power::Ransacking => "Whoever's active region you conquer pays you a coin.",
            Power::Were => "On even turns (the nights), conquer any region with 2 tokens less.",
            Power::Historian => "+1 coin for each race in decline when you pick it, +1 whenever another race goes into decline, +1 when yours does.",
            Power::PeaceLoving => "+3 coins at the end of each turn you attacked no active race.",
            Power::Barricade => "+3 coins at the end of each turn you hold 4 regions or fewer.",
            Power::Catapult => "Once per turn, conquer a region two away (not over a sea) with 1 token less; the Catapult makes the region it fired from immune.",
            Power::Corrupt => "Whoever conquers one of your active regions pays you a coin.",
            Power::Imperial => "+1 coin for each region beyond 3.",
            Power::Mercenary => "Pay a coin to conquer a region with 2 tokens less.",
            Power::Aquatic => "+1 coin per coastal region, 1 coin less for every other region.",
            Power::Behemoth => "2 Behemoths, each worth as many tokens as you hold Swamps, to attack and defend with.",
            Power::Fireball => "When you redeploy, a Fireball for each Magic Source you hold (4 at most): each counts as 2 tokens in a later conquest.",
            Power::Copycat => "At the start of your turn, copy the power of one of the six combos until your next turn.",
            Power::Lava => "At the end of your turn, pour Lava next to each of your Mountains: whoever was there takes their tokens back, and nobody may enter until your next turn.",
            Power::SoulTouch => "Going into decline brings your race in decline back to life, with all its tokens and its power, for your next turn.",
            Power::Goldsmith => "+2 coins per Mine region, 1 coin less for every other region.",
            Power::Zeppelined => "Up to 5 Zeppelin conquests a turn, anywhere on the board: roll the die first, each pip is a token; a blank burns the region.",
            Power::Airborne => "On the turn you pick it, conquer any region with 2 tokens less.",
            Power::Haggling => "Hand out 5 Trade Pacts: whoever holds some pays you a coin each if they conquer one of your regions on their turn, or gets one each from you if they don't.",
            Power::Gunner => "2 Cannons make your regions immune; a firing Cannon makes conquests next to it 2 tokens cheaper.",
            Power::Exploring => "+1 coin for each region you hold on the Ground and in the Sky, whichever you hold fewer of.",
            Power::Racketeering => "Whenever someone picks a combo, you get a coin for each one they skip; your next race is free.",
        }
    }
}
