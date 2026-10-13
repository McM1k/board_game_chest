//! The Rules button: beside the report button, in the waiting room and the
//! game of every game. It opens a short summary of the game's rules (each
//! game writes its own, see [`crate::games::Game::rules`]) with links to the
//! official rulebooks.

use crate::app::{App, Screen};
use crate::games;
use leptos::prelude::*;
use protocol::GameKind;

/// A game's rules, short enough to read on a phone.
pub struct Rules {
    /// What the game is about, in a sentence or two.
    pub intro: &'static str,
    /// Setup, a turn or a round, scoring, the end, and anything else.
    pub sections: &'static [Section],
    /// The official rulebooks: what each one is, and where.
    pub links: &'static [(&'static str, &'static str)],
}

pub struct Section {
    pub title: &'static str,
    pub points: &'static [&'static str],
}

/// The game whose rules the screen can show: in its waiting room or its game.
fn game_on_screen(app: App) -> Option<GameKind> {
    app.screen.with(|screen| match screen {
        Screen::Lobby(lobby) => Some(lobby.game),
        Screen::Playing(kind) => Some(*kind),
        Screen::Menu | Screen::Home(_) => None,
    })
}

/// Opens or closes the rules. They share the corner with the report's
/// dialog, so one closes the other.
pub fn toggle(app: App) {
    app.rules_open.update(|open| *open = !*open);
    if app.rules_open.get_untracked() {
        app.report.open.set(false);
    }
}

/// The button, and the rules when they are open.
pub fn widget(app: App) -> impl IntoView {
    move || {
        game_on_screen(app).map(|kind| {
            view! {
                <div class="rules">
                    {move || app.rules_open.get().then(|| panel(app, kind))}
                    <button
                        class="rules-button"
                        type="button"
                        title="The rules of the game"
                        aria-haspopup="dialog"
                        aria-expanded=move || app.rules_open.get().to_string()
                        on:click=move |_| toggle(app)
                    >
                        "Rules"
                    </button>
                </div>
            }
        })
    }
}

fn panel(app: App, kind: GameKind) -> impl IntoView {
    let rules = games::game(kind).rules();
    let close = move || app.rules_open.set(false);
    let on_key = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            close();
        }
    };
    let sections = rules
        .sections
        .iter()
        .map(|s| {
            let points = s.points.iter().map(|p| view! { <li>{*p}</li> }).collect_view();
            view! {
                <section>
                    <h3>{s.title}</h3>
                    <ul>{points}</ul>
                </section>
            }
        })
        .collect_view();
    let links = rules
        .links
        .iter()
        .map(|(label, url)| view! { <li><a href=*url target="_blank" rel="noopener">{*label}</a></li> })
        .collect_view();
    view! {
        <div class="rules-panel" role="dialog" aria-labelledby="rules-title" on:keydown=on_key>
            <div class="rules-head">
                <h2 id="rules-title">{kind.title()} " rules"</h2>
                <button type="button" class="rules-close" aria-label="Close" title="Close" on:click=move |_| close()>
                    "×"
                </button>
            </div>
            <div class="rules-body">
                <p class="rules-intro">{rules.intro}</p>
                {sections}
                <h3>"Official rulebook"</h3>
                <ul class="rules-links">{links}</ul>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use crate::games;
    use protocol::GameKind;

    /// Every game covers the same ground, links its rulebook, and stays
    /// short: a few minutes' read on a phone.
    #[test]
    fn every_game_has_a_short_summary_and_its_rulebook() {
        for kind in GameKind::ALL {
            let rules = games::game(kind).rules();
            let titles: Vec<&str> = rules.sections.iter().map(|s| s.title).collect();
            assert_eq!(titles[0], "Setup", "{kind:?}");
            assert!(titles.iter().any(|t| ["Your turn", "A round"].contains(t)), "{kind:?}: {titles:?}");
            let scoring = titles.iter().position(|&t| t == "Scoring");
            assert!(scoring.is_some() && titles.get(scoring.unwrap() + 1) == Some(&"End of game"), "{kind:?}: {titles:?}");
            assert!(rules.sections.iter().all(|s| !s.points.is_empty()), "{kind:?}");
            assert!(!rules.links.is_empty(), "{kind:?}");
            for (label, url) in rules.links {
                assert!(url.starts_with("https://") && url.ends_with(".pdf"), "{kind:?}: {url}");
                assert!(!label.is_empty());
            }
            let words: usize = rules.intro.split_whitespace().count()
                + rules.sections.iter().flat_map(|s| s.points).map(|p| p.split_whitespace().count()).sum::<usize>();
            assert!(words <= 520, "{kind:?}: {words} words");
        }
    }
}
