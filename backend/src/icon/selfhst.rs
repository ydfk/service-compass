use serde::Serialize;

#[derive(Serialize)]
pub struct IconSuggestion {
    pub reference: String,
    pub urls: Vec<String>,
}

pub fn suggest(name: &str) -> IconSuggestion {
    let reference = slug(name);
    IconSuggestion {
        urls: urls(&reference),
        reference,
    }
}

pub fn urls(reference: &str) -> Vec<String> {
    ["svg", "png", "webp"]
        .into_iter()
        .map(|extension| {
            format!("https://cdn.jsdelivr.net/gh/selfhst/icons/{extension}/{reference}.{extension}")
        })
        .collect()
}

pub fn url(reference: &str) -> String {
    urls(reference).into_iter().next().unwrap_or_default()
}

pub fn slug(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::slug;

    #[test]
    fn generates_expected_references() {
        assert_eq!(slug("Plex"), "plex");
        assert_eq!(slug("Home Assistant"), "home-assistant");
        assert_eq!(slug("qBittorrent"), "qbittorrent");
    }
}
