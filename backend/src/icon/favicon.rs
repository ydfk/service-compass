use anyhow::{Context, Result};
use reqwest::{Client, Url};
use scraper::{Html, Selector};

pub async fn discover(client: &Client, input: &str) -> Result<Vec<String>> {
    let page_url = Url::parse(input).context("服务 URL 无效")?;
    let html = client.get(page_url.clone()).send().await?.text().await?;
    let mut candidates = icon_links(&page_url, &html);
    candidates.push(page_url.join("/favicon.ico")?.to_string());
    candidates.sort();
    candidates.dedup();

    let mut available = Vec::new();
    for url in candidates {
        if client
            .get(&url)
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
    use reqwest::Url;

    use super::icon_links;

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
}
