use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    str::FromStr,
};

use axum::extract::Request;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

pub const DEFAULT_ANONYMOUS_CIDRS: &str = "\
127.0.0.1/8
::1/128
10.0.0.0/8
172.16.0.0/12
192.168.0.0/16
fc00::/7
fe80::/10";

#[derive(Clone, Debug)]
struct IpNetwork {
    address: IpAddr,
    prefix: u8,
}

pub async fn is_anonymous_allowed_for_ip(state: &AppState, ip: Option<IpAddr>) -> AppResult<bool> {
    let Some(ip) = ip else {
        return Ok(true);
    };
    let cidrs = anonymous_cidrs(state).await?;
    ip_in_list(ip, &cidrs)
}

pub fn validate_cidrs(value: &str) -> AppResult<()> {
    parse_networks(value).map(|_| ())
}

pub fn client_ip(request: &Request) -> Option<IpAddr> {
    forwarded_ip(request)
        .or_else(|| header_ip(request, "x-forwarded-for"))
        .or_else(|| header_ip(request, "x-real-ip"))
        .or_else(|| {
            request
                .extensions()
                .get::<axum::extract::ConnectInfo<SocketAddr>>()
                .map(|item| item.0.ip())
        })
}

fn forwarded_ip(request: &Request) -> Option<IpAddr> {
    let value = request.headers().get("forwarded")?.to_str().ok()?;
    value
        .split(',')
        .next()?
        .split(';')
        .find_map(|part| part.trim().strip_prefix("for="))
        .and_then(parse_header_ip)
}

fn header_ip(request: &Request, name: &str) -> Option<IpAddr> {
    let value = request.headers().get(name)?.to_str().ok()?;
    value.split(',').next().and_then(parse_header_ip)
}

fn parse_header_ip(value: &str) -> Option<IpAddr> {
    let value = value.trim().trim_matches('"').trim_matches(['[', ']']);
    value
        .parse()
        .ok()
        .or_else(|| value.split(':').next()?.parse().ok())
}

async fn anonymous_cidrs(state: &AppState) -> AppResult<String> {
    let value: Option<String> =
        sqlx::query_scalar("SELECT value FROM settings WHERE key = 'anonymous_access_cidrs'")
            .fetch_optional(&state.pool)
            .await?;
    Ok(value.unwrap_or_else(|| DEFAULT_ANONYMOUS_CIDRS.to_owned()))
}

fn ip_in_list(ip: IpAddr, cidrs: &str) -> AppResult<bool> {
    Ok(parse_networks(cidrs)?
        .into_iter()
        .any(|network| network.contains(ip)))
}

fn parse_networks(value: &str) -> AppResult<Vec<IpNetwork>> {
    value
        .split(|character: char| character == ',' || character == '\n' || character.is_whitespace())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(parse_network)
        .collect()
}

fn parse_network(value: &str) -> AppResult<IpNetwork> {
    let (address, prefix) = match value.split_once('/') {
        Some((address, prefix)) => {
            let address = IpAddr::from_str(address.trim())
                .map_err(|_| AppError::Validation(format!("网段格式无效：{value}")))?;
            let prefix = prefix
                .trim()
                .parse::<u8>()
                .map_err(|_| AppError::Validation(format!("网段格式无效：{value}")))?;
            (address, prefix)
        }
        None => {
            let address = IpAddr::from_str(value)
                .map_err(|_| AppError::Validation(format!("网段格式无效：{value}")))?;
            let prefix = match address {
                IpAddr::V4(_) => 32,
                IpAddr::V6(_) => 128,
            };
            (address, prefix)
        }
    };
    validate_prefix(address, prefix, value)?;
    Ok(IpNetwork { address, prefix })
}

fn validate_prefix(address: IpAddr, prefix: u8, raw: &str) -> AppResult<()> {
    let max = match address {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    };
    if prefix > max {
        return Err(AppError::Validation(format!("网段前缀无效：{raw}")));
    }
    Ok(())
}

impl IpNetwork {
    fn contains(&self, ip: IpAddr) -> bool {
        match (self.address, ip) {
            (IpAddr::V4(network), IpAddr::V4(ip)) => contains_v4(network, ip, self.prefix),
            (IpAddr::V6(network), IpAddr::V6(ip)) => contains_v6(network, ip, self.prefix),
            _ => false,
        }
    }
}

fn contains_v4(network: Ipv4Addr, ip: Ipv4Addr, prefix: u8) -> bool {
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    u32::from(network) & mask == u32::from(ip) & mask
}

fn contains_v6(network: Ipv6Addr, ip: Ipv6Addr, prefix: u8) -> bool {
    let network = u128::from(network);
    let ip = u128::from(ip);
    let mask = if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix)
    };
    network & mask == ip & mask
}

#[cfg(test)]
mod tests {
    use super::{ip_in_list, validate_cidrs};

    #[test]
    fn cidr_matching_supports_multiple_ranges() {
        assert!(
            ip_in_list(
                "192.168.1.42".parse().unwrap(),
                "10.0.0.0/8\n192.168.1.0/24"
            )
            .unwrap()
        );
        assert!(!ip_in_list("8.8.8.8".parse().unwrap(), "10.0.0.0/8\n192.168.1.0/24").unwrap());
    }

    #[test]
    fn invalid_cidr_is_rejected() {
        assert!(validate_cidrs("192.168.1.0/33").is_err());
    }
}
