//! The dice on the island: the last roll as two die faces in the sea's
//! bottom-right corner, with its total and whose roll it is, and the turn's
//! Roll dice / End turn button for the player whose turn it is. The faces
//! tumble in when a new roll arrives (see `look.rs`).

use super::look::{now_ms, FreshKind};
use super::map::PLAYER_COLORS;
use super::Mode;
use crate::app::{send, App};
use catan_core::{Action, GameView, Phase};
use leptos::prelude::*;
use protocol::{ClientMsg, GameMove};

/// How long the dice tumble after a roll, the second one a little later.
pub const ROLL_MS: f64 = 700.0;
const SECOND_DIE_MS: f64 = 90.0;

/// Where the pips of a face sit, on a 10×10 die.
fn pips(n: u8) -> &'static [(f64, f64)] {
    const L: f64 = 2.7;
    const M: f64 = 5.0;
    const H: f64 = 7.3;
    match n {
        1 => &[(M, M)],
        2 => &[(H, L), (L, H)],
        3 => &[(H, L), (M, M), (L, H)],
        4 => &[(L, L), (H, L), (L, H), (H, H)],
        5 => &[(L, L), (H, L), (M, M), (L, H), (H, H)],
        _ => &[(L, L), (H, L), (L, M), (H, M), (L, H), (H, H)],
    }
}

/// One die: its face, or a blank one before the roll. `age` is how long
/// ago it was rolled, while it is still tumbling.
fn die(face: Option<u8>, age: Option<f64>) -> impl IntoView {
    let dots = face
        .map(|n| {
            pips(n)
                .iter()
                .map(|&(x, y)| view! { <circle class="pip" cx=x cy=y r="0.95" /> })
                .collect_view()
        });
    let label = face.map_or("not rolled yet".to_string(), |n| format!("a die showing {n}"));
    view! {
        <svg class=if age.is_some() { "die rolling" } else { "die" } class:blank=face.is_none()
             viewBox="0 0 10 10" role="img" aria-label=label
             style=age.map(|a| format!("animation-delay:{:.0}ms", -a))>
            <rect x="0.4" y="0.4" width="9.2" height="9.2" rx="1.9" />
            {dots}
        </svg>
    }
}

/// The tray in the sea's corner; nothing while the first pieces go down
/// or once the game is over.
pub fn dice_tray(app: App, v: &GameView) -> AnyView {
    if matches!(v.phase, Phase::Setup { .. } | Phase::Finished { .. }) {
        return ().into_any();
    }
    let cur = v.current;
    let mine = v.seat == Some(cur);
    let who = if mine { "You".to_string() } else { v.players[cur].name.clone() };
    let caption = match (v.dice, mine) {
        (Some(_), _) => format!("{who} rolled"),
        (None, true) => "Your roll".to_string(),
        (None, false) => format!("{who} to roll"),
    };
    let button = match (&v.phase, mine) {
        (Phase::Roll, true) => Some(("Roll dice", Action::Roll)),
        (Phase::Main, true) => Some(("End turn", Action::EndTurn)),
        _ => None,
    };

    // Tumbling while the roll is fresh; a redraw later on leaves them still.
    let age = v.dice.and_then(|_| {
        let now = now_ms();
        app.games.catan.fresh.with_untracked(|list| {
            list.iter()
                .filter(|f| f.kind == FreshKind::Roll)
                .map(|f| (now - f.at).max(0.0))
                .find(|&age| age < ROLL_MS + SECOND_DIE_MS)
        })
    });
    let (a, b) = v.dice.unzip();
    let total = v.dice.map(|(a, b)| {
        let n = a + b;
        view! { <span class=if n == 7 { "dice-total seven" } else { "dice-total" }>{n}</span> }
    });
    let title = match v.dice {
        Some((a, b)) => format!("{caption} {a} + {b} = {}", a + b),
        None => caption.clone(),
    };

    let footer = match button {
        Some((label, action)) => view! {
            <button class="primary" on:click=move |_| {
                app.games.catan.ui.mode.set(Mode::None);
                send(app, &ClientMsg::Move(GameMove::Catan(action.clone())));
            }>{label}</button>
        }
        .into_any(),
        None => view! { <div class="dice-caption">{caption}</div> }.into_any(),
    };

    view! {
        <div class="dice-tray" title=title style=format!("--who:{}", PLAYER_COLORS[cur % 4])>
            <div class="dice-row">
                {die(a, age)}
                {die(b, age.map(|a| a - SECOND_DIE_MS))}
                {total}
            </div>
            {footer}
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_face_has_its_number_of_pips_inside_the_die() {
        for n in 1..=6 {
            let p = pips(n);
            assert_eq!(p.len(), n as usize);
            assert!(p.iter().all(|&(x, y)| (1.5..=8.5).contains(&x) && (1.5..=8.5).contains(&y)));
        }
    }
}
