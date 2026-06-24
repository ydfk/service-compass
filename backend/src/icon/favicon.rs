use anyhow::{Context, Result};
use reqwest::{Client, Url};
use scraper::{Html, Selector};

pub async fn discover(client: &Client, input: &str) -> Result<String> {
    let page_url = Url::parse(input).context("服务 URL 无效")?;
    let origin = page_url.join("/favicon.ico")?;
    if client
        .get(origin.clone())
        .send()
        .await
        .is_ok_and(|response| response.status().is_success())
    {
        return Ok(origin.to_string());
    }

    let html = client.get(page_url.clone()).send().await?.text().await?;
    let document = Html::parse_document(&html);
    let selector = Selector::parse("link[rel]").map_err(|_| anyhow::anyhow!("图标选择器无效"))?;
    for element in document.select(&selector) {
        let rel = element
            .value()
            .attr("rel")
            .unwrap_or_default()
            .to_lowercase();
        if !rel.contains("icon") {
            continue;
        }
        if let Some(href) = element.value().attr("href") {
            return Ok(page_url.join(href)?.to_string());
        }
    }
    Err(anyhow::anyhow!("未发现 favicon"))
}
