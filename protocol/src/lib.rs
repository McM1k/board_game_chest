//! Messages exchanged over the WebSocket between the browser and the server,
//! shared by every game.
//!
//! Rooms work the same way for all games: one player creates a room and gets
//! a 4-letter code, the others join with that code, the host starts the game,
//! and a per-seat token lets a player come back after a dropped connection.
//! The rest differs per game and travels in four enums with one variant per
//! game: [`GameKind`] (which game), [`GameOptions`] (what the host picks in
//! the lobby), [`GameMove`] (a move) and [`GameView`] (the running game as
//! one seat sees it). Adding a game adds a variant to each of them (see
//! `docs/adding-a-game.md`).

use serde::{Deserialize, Serialize};

/// The games the server can host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameKind {
    Catan,
    Hanabi,
    /// It's a Wonderful World, base game.
    Wonderful,
    SmallWorld,
}

impl GameKind {
    pub const ALL: [GameKind; 4] = [GameKind::Catan, GameKind::Hanabi, GameKind::Wonderful, GameKind::SmallWorld];

    pub fn title(self) -> &'static str {
        match self {
            GameKind::Catan => "Catan",
            GameKind::Hanabi => "Hanabi",
            GameKind::Wonderful => "It's a Wonderful World",
            GameKind::SmallWorld => "Small World",
        }
    }

    pub fn min_players(self) -> usize {
        2
    }

    /// The most players the game ever seats, with every option on.
    pub fn max_players(self) -> usize {
        match self {
            GameKind::Catan => 4,
            GameKind::Hanabi => 5,
            GameKind::Wonderful => 7,
            GameKind::SmallWorld => smallworld_core::MAX_PLAYERS,
        }
    }

    /// The most players a game with these lobby options seats.
    pub fn max_players_with(self, options: Option<&GameOptions>) -> usize {
        options.and_then(GameOptions::max_players).unwrap_or(self.max_players())
    }

    /// The fewest players a game with these lobby options needs.
    pub fn min_players_with(self, options: Option<&GameOptions>) -> usize {
        options.and_then(GameOptions::min_players).unwrap_or(self.min_players())
    }
}

/// What the host picked in the lobby, for a game that has options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GameOptions {
    /// Hanabi's variant rules. The server normalizes them: the "hanabii"
    /// mode replaces every other option.
    Hanabi(hanabi_core::GameRules),
    /// It's a Wonderful World's expansion.
    Wonderful(wonderful_core::Options),
    /// Small World's expansions.
    SmallWorld(smallworld_core::Options),
}

impl GameOptions {
    pub fn game(&self) -> GameKind {
        match self {
            GameOptions::Hanabi(_) => GameKind::Hanabi,
            GameOptions::Wonderful(_) => GameKind::Wonderful,
            GameOptions::SmallWorld(_) => GameKind::SmallWorld,
        }
    }

    /// The most players these options allow, when fewer than the game's
    /// maximum (It's a Wonderful World seats 7 only with its expansion,
    /// Small World 6 only with the Sky Islands).
    pub fn max_players(&self) -> Option<usize> {
        match self {
            GameOptions::Hanabi(_) => None,
            GameOptions::Wonderful(options) => Some(options.max_players()),
            GameOptions::SmallWorld(options) => Some(options.max_players()),
        }
    }

    /// The fewest players these options need, when more than the game's
    /// minimum (Small World's Sky Islands need 3).
    pub fn min_players(&self) -> Option<usize> {
        match self {
            GameOptions::SmallWorld(options) => Some(options.min_players()),
            _ => None,
        }
    }
}

/// A move in a running game.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GameMove {
    Catan(catan_core::Action),
    Hanabi(hanabi_core::Action),
    /// Everybody moves at the same time, so the server checks a move against
    /// the rules only, never against a turn.
    Wonderful(wonderful_core::Action),
    /// Checked against whose turn it is, except for the tokens a player who
    /// lost regions puts back after someone else's turn.
    SmallWorld(smallworld_core::Action),
}

impl GameMove {
    pub fn game(&self) -> GameKind {
        match self {
            GameMove::Catan(_) => GameKind::Catan,
            GameMove::Hanabi(_) => GameKind::Hanabi,
            GameMove::Wonderful(_) => GameKind::Wonderful,
            GameMove::SmallWorld(_) => GameKind::SmallWorld,
        }
    }
}

/// A running game as one seat sees it: what the others hide stays hidden.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GameView {
    Catan(Box<catan_core::GameView>),
    Hanabi(Box<hanabi_core::PlayerView>),
    Wonderful(Box<wonderful_core::View>),
    SmallWorld(Box<smallworld_core::View>),
}

