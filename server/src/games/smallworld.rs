//! Small World on the server. The rules check whose move it is: the player
//! whose turn it is, or one putting back tokens they kept after losing a
//! region. Every seat is then sent its own view, with the others' coins
//! hidden.

use super::{Game, Running, WRONG_GAME};
use protocol::{GameMove, GameOptions, GameView};
use rand::{Rng, RngCore};
use smallworld_core::{Options, Power, Race, State};

pub struct SmallWorld;

impl Game for SmallWorld {
    /// Seats are shuffled for a random first player.
    fn shuffles_seats(&self) -> bool {
        true
    }

    /// The base game, until the host ticks some expansions.
    fn default_options(&self) -> Option<GameOptions> {
        Some(GameOptions::SmallWorld(Options::default()))
    }

    fn check_options(&self, options: GameOptions) -> Result<GameOptions, String> {
        match options {
            GameOptions::SmallWorld(_) => Ok(options),
            _ => Err("Those options are for another game.".into()),
        }
    }

    fn start(&self, names: Vec<String>, options: Option<&GameOptions>, rng: &mut dyn RngCore) -> Box<dyn Running> {
        let mut options = match options {
            Some(GameOptions::SmallWorld(options)) => *options,
            _ => Options::default(),
        };
        // The rooms check the player count; this only guards the rules.
        if names.len() < options.min_players() {
            options.sky = None;
        }
        let mut state = State::with_options(names.len(), rng.gen(), options);
        if let Ok(Some((race, power))) = top_combo() {
            if !state.put_on_top(race, power) {
                eprintln!("{TOP_VAR}: not in this game (is its expansion on?), so the column is left as dealt");
            }
        }
        Box::new(state)
    }
}

/// For trying a combo out: `SMALLWORLD_TOP=Ghouls,Spirit` puts it on top
/// of the column, the free combo, in every new game. A race alone
/// (`Ghouls`) or a power alone (`,Spirit`) works too.
pub const TOP_VAR: &str = "SMALLWORLD_TOP";

/// A race, a power, or both.
pub type Combo = (Option<Race>, Option<Power>);

/// The combo `SMALLWORLD_TOP` asks for, if it is set.
pub fn top_combo() -> Result<Option<Combo>, String> {
    match std::env::var(TOP_VAR) {
        Ok(spec) if !spec.trim().is_empty() => parse_combo(&spec).map(Some),
        _ => Ok(None),
    }
}

/// Reads `race,power`, either of them left out. Case, spaces and hyphens
/// don't matter: `storm giants,hordes of` and `Trolls,Were` are fine.
fn parse_combo(spec: &str) -> Result<Combo, String> {
    let key = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
    let (race, power) = spec.split_once(',').unwrap_or((spec, ""));
    let race = match key(race) {
        k if k.is_empty() => None,
        k => Some(
            Race::ALL.into_iter().find(|r| key(r.name()) == k).ok_or(format!("no race called \"{}\"", race.trim()))?,
        ),
    };
    let power = match key(power) {
        k if k.is_empty() => None,
        k => Some(
            Power::ALL.into_iter().find(|p| key(p.name()) == k).ok_or(format!("no power called \"{}\"", power.trim()))?,
        ),
    };
    if race.is_none() && power.is_none() {
        return Err("name a race, a power or both, as in Ghouls,Spirit".into());
    }
    Ok((race, power))
}

impl Running for State {
    fn play(&mut self, seat: usize, mv: GameMove) -> Result<(), String> {
        let GameMove::SmallWorld(action) = mv else {
            return Err(WRONG_GAME.into());
        };
        self.apply(seat, action).map_err(|e| e.message().into())
    }

    fn view(&self, seat: usize) -> GameView {
        GameView::SmallWorld(Box::new(self.view_for(seat)))
    }
}

#[cfg(test)]
mod tests {
    use crate::rooms::disconnect;
    use crate::testing::*;
    use protocol::{ClientMsg, GameKind, GameMove, GameOptions, GameView, ServerMsg};
    use smallworld_core::{Action, Options, Phase, RegionMove, Sky, State, View, START_COINS};

    fn play(action: Action) -> ClientMsg {
        ClientMsg::Move(GameMove::SmallWorld(action))
    }

    /// The latest Small World view each client has been sent.
    fn latest_views(clients: &mut [Client], views: &mut [Option<Box<View>>]) {
        for (c, v) in clients.iter_mut().zip(views.iter_mut()) {
            if let Some((GameView::SmallWorld(view), _, _)) = c.latest_game() {
                *v = Some(view);
            }
        }
    }

