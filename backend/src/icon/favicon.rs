use anyhow::{Context, Result};
use reqwest::{Client, RequestBuilder, Url};
use scraper::{Html, Selector};

#[derive(Clone, Debug)]
pub struct FaviconAuth {
    pub username: String,
    pub password: String,
}

pub async fn discover(
    client: &Client,
    input: &str,
    auth: Option<&FaviconAuth>,
) -> Result<Vec<String>> {
    let page_url = Url::parse(input).context("服务 URL 无效")?;
    let html = apply_auth(client.get(page_url.clone()), auth)
        .send()
        .await?
        .text()
        .await?;
    let mut candidates = icon_links(&page_url, &html);
    candidates.push(page_url.join("/favicon.ico")?.to_string());
    candidates.sort();
    candidates.dedup();

    let mut available = Vec::new();
    for url in candidates {
        if apply_auth(client.get(&url), auth)
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            available.push(url);
        }
    }
    if available.is_empty() {
        return Err(anyhow::anyhow!("未发现 favicon"));
    }
    Ok(available)
}

fn apply_auth(request: RequestBuilder, auth: Option<&FaviconAuth>) -> RequestBuilder {
    match auth {
        Some(auth) => request.basic_auth(&auth.username, Some(&auth.password)),
        None => request,
    }
}

fn icon_links(page_url: &Url, html: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let Ok(selector) = Selector::parse("link[rel]") else {
        return Vec::new();
    };
    document
        .select(&selector)
        .filter(|element| {
            element
                .value()
                .attr("rel")
                .is_some_and(|rel| rel.to_lowercase().contains("icon"))
        })
        .filter_map(|element| element.value().attr("href"))
        .filter_map(|href| page_url.join(href).ok())
        .map(|url| url.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use reqwest::{Client, Url, header};

    use super::{FaviconAuth, apply_auth, icon_links};

    #[test]
    fn finds_and_resolves_all_icon_links() {
        let page = Url::parse("https://example.com/app/").unwrap();
        let html = r#"
            <link rel="icon" href="/favicon-32.png">
            <link rel="apple-touch-icon" href="icons/apple.png">
            <link rel="stylesheet" href="style.css">
        "#;
        assert_eq!(
            icon_links(&page, html),
            vec![
                "https://example.com/favicon-32.png",
                "https://example.com/app/icons/apple.png"
            ]
        );
    }

    #[test]
    fn basic_auth_header_is_only_added_when_auth_exists() {
        let client = Client::new();
        let plain = apply_auth(client.get("https://example.com/favicon.ico"), None)
            .build()
            .unwrap();
        assert!(!plain.headers().contains_key(header::AUTHORIZATION));

        let auth = FaviconAuth {
            username: "user".into(),
            password: "pass".into(),
        };
        let secured = apply_auth(client.get("https://example.com/favicon.ico"), Some(&auth))
            .build()
            .unwrap();
        assert_eq!(
            secured.headers().get(header::AUTHORIZATION).unwrap(),
            "Basic dXNlcjpwYXNz"
        );
    }
}
