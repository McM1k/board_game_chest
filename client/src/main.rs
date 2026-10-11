//! The browser client: a shell (the menu, the connection and the routing
//! between screens) and, under `games`, one folder per game.

// The views nest deeply; the compiler needs room to work out their types.
#![recursion_limit = "256"]

mod app;
mod games;
mod menu;
#[cfg(test)]
mod testing;

use app::{connect, App, Screen};
use leptos::prelude::*;

/// Whatever the current screen says should be shown, in its game's frame.
fn current_screen(app: App) -> AnyView {
    match app.screen.get() {
        Screen::Menu => menu::menu_screen(app).into_any(),
        Screen::Home(kind) => {
            let game = games::game(kind);
            game.frame(app, game.home(app))
        }
        Screen::Lobby(lobby) => {
            let game = games::game(lobby.game);
            game.frame(app, game.lobby(app, &lobby))
        }
        Screen::Playing(kind) => {
            let game = games::game(kind);
            game.frame(app, game.board(app))
        }
    }
}

/// The title of the browser tab: the game on screen, or just "Games".
fn page_title(app: App) -> &'static str {
    let game = app.screen.with(|screen| match screen {
        Screen::Menu => None,
        Screen::Home(kind) | Screen::Playing(kind) => Some(*kind),
        Screen::Lobby(lobby) => Some(lobby.game),
    });
    game.map_or("Games", |kind| games::game(kind).tab_title(app))
}

