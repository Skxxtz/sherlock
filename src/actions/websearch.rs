use std::collections::HashMap;

use super::commandlaunch::command_launch;
use crate::utils::{
    config::{ConfigGuard, ConstantDefaults},
    errors::SherlockError,
};

pub fn websearch(
    mut engine: &str,
    query: &str,
    browser: Option<&str>,
) -> Result<(), SherlockError> {
    if is_url(query) {
        engine = "plain";
    }
    let engines: HashMap<&str, &str> = HashMap::from([
        ("google", "https://www.google.com/search?q={keyword}"),
        ("bing", "https://www.bing.com/search?q={keyword}"),
        ("duckduckgo", "https://duckduckgo.com/?q={keyword}"),
        ("yahoo", "https://search.yahoo.com/search?p={keyword}"),
        ("baidu", "https://www.baidu.com/s?wd={keyword}"),
        ("yandex", "https://yandex.com/search/?text={keyword}"),
        ("ask", "https://www.ask.com/web?q={keyword}"),
        ("ecosia", "https://www.ecosia.org/search?q={keyword}"),
        ("qwant", "https://www.qwant.com/?q={keyword}"),
        (
            "startpage",
            "https://www.startpage.com/sp/search?q={keyword}",
        ),
        ("plain", "{keyword}"),
    ]);
    let url_template = if let Some(url) = engines.get(engine) {
        url
    } else {
        engine
    };

    let mut browser = match browser {
        Some(b) => b.to_string(),
        None => {
            let c = ConfigGuard::read()?;
            c.default_apps
                .browser
                .clone()
                .unwrap_or(ConstantDefaults::browser()?)
        }
    };

    let url = url_template.replace("{keyword}", &query.replace(" ", "+"));
    let command = if browser.contains("%u") {
        browser.replace("%u", &format!(r#" "{}""#, url))
    } else {
        browser.push_str(&format!(r#" "{}""#, url));
        browser
    };
    command_launch(&command, "", HashMap::new())
}

fn is_url(input: &str) -> bool {
    let s = input.trim();

    if s.is_empty() {
        return false;
    }

    let bytes = s.as_bytes();

    // Check for <scheme>: (http:, https:, ...)
    if let Some(colon_pos) = memchr::memchr(b':', bytes) {
        if colon_pos == 0 {
            return false;
        }

        if colon_pos + 2 >= bytes.len() {
            return false;
        }

        if bytes.get(colon_pos + 1) == Some(&b'/') && bytes.get(colon_pos + 2) == Some(&b'/') {
            return true;
        }

        return false;
    }

    if s.eq_ignore_ascii_case("localhost") {
        return true;
    }

    // IPv4 detection
    // Format: ddd.ddd.ddd.ddd with up to three digits & three dots
    let mut dot_count = 0;
    let mut digit_count = 0;
    for &c in bytes {
        if c == b'.' {
            dot_count += 1;
            if dot_count > 3 || digit_count == 0 {
                break;
            }
            digit_count = 0;
        } else if c.is_ascii_digit() {
            digit_count += 1;
            if digit_count > 3 {
                break;
            }
        } else {
            dot_count = 0;
            break;
        }
    }
    if dot_count == 3 {
        return true;
    }

    // Has a single dot and no spaces → domain.com
    if !s.contains(' ') && !s.ends_with('.') && memchr::memchr(b'.', bytes).is_some() {
        return true;
    }

    // host:port (only digits after colon)
    if let Some((host, port)) = s.split_once(':') {
        if !host.contains(' ') && port.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }

    false
}

#[test]
fn test_url_detector() {
    assert!(is_url("google.com"));
    assert!(!is_url("http:"));
    assert!(is_url("http://x"));
    assert!(is_url("8.8.8.8"));
    assert!(is_url("localhost"));

    assert!(!is_url("hello"));
    assert!(!is_url("rust regex"));
    assert!(!is_url("a b.com"));
}