impl GameView {
    pub fn game(&self) -> GameKind {
        match self {
            GameView::Catan(_) => GameKind::Catan,
            GameView::Hanabi(_) => GameKind::Hanabi,
            GameView::Wonderful(_) => GameKind::Wonderful,
            GameView::SmallWorld(_) => GameKind::SmallWorld,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMsg {
    /// Open a new room for `game` and sit down in it.
    Create { game: GameKind, name: String },
    /// Join (or, with a token, rejoin) a room. The room's code decides which
    /// game you end up in.
    Join {
        room: String,
        name: String,
        token: Option<String>,
    },
    /// Host only, in a lobby: choose the game's options. The server checks
    /// them and echoes what will actually be played back to everyone.
    SetOptions(GameOptions),
    /// Host only: start the game once enough players are seated.
    Start,
    /// Leave the room for good (in a lobby this frees the seat).
    Leave,
    /// A move in the running game.
    Move(GameMove),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LobbyPlayer {
    pub name: String,
    pub connected: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMsg {
    /// Sent once after a successful Create/Join. Keep `token` to reconnect.
    Joined { room: String, token: String },
    /// The waiting room, sent to everyone whenever it changes.
    Lobby {
        room: String,
        game: GameKind,
        players: Vec<LobbyPlayer>,
        you: usize,
        is_host: bool,
        /// The options picked so far (`None` for a game that has none).
        options: Option<GameOptions>,
    },
    /// The running game, personalised for the receiving seat. `names` and
    /// `connected` are indexed by seat.
    Game {
        view: GameView,
        names: Vec<String>,
        connected: Vec<bool>,
    },
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use hanabi_core::{GameRules, GameState, PlayerId};
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn game_limits() {
        assert_eq!(GameKind::Catan.max_players(), 4);
        assert_eq!(GameKind::Hanabi.max_players(), 5);
        assert_eq!(GameKind::Wonderful.max_players(), 7);
        // Lobby options can lower it: It's a Wonderful World seats 7 only
        // with its expansion.
        let base = GameOptions::Wonderful(wonderful_core::Options::default());
        let expanded = GameOptions::Wonderful(wonderful_core::Options { ascension: true });
        assert_eq!(GameKind::Wonderful.max_players_with(Some(&base)), 5);
        assert_eq!(GameKind::Wonderful.max_players_with(Some(&expanded)), 7);
        let hanabi = GameOptions::Hanabi(GameRules::default());
        assert_eq!(GameKind::Hanabi.max_players_with(Some(&hanabi)), 5);
        assert_eq!(GameKind::Catan.max_players_with(None), 4);
        assert_eq!(GameKind::SmallWorld.max_players(), 6);
        let base = GameOptions::SmallWorld(smallworld_core::Options::default());
        let sky = GameOptions::SmallWorld(smallworld_core::Options::all());
        assert_eq!(GameKind::SmallWorld.max_players_with(Some(&base)), 5);
        assert_eq!(GameKind::SmallWorld.max_players_with(Some(&sky)), 6);
        assert_eq!(GameKind::SmallWorld.min_players_with(Some(&base)), 2);
        assert_eq!(GameKind::SmallWorld.min_players_with(Some(&sky)), 3);
        assert_eq!(GameKind::Hanabi.min_players_with(Some(&hanabi)), 2);
        assert_eq!(GameKind::ALL.len(), 4);
        for g in GameKind::ALL {
            assert_eq!(g.min_players(), 2);
            assert!(!g.title().is_empty());
        }
    }

    #[test]
    fn client_messages_round_trip() {
        let msgs = vec![
            ClientMsg::Create { game: GameKind::Hanabi, name: "Ann".into() },
            ClientMsg::Join { room: "ABCD".into(), name: "Bob".into(), token: Some("t".into()) },
            ClientMsg::SetOptions(GameOptions::Hanabi(GameRules { hanabii: true, ..Default::default() })),
            ClientMsg::Start,
            ClientMsg::Leave,
            ClientMsg::Move(GameMove::Catan(catan_core::Action::EndTurn)),
            ClientMsg::Move(GameMove::Hanabi(hanabi_core::Action::Play { card_id: hanabi_core::CardId(3) })),
            ClientMsg::Move(GameMove::Hanabi(hanabi_core::Action::Clue {
                target: PlayerId(1),
                clue: hanabi_core::Clue::Color(hanabi_core::Color::Red),
            })),
            ClientMsg::Create { game: GameKind::Wonderful, name: "Cy".into() },
            ClientMsg::Move(GameMove::Wonderful(wonderful_core::Action::Draft { card: wonderful_core::CardId(7) })),
            ClientMsg::Move(GameMove::Wonderful(wonderful_core::Action::Recycle {
                card: wonderful_core::CardId(8),
                to: wonderful_core::Target::Empire,
            })),
            ClientMsg::Move(GameMove::Wonderful(wonderful_core::Action::Place {
                piece: wonderful_core::Piece::Krystallium(Some(wonderful_core::Res::Gold)),
                target: wonderful_core::Target::Card(wonderful_core::CardId(9)),
            })),
            ClientMsg::Move(GameMove::Wonderful(wonderful_core::Action::Ready)),
            ClientMsg::SetOptions(GameOptions::Wonderful(wonderful_core::Options { ascension: true })),
            ClientMsg::Create { game: GameKind::SmallWorld, name: "Di".into() },
            ClientMsg::Move(GameMove::SmallWorld(smallworld_core::Action::Pick { offer: 2 })),
            ClientMsg::Move(GameMove::SmallWorld(smallworld_core::Action::Conquer { region: 12 })),
            ClientMsg::Move(GameMove::SmallWorld(smallworld_core::Action::Ally { seat: Some(1) })),
            ClientMsg::Move(GameMove::SmallWorld(smallworld_core::Action::EndTurn)),
            ClientMsg::Move(GameMove::SmallWorld(smallworld_core::Action::Copy { offer: Some(3) })),
            ClientMsg::Move(GameMove::SmallWorld(smallworld_core::Action::Pact { seat: 2, give: true })),
            ClientMsg::SetOptions(GameOptions::SmallWorld(smallworld_core::Options::all())),
        ];
        for m in msgs {
            let json = serde_json::to_string(&m).unwrap();
            let back: ClientMsg = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{m:?}"), format!("{back:?}"), "{json}");
        }
    }

    #[test]
    fn every_move_view_and_option_knows_its_game() {
        let mut rng = StdRng::seed_from_u64(3);
        let catan = catan_core::Game::new(catan_core::Rules::default(), vec!["A".into(), "B".into()], &mut rng);
        let hanabi = GameState::new(2, 1, GameRules::default());
        let wonderful = wonderful_core::State::new(2, 1);
        let small_world = smallworld_core::State::new(2, 1);
        let views = [
            GameView::Catan(Box::new(catan.view_for(Some(0)))),
            GameView::Hanabi(Box::new(hanabi.view_for(PlayerId(0)))),
            GameView::Wonderful(Box::new(wonderful.view_for(0))),
            GameView::SmallWorld(Box::new(small_world.view_for(0))),
        ];
        let moves = [
            GameMove::Catan(catan_core::Action::EndTurn),
            GameMove::Hanabi(hanabi_core::Action::Discard { card_id: hanabi_core::CardId(0) }),
            GameMove::Wonderful(wonderful_core::Action::Ready),
            GameMove::SmallWorld(smallworld_core::Action::EndConquests),
        ];
        for ((view, mv), kind) in views.iter().zip(&moves).zip(GameKind::ALL) {
            assert_eq!((view.game(), mv.game()), (kind, kind));
        }
        assert_eq!(GameOptions::Hanabi(GameRules::default()).game(), GameKind::Hanabi);
        assert_eq!(GameOptions::Wonderful(wonderful_core::Options::default()).game(), GameKind::Wonderful);
        assert_eq!(GameOptions::SmallWorld(smallworld_core::Options::default()).game(), GameKind::SmallWorld);
    }

    /// Unwraps a `ServerMsg::Game` that went through JSON.
    fn game_through_json(msg: &ServerMsg) -> (GameView, Vec<String>, Vec<bool>) {
        let json = serde_json::to_string(msg).unwrap();
        let ServerMsg::Game { view, names, connected } = serde_json::from_str(&json).unwrap() else {
            panic!("wrong variant: {json}");
        };
        (view, names, connected)
    }

    #[test]
    fn catan_state_survives_json() {
        let mut rng = StdRng::seed_from_u64(3);
        let names = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let game = catan_core::Game::new(catan_core::Rules::default(), names.clone(), &mut rng);
        let msg = ServerMsg::Game {
            view: GameView::Catan(Box::new(game.view_for(Some(1)))),
            names,
            connected: vec![true, false, true],
        };
        let (GameView::Catan(view), _, connected) = game_through_json(&msg) else {
            panic!("not a Catan view");
        };
        assert_eq!(view.seat, Some(1));
        assert_eq!(view.players.len(), 3);
        assert_eq!(connected, vec![true, false, true]);
    }

    /// The Hanabi view keys maps by `PlayerId` and `Color`; JSON only allows
    /// string keys, so make sure a whole state message survives the trip.
    #[test]
    fn hanabi_state_survives_json() {
        let rules = GameRules { multicolor: true, black: true, ..Default::default() };
        let game = GameState::new(3, 11, rules);
        let msg = ServerMsg::Game {
            view: GameView::Hanabi(Box::new(game.view_for(PlayerId(1)))),
            names: vec!["A".into(), "B".into(), "C".into()],
            connected: vec![true, true, false],
        };
        let (GameView::Hanabi(view), names, connected) = game_through_json(&msg) else {
            panic!("not a Hanabi view");
        };
        assert_eq!(view.you, PlayerId(1));
        assert_eq!(view.hands.len(), 3);
        assert_eq!(view.fireworks.len(), game.fireworks.len());
        assert_eq!(view.rules, rules);
        assert_eq!(names.len(), 3);
        assert_eq!(connected, vec![true, true, false]);
        // Your own hand is redacted on the wire.
        assert!(view.hands[&PlayerId(1)].iter().all(|c| c.card.is_none()));
        assert!(view.hands[&PlayerId(0)].iter().all(|c| c.card.is_some()));
    }

    /// The Wonderful World view has no maps, and hides other players' drafts.
    #[test]
    fn wonderful_state_survives_json() {
        let mut game = wonderful_core::State::new(3, 5);
        let card = game.players[1].hand[0];
        game.apply(1, wonderful_core::Action::Draft { card }).unwrap();
        let msg = ServerMsg::Game {
            view: GameView::Wonderful(Box::new(game.view_for(2))),
            names: vec!["A".into(), "B".into(), "C".into()],
            connected: vec![true, false, true],
        };
        let (GameView::Wonderful(view), names, connected) = game_through_json(&msg) else {
            panic!("not a Wonderful World view");
        };
        assert_eq!(*view, game.view_for(2));
        assert_eq!(view.you, 2);
        assert_eq!(view.hand.len(), 7);
        // Seat 1 has picked, but nobody else can tell which card.
        assert!(view.players[1].picked && view.picked.is_none());
        assert_eq!(names.len(), 3);
        assert_eq!(connected, vec![true, false, true]);
    }

    /// The Small World view hides the other players' coins.
    #[test]
    fn small_world_state_survives_json() {
        let mut game = smallworld_core::State::new(3, 8);
        game.apply(0, smallworld_core::Action::Pick { offer: 1 }).unwrap();
        let msg = ServerMsg::Game {
            view: GameView::SmallWorld(Box::new(game.view_for(1))),
            names: vec!["A".into(), "B".into(), "C".into()],
            connected: vec![true, true, false],
        };
        let (GameView::SmallWorld(view), names, _) = game_through_json(&msg) else {
            panic!("not a Small World view");
        };
        assert_eq!(*view, game.view_for(1));
        assert_eq!((view.you, view.board, view.regions.len()), (1, 3, 30));
        assert_eq!(view.players[1].coins, Some(smallworld_core::START_COINS));
        assert_eq!(view.players[0].coins, None);
        assert!(view.players[0].active.is_some());
        assert_eq!(names.len(), 3);

        // With every expansion: the Sky Islands beside the board, six seats.
        let game = smallworld_core::State::with_options(6, 8, smallworld_core::Options::all());
        let msg = ServerMsg::Game {
            view: GameView::SmallWorld(Box::new(game.view_for(5))),
            names: (0..6).map(|i| format!("P{i}")).collect(),
            connected: vec![true; 6],
        };
        let (GameView::SmallWorld(view), _, _) = game_through_json(&msg) else {
            panic!("not a Small World view");
        };
        assert_eq!(*view, game.view_for(5));
        assert_eq!((view.board, view.access.len(), view.map().len()), (5, 2, 47 + 9));
    }

    #[test]
    fn lobby_message_carries_the_game_and_its_options() {
        let msg = ServerMsg::Lobby {
            room: "WXYZ".into(),
            game: GameKind::Hanabi,
            players: vec![LobbyPlayer { name: "Ann".into(), connected: true }],
            you: 0,
            is_host: true,
            options: Some(GameOptions::Hanabi(GameRules { six_cards: true, ..Default::default() })),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let ServerMsg::Lobby { game, options, players, .. } = serde_json::from_str(&json).unwrap() else {
            panic!("wrong variant: {json}");
        };
        assert_eq!(game, GameKind::Hanabi);
        let Some(GameOptions::Hanabi(rules)) = options else { panic!("no Hanabi options") };
        assert!(rules.six_cards);
        assert_eq!(players[0].name, "Ann");
    }

    #[test]
    fn rules_payloads_from_before_hanabii_existed_still_parse() {
        // `#[serde(default)]` on every field: a client that has never heard
        // of the mode just doesn't send it, and gets a normal game.
        let old: GameRules = serde_json::from_str(r#"{"multicolor":true,"extra_colors":1}"#).unwrap();
        assert!(old.multicolor && !old.hanabii);

        let bare: GameRules = serde_json::from_str("{}").unwrap();
        assert_eq!(bare, GameRules::default());

        let json = serde_json::to_string(&GameRules { hanabii: true, ..Default::default() }).unwrap();
        assert!(serde_json::from_str::<GameRules>(&json).unwrap().hanabii);
    }
}