#[component]
fn Root() -> impl IntoView {
    let app = App::new();
    connect(app);

    Effect::new(move |_| document().set_title(page_title(app)));

    view! {
        <div class="root">
            {move || {
                let err = app.error.get();
                err.map(|e| view! { <div class="toast">{e}</div> })
            }}
            {move || (!app.online.get()).then(|| view! { <div class="offline-banner">"Connecting to server…"</div> })}
            {move || current_screen(app)}
        </div>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(Root);
}

/// Native render tests of the shell: the menu, the routing to each game's
/// screens and frame, and the page title. Each game's own screens are
/// tested in its folder. These do not replace a browser test.
#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::app::{handle, leave, Lobby};
    use crate::testing::{players, shows};
    use hanabi_core::{GameRules, GameState, PlayerId};
    use leptos::tachys::view::RenderHtml;
    use protocol::{GameKind, GameOptions, GameView, ServerMsg};
    use rand::{rngs::StdRng, SeedableRng};

    /// The class of the element each game's stylesheet is scoped to.
    fn frame_class(kind: GameKind) -> &'static str {
        match kind {
            GameKind::Catan => "class=\"theme-catan\"",
            GameKind::Hanabi => "class=\"hanabi\"",
            GameKind::Wonderful => "class=\"wonderful\"",
            GameKind::SmallWorld => "class=\"smallworld\"",
        }
    }

    /// A first view of each game, from seat 1 of a three-player game.
    fn first_view(kind: GameKind) -> GameView {
        let names: Vec<String> = ["Ann", "Bob", "Cy"].iter().map(|s| s.to_string()).collect();
        match kind {
            GameKind::Catan => {
                let mut rng = StdRng::seed_from_u64(3);
                let game = catan_core::Game::new(catan_core::Rules::default(), names, &mut rng);
                GameView::Catan(Box::new(game.view_for(Some(1))))
            }
            GameKind::Hanabi => GameView::Hanabi(Box::new(GameState::new(3, 5, GameRules::default()).view_for(PlayerId(1)))),
            GameKind::Wonderful => GameView::Wonderful(Box::new(wonderful_core::State::new(3, 7).view_for(1))),
            GameKind::SmallWorld => GameView::SmallWorld(Box::new(smallworld_core::State::new(3, 7).view_for(1))),
        }
    }

    #[test]
    fn menu_lists_every_game_and_leads_to_their_rooms() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            assert!(matches!(app.screen.get_untracked(), Screen::Menu));
            let html = current_screen(app).to_html();
            assert!(html.contains("Pick a game") && html.contains("menu-page"));
            assert!(html.contains("Catan") && html.contains("Hanabi"));
            assert!(shows(&html, "It's a Wonderful World") && html.contains("Small World"));
            // Catan seats 2-4, Hanabi 2-5, Small World 2-6 (with the Sky
            // Islands), It's a Wonderful World 2-7 (with its expansion).
            assert_eq!(html.matches("2\u{2013}4 players").count(), 1);
            assert_eq!(html.matches("2\u{2013}5 players").count(), 1);
            assert_eq!(html.matches("2\u{2013}6 players").count(), 1);
            assert_eq!(html.matches("2\u{2013}7 players").count(), 1);
            // One card per game, each with its picture and its sentence.
            assert_eq!(html.matches("class=\"game-card\"").count(), GameKind::ALL.len());
            for kind in GameKind::ALL {
                assert!(shows(&html, games::game(kind).blurb()), "{kind:?}");
            }
            // Every game is playable here: no greyed-out card, no outside link.
            assert!(!html.contains("disabled") && !html.contains("href="));
            assert_eq!(page_title(app), "Games");
        });
    }

    #[test]
    fn every_game_is_registered_under_its_own_kind_and_draws_in_its_own_frame() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            for kind in GameKind::ALL {
                let game = games::game(kind);
                assert_eq!(game.kind(), kind);

                app.screen.set(Screen::Home(kind));
                let html = current_screen(app).to_html();
                assert!(html.contains(frame_class(kind)), "{kind:?}: {html}");
                assert!(html.contains("Create a room") && html.contains("Join room"), "{kind:?}");
                assert!(html.contains("All games"), "{kind:?} leads back to the menu");
                for other in GameKind::ALL.into_iter().filter(|&k| k != kind) {
                    assert!(!html.contains(frame_class(other)), "{kind:?} drawn in {other:?}'s frame");
                }
                assert_eq!(page_title(app), kind.title());

                let lobby = Lobby { room: "WXYZ".into(), game: kind, players: players(&["Ann", "Bob"]), you: 0, is_host: true };
                app.screen.set(Screen::Lobby(lobby));
                let html = current_screen(app).to_html();
                assert!(html.contains(frame_class(kind)) && html.contains("WXYZ"), "{kind:?}");
                assert!(html.contains("Start game") && html.contains("Leave room"), "{kind:?}");
            }
        });
    }

    #[test]
    fn the_room_code_decides_which_lobby_you_land_in() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let lobby = |game| {
                Screen::Lobby(Lobby { room: "WXYZ".into(), game, players: players(&["Ann", "Bob"]), you: 1, is_host: false })
            };

            // Picked Catan in the menu, but the code belongs to a Hanabi room.
            app.screen.set(Screen::Home(GameKind::Catan));
            app.screen.set(lobby(GameKind::Hanabi));
            let html = current_screen(app).to_html();
            assert!(html.contains("WXYZ") && html.contains("Hanabii mode"));
            assert!(html.contains("Waiting for the host to pick the rules"));
            assert!(html.contains("Bob") && html.contains("(you)"));
            assert!(!html.contains("Start game"));

            app.screen.set(lobby(GameKind::Catan));
            let html = current_screen(app).to_html();
            assert!(html.contains("WXYZ") && html.contains("Waiting for the host to start"));
            assert!(!html.contains("Hanabii mode"));

            // ...or to It's a Wonderful World, which has no rules to pick.
            app.screen.set(lobby(GameKind::Wonderful));
            let html = current_screen(app).to_html();
            assert!(html.contains("WXYZ") && html.contains("Waiting for the host to start"));
            assert!(html.contains("class=\"wonderful\"") && html.contains("(you)"));
            assert!(!html.contains("Hanabii mode") && !html.contains("theme-catan"));
            assert!(!html.contains("Start game"));
            assert_eq!(page_title(app), "It's a Wonderful World");
        });
    }

    #[test]
    fn a_state_from_the_server_puts_its_game_on_screen_until_you_leave() {
        let owner = Owner::new();
        owner.with(|| {
            for kind in GameKind::ALL {
                let app = App::new();
                handle(app, ServerMsg::Joined { room: "WXYZ".into(), token: "t".into() });
                let names = vec!["Ann".to_string(), "Bob".to_string(), "Cy".to_string()];
                handle(app, ServerMsg::Game { view: first_view(kind), names, connected: vec![true; 3] });
                assert!(matches!(app.screen.get_untracked(), Screen::Playing(k) if k == kind), "{kind:?}");
                let html = current_screen(app).to_html();
                assert!(html.contains(frame_class(kind)) && html.contains("WXYZ"), "{kind:?}: {html}");
                assert!(html.contains("Leave"), "{kind:?}");
                assert_eq!(page_title(app), kind.title());

                // Leaving forgets every game and goes back to the menu.
                leave(app);
                assert!(matches!(app.screen.get_untracked(), Screen::Menu));
                let games = app.games;
                assert!(games.catan.view.get_untracked().is_none());
                assert!(games.hanabi.view.get_untracked().is_none());
                assert!(games.wonderful.view.get_untracked().is_none());
                assert!(games.smallworld.view.get_untracked().is_none());
                assert!(app.room.get_untracked().is_empty());
            }
        });
    }

    #[test]
    fn lobby_options_reach_their_game() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let rules = GameRules { hanabii: true, ..Default::default() }.normalized();
            let lobby = |options| ServerMsg::Lobby {
                room: "WXYZ".into(),
                game: GameKind::Hanabi,
                players: players(&["Ann"]),
                you: 0,
                is_host: true,
                options,
            };
            handle(app, lobby(Some(GameOptions::Hanabi(rules))));
            assert_eq!(app.games.hanabi.rules.get_untracked(), rules);
            assert!(matches!(app.screen.get_untracked(), Screen::Lobby(ref l) if l.game == GameKind::Hanabi));
            assert_eq!(app.room.get_untracked(), "WXYZ");
            // The next change from the host replaces them.
            handle(app, lobby(Some(GameOptions::Hanabi(GameRules::default()))));
            assert_eq!(app.games.hanabi.rules.get_untracked(), GameRules::default());
        });
    }

    #[test]
    fn hanabii_mode_locks_the_other_rules_and_retitles_the_page() {
        let owner = Owner::new();
        owner.with(|| {
            let app = App::new();
            let hanabi = games::game(GameKind::Hanabi);
            let lobby = Lobby { room: "WXYZ".into(), game: GameKind::Hanabi, players: players(&["A", "B"]), you: 0, is_host: true };
            let ordinary = hanabi.frame(app, hanabi.lobby(app, &lobby)).to_html();
            assert!(ordinary.contains("<h1>Hanabi</h1>") && !ordinary.contains("mode-popover"));
            assert!(!ordinary.contains("rule-group-locked"));
            app.screen.set(Screen::Playing(GameKind::Hanabi));
            assert_eq!(page_title(app), "Hanabi");

            app.games.hanabi.rules.set(GameRules { hanabii: true, ..Default::default() }.normalized());
            let html = hanabi.frame(app, hanabi.lobby(app, &lobby)).to_html();
            assert!(html.contains("<h1>Hanabii</h1>") && html.contains("mode-popover"));
            assert!(html.contains("rule-group-locked"));
            assert_eq!(page_title(app), "Hanabii");

            // Once the game runs, what it is played with decides.
            let state = GameState::new(2, 3, GameRules::default());
            app.games.hanabi.view.set(Some(state.view_for(PlayerId(0))));
            assert_eq!(page_title(app), "Hanabi");
        });
    }
}