    /// What a player who only sees their own view would do next: conquer
    /// the first region on offer, put tokens back, end the turn.
    fn next_move(v: &View) -> Action {
        let m = &v.moves;
        let first = |kind: RegionMove| m.list(kind).first().copied();
        let why = m.end_turn.as_deref();
        if let Some(&(offer, _)) = m.pick.first() {
            Action::Pick { offer }
        } else if let Some(region) = first(RegionMove::Conquer) {
            Action::Conquer { region }
        } else if m.end_conquests {
            Action::EndConquests
        } else if why == Some("Place both your Heroes first.") {
            let region = *m.list(RegionMove::Hero).iter().find(|&&r| !v.regions[r].hero).unwrap();
            Action::Hero { region }
        } else if why == Some("Place both your Cannons first.") {
            let region = *m.list(RegionMove::Cannon).iter().find(|&&r| v.regions[r].cannon.is_none()).unwrap();
            Action::Cannon { region }
        } else if why == Some("Keep 4 Amazons in hand: they rest until your next turn.") {
            let hand = v.players[v.you].active.map_or(0, |a| a.hand);
            match (hand < 4, first(RegionMove::Take), first(RegionMove::Put)) {
                (true, Some(region), _) => Action::Take { region },
                (_, _, Some(region)) => Action::Put { region },
                _ => Action::EndTurn,
            }
        } else if let (false, Some(region)) = (m.can_end_turn, first(RegionMove::Put)) {
            Action::Put { region }
        } else {
            Action::EndTurn
        }
    }

    /// The host's message picking the expansions.
    fn expansions(options: Options) -> ClientMsg {
        ClientMsg::SetOptions(GameOptions::SmallWorld(options))
    }

    #[test]
    fn the_combo_to_try_out_is_read_from_its_names() {
        use super::parse_combo;
        use smallworld_core::{Power, Race};
        assert_eq!(parse_combo("Ghouls,Spirit"), Ok((Some(Race::Ghouls), Some(Power::Spirit))));
        assert_eq!(parse_combo(" ghouls , SPIRIT "), Ok((Some(Race::Ghouls), Some(Power::Spirit))));
        assert_eq!(parse_combo("storm giants,hordes of"), Ok((Some(Race::StormGiants), Some(Power::HordesOf))));
        assert_eq!(parse_combo("Trolls,Were"), Ok((Some(Race::Trolls), Some(Power::Were))));
        assert_eq!(parse_combo("Ghouls"), Ok((Some(Race::Ghouls), None)));
        assert_eq!(parse_combo(",Peace-Loving"), Ok((None, Some(Power::PeaceLoving))));
        assert!(parse_combo("Gouls,Spirit").unwrap_err().contains("Gouls"));
        assert!(parse_combo("Ghouls,Spirt").unwrap_err().contains("Spirt"));
        assert!(parse_combo("Spirit").unwrap_err().contains("Spirit"), "a race comes first");
        assert!(parse_combo(",").is_err());
    }

