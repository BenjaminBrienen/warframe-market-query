pub const PLATINUM: &str = "p";

/// warframe.market allows 3 requests per second.
const SECONDS_PER_REQUEST: f32 = 0.334;

pub fn timestamp() -> impl std::fmt::Display {
    chrono::Local::now().format("%F %X")
}

/// Rough time needed to make `requests` rate-limited requests.
pub fn eta_secs(requests: usize) -> f32 {
    requests as f32 * SECONDS_PER_REQUEST
}

/// Logs `[done/total]` every 50 items and on the last one.
/// `done` is the number of items already processed (1-based).
pub fn log_progress(
    done: usize,
    total: usize,
    verb: &str,
) {
    if done.is_multiple_of(50) || done == total {
        eprintln!("[{}]:   [{done}/{total}] {verb}", timestamp());
    }
}

/// `Some(ducats)` only when the item is actually worth ducats.
pub fn positive_ducats(ducats: Option<u32>) -> Option<u32> {
    ducats.filter(|&ducats| ducats > 0)
}

pub fn username_quoted(username: &str) -> String {
    if username.contains(' ') {
        format!("\"{username}\"")
    } else {
        username.to_owned()
    }
}

pub fn ingame_name_to_user_slug_hack(ingame_name: &str) -> String {
    ingame_name
        .chars()
        .map(|character| match character {
            ' ' | '.' => '-',
            character => character.to_ascii_lowercase(),
        })
        .collect()
}
