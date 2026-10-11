//! Helpers shared by the native render tests.

use protocol::LobbyPlayer;

/// Connected players with these names, as a lobby lists them.
pub fn players(names: &[&str]) -> Vec<LobbyPlayer> {
    names
        .iter()
        .map(|n| LobbyPlayer { name: n.to_string(), connected: true })
        .collect()
}

/// Whether `html` shows `text`, however the renderer escaped its quotes.
pub fn shows(html: &str, text: &str) -> bool {
    html.contains(text) || html.contains(&text.replace('\'', "&#39;"))
}
