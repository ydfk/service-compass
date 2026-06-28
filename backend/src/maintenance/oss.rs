use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::{Client, Method, Url};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct OssTarget {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub prefix: Option<String>,
    pub access_key_id: String,
    pub access_key_secret: String,
}

pub struct OssResponse {
    pub url: String,
    pub status_code: u16,
    pub response_summary: String,
}

pub async fn put_object(
    client: &Client,
    target: &OssTarget,
    filename: &str,
    bytes: Vec<u8>,
) -> AppResult<OssResponse> {
    request(client, target, Method::PUT, filename, bytes).await
}

pub async fn delete_object(
    client: &Client,
    target: &OssTarget,
    filename: &str,
) -> AppResult<OssResponse> {
    request(client, target, Method::DELETE, filename, Vec::new()).await
}

async fn request(
    client: &Client,
    target: &OssTarget,
    method: Method,
    filename: &str,
    bytes: Vec<u8>,
) -> AppResult<OssResponse> {
    let url = object_url(target, filename)?;
    let payload_hash = sha256_hex(&bytes);
    let now = Utc::now();
    let date = now.format("%Y%m%d").to_string();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let host = url
        .host_str()
        .map(|host| match url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_owned(),
        })
        .ok_or_else(|| AppError::Validation("OSS Endpoint 无效".into()))?;
    let canonical_uri = url.path();
    let canonical_headers =
        format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n");
    let signed_headers = "host;x-amz-content-sha256;x-amz-date";
    let canonical_request = format!(
        "{}\n{}\n\n{}\n{}\n{}",
        method.as_str(),
        canonical_uri,
        canonical_headers,
        signed_headers,
        payload_hash
    );
    let scope = format!("{date}/{}/s3/aws4_request", target.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );
    let signature = sign(
        &target.access_key_secret,
        &date,
        &target.region,
        &string_to_sign,
    )?;
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
        target.access_key_id
    );
    let response = client
        .request(method.clone(), url.clone())
        .header("x-amz-date", amz_date)
        .header("x-amz-content-sha256", payload_hash)
        .header("Authorization", authorization)
        .body(bytes)
        .send()
        .await
        .map_err(|error| AppError::External(format!("阿里云 OSS 请求失败：{error}")))?;
    let status_code = response.status().as_u16();
    let response_summary = response
        .text()
        .await
        .unwrap_or_default()
        .chars()
        .take(512)
        .collect::<String>();
    Ok(OssResponse {
        url: url.to_string(),
        status_code,
        response_summary,
    })
}

fn object_url(target: &OssTarget, filename: &str) -> AppResult<Url> {
    let mut url = Url::parse(target.endpoint.trim_end_matches('/'))
        .map_err(|_| AppError::Validation("OSS Endpoint 格式无效".into()))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| AppError::Validation("OSS Endpoint 格式无效".into()))?;
        segments.clear();
        segments.push(target.bucket.trim_matches('/'));
        if let Some(prefix) = target.prefix.as_deref() {
            for part in prefix
                .trim_matches('/')
                .split('/')
                .filter(|part| !part.is_empty())
            {
                segments.push(part);
            }
        }
        segments.push(filename);
    }
    Ok(url)
}

fn sign(secret: &str, date: &str, region: &str, string_to_sign: &str) -> AppResult<String> {
    let date_key = hmac(format!("AWS4{secret}").as_bytes(), date.as_bytes())?;
    let region_key = hmac(&date_key, region.as_bytes())?;
    let service_key = hmac(&region_key, b"s3")?;
    let signing_key = hmac(&service_key, b"aws4_request")?;
    Ok(hex::encode(hmac(&signing_key, string_to_sign.as_bytes())?))
}

fn hmac(key: &[u8], message: &[u8]) -> AppResult<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(anyhow::Error::from)?;
    mac.update(message);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn sha256_hex(bytes: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(bytes.as_ref()))
}