    #[test]
    fn a_small_world_game_starts_with_the_first_seat_picking_a_race() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::SmallWorld, 4);
        assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
        let mut views = vec![None; 4];
        latest_views(&mut clients, &mut views);
        let mut seats: Vec<usize> = Vec::new();
        for v in views.iter().flatten() {
            assert_eq!((v.board, v.regions.len(), v.turns), (4, 40, 9));
            assert_eq!((v.turn, v.current, v.phase), (1, 0, Phase::Pick));
            // Your own coins only.
            for (i, p) in v.players.iter().enumerate() {
                assert_eq!(p.coins, (i == v.you).then_some(START_COINS));
            }
            // Only the first seat has something to do.
            assert_eq!(v.moves.pick.is_empty(), v.you != 0);
            seats.push(v.you);
        }
        seats.sort();
        assert_eq!(seats, [0, 1, 2, 3]);
        with_game(&state, &code, |g: &mut State| assert_eq!(g.seats(), 4));
    }

    #[test]
    fn small_world_moves_are_checked_against_the_seat_whose_turn_it_is() {
        let state = new_state();
        let (_code, mut clients) = lobby(&state, GameKind::SmallWorld, 2);
        clients[0].send(&state, ClientMsg::Start);
        let mut views = vec![None; 2];
        latest_views(&mut clients, &mut views);
        let first = views.iter().position(|v| v.as_ref().unwrap().you == 0).unwrap();
        let second = 1 - first;
        let r = clients[second].send(&state, play(Action::Pick { offer: 0 }));
        assert_eq!(r.as_deref(), Some("It isn't your turn."));
        let r = clients[first].send(&state, play(Action::EndTurn));
        assert_eq!(r.as_deref(), Some("You can't do that right now."));
        assert_eq!(clients[first].send(&state, play(Action::Pick { offer: 0 })), None);
        // Everyone sees the pick.
        latest_views(&mut clients, &mut views);
        for v in views.iter().flatten() {
            assert!(v.players[0].active.is_some());
            assert_eq!(v.phase, Phase::Conquest { ghouls: false });
        }
    }

    #[test]
    fn a_dropped_small_world_player_comes_back_to_their_seat() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::SmallWorld, 3);
        clients[0].send(&state, ClientMsg::Start);
        let mut views = vec![None; 3];
        latest_views(&mut clients, &mut views);
        let before = views[2].clone().unwrap();

        let mut dropped = clients.remove(2);
        let token = dropped.token();
        disconnect(&state, &dropped.tx, dropped.ident.take());
        let mut back = Client::new();
        let r = back.send(&state, ClientMsg::Join { room: code, name: "P2".into(), token: Some(token) });
        assert_eq!(r, None);
        let Some((GameView::SmallWorld(view), _, connected)) = back.latest_game() else {
            panic!("expected the game state on reconnecting");
        };
        assert_eq!(view.you, before.you);
        assert_eq!(connected, [true; 3]);
    }

    #[test]
    fn the_sky_islands_need_three_players_and_seat_six() {
        let state = new_state();
        let (code, mut clients) = lobby(&state, GameKind::SmallWorld, 2);
        assert_eq!(clients[1].send(&state, expansions(Options::all())).as_deref(), Some("Only the host can change the options."));
        assert_eq!(clients[0].send(&state, expansions(Options::all())), None);
        for c in &mut clients {
            let on = Some(GameOptions::SmallWorld(Options::all()));
            assert!(matches!(c.drain().last(), Some(ServerMsg::Lobby { options, .. }) if *options == on));
        }
        assert_eq!(clients[0].send(&state, ClientMsg::Start).as_deref(), Some("You need at least three players."));
        for i in 2..6 {
            clients.push(join(&state, &code, &format!("P{i}")).unwrap());
        }
        assert_eq!(join(&state, &code, "P6").err().as_deref(), Some("That room is full."));
        // Six seated: the Sky Islands can't go now.
        let off = Options { sky: None, ..Options::all() };
        assert_eq!(clients[0].send(&state, expansions(off)).as_deref(), Some("There are too many players here for that."));
        assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
        with_game(&state, &code, |g: &mut State| {
            assert_eq!((g.seats(), g.board.ground, g.board.sky), (6, 5, Some(Sky::Three)));
            assert_eq!(g.options, Options::all());
        });
    }

    #[test]
    fn whole_small_world_games_can_be_played_to_the_end_through_the_server() {
        let all = Options::all();
        let packs = Options { sky: None, ..all };
        for (n, options) in [(2, Options::default()), (3, packs), (4, Options::default()), (5, packs), (3, all), (6, all)] {
            let state = new_state();
            let (code, mut clients) = lobby(&state, GameKind::SmallWorld, n.min(2));
            assert_eq!(clients[0].send(&state, expansions(options)), None);
            for i in 2..n {
                clients.push(join(&state, &code, &format!("P{i}")).unwrap());
            }
            assert_eq!(clients[0].send(&state, ClientMsg::Start), None);
            let mut views: Vec<Option<Box<View>>> = vec![None; n];
            latest_views(&mut clients, &mut views);
            let mut moves = 0;
            while views[0].as_ref().unwrap().phase != Phase::Over {
                let actor = views[0].as_ref().unwrap().actor.expect("somebody acts");
                let i = views.iter().position(|v| v.as_ref().unwrap().you == actor).unwrap();
                let action = next_move(views[i].as_ref().unwrap());
                let r = clients[i].send(&state, play(action));
                assert_eq!(r, None, "{n} players: seat {actor} {action:?}");
                latest_views(&mut clients, &mut views);
                moves += 1;
                assert!(moves < 40_000, "the game got stuck");
            }
            for v in views.iter().flatten() {
                // The coins are out on the table at the end.
                assert!(v.players.iter().all(|p| p.coins.is_some()));
                assert!(!v.winners.is_empty());
            }
            let r = clients[0].send(&state, play(Action::EndTurn));
            assert_eq!(r.as_deref(), Some("The game is over."));
            with_game(&state, &code, |g: &mut State| assert_eq!(g.phase, Phase::Over));
        }
    }
}
