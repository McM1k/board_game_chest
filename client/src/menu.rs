//! The first screen: pick which game to play. Every game of the registry
//! (see `games`) gets a card here.

use crate::app::{App, Screen};
use crate::games;
use leptos::prelude::*;
use protocol::GameKind;

pub fn menu_screen(app: App) -> impl IntoView {
    let cards = GameKind::ALL.iter().map(|&game| game_card(app, game)).collect_view();
    view! {
        <div class="menu-page">
            <div class="menu">
                <h1>"Pick a game"</h1>
                <p class="muted">"Play with friends in your browser: no accounts, just a room code."</p>
                <div class="game-grid">{cards}</div>
            </div>
        </div>
    }
}

/// One card of the picker; it leads to that game's create/join screen.
fn game_card(app: App, game: GameKind) -> impl IntoView {
    let def = games::game(game);
    let players = format!("{}\u{2013}{} players", game.min_players(), game.max_players());
    view! {
        <button class="game-card" on:click=move |_| app.screen.set(Screen::Home(game))>
            <div class="game-icon">{def.icon()}</div>
            <h2>{game.title()}</h2>
            <div class="game-players">{players}</div>
            <p class="game-blurb">{def.blurb()}</p>
            <div class="game-cta">"Play"</div>
        </button>
    }
}
