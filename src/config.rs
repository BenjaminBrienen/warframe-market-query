use serde::Deserialize;

/// File holding the warframe.market session token, as `JWT:"<token>"`.
const AUTH_FILE: &str = "auth.txt";

#[derive(Deserialize)]
#[serde(default)]
pub struct Config {
    /// How many minutes an orders cache file stays valid before re-fetching.
    pub orders_cache_minutes: u64,

    /// Whether to use the cached user-order information.
    pub cache_user_orders: bool,

    /// Session JWT read from `auth.txt`, needed for commands that change
    /// your account (`buy-orders`). Never read from `wfmq.json`.
    #[serde(skip)]
    pub auth_token: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            orders_cache_minutes: 5,
            cache_user_orders: false,
            auth_token: None,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut config = Self::load_json();
        config.auth_token = load_auth_token();
        config
    }

    fn load_json() -> Self {
        let Ok(text) = std::fs::read_to_string("wfmq.json") else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_else(|error| {
            eprintln!("⚠  wfmq.json is invalid ({error}), using defaults.");
            Self::default()
        })
    }

    /// The JWT to authenticate with, if `auth.txt` held a usable one.
    pub fn auth_token(&self) -> Option<String> {
        self.auth_token.clone()
    }
}

/// Reads and parses `auth.txt`. A missing file is fine (most commands don't
/// need a token); an unreadable or malformed one is reported.
fn load_auth_token() -> Option<String> {
    let text = std::fs::read_to_string(AUTH_FILE).ok()?;
    let token = parse_auth_text(&text);
    match &token {
        None => eprintln!("⚠  {AUTH_FILE} should contain: JWT:\"<token>\""),
        Some(token) if token.split('.').count() != 3 => {
            eprintln!(
                "⚠  the token in {AUTH_FILE} doesn't look like a JWT (expected 3 dot-separated parts)"
            );
        },
        Some(_) => {},
    }
    token
}

/// Extracts the token from text like `JWT:"eyJ..."`.
/// Also tolerates spaces after the colon, a missing pair of quotes,
/// and a bare token with no `JWT:` prefix.
fn parse_auth_text(text: &str) -> Option<String> {
    let text = text.trim();
    let value = match text.strip_prefix("JWT") {
        Some(rest) => rest.trim_start().strip_prefix(':')?,
        None => text,
    };
    let token = value.trim().trim_matches('"').trim();
    if token.is_empty() || token.contains(char::is_whitespace) {
        return None;
    }
    Some(token.to_owned())
}

#[cfg(test)]
mod tests {
    use super::parse_auth_text;

    #[test]
    fn parses_quoted_value() {
        assert_eq!(
            parse_auth_text("JWT:\"aaa.bbb.ccc\"\n").as_deref(),
            Some("aaa.bbb.ccc")
        );
    }

    #[test]
    fn tolerates_spaces_and_bare_tokens() {
        assert_eq!(
            parse_auth_text("JWT: \"aaa.bbb.ccc\"").as_deref(),
            Some("aaa.bbb.ccc")
        );
        assert_eq!(
            parse_auth_text("aaa.bbb.ccc").as_deref(),
            Some("aaa.bbb.ccc")
        );
    }

    #[test]
    fn rejects_empty_or_malformed() {
        assert_eq!(parse_auth_text(""), None);
        assert_eq!(parse_auth_text("JWT:\"\""), None);
        assert_eq!(parse_auth_text("JWT \"aaa\""), None);
    }
}
