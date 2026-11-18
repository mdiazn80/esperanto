//! Remote profiles and their storage (not their tokens, see
//! [`super::keychain`]).

use std::net::IpAddr;

use reqwest::Url;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};

use crate::error::{CommandResult, ErrorCode};
use crate::store;

const PROVIDERS_KEY: &str = "providers";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteProfile {
    pub id: String,
    pub name: String,
    /// Base URL including the API prefix, e.g. `https://gateway.example.com/v1`.
    pub base_url: String,
    /// Model name sent in requests.
    pub model: String,
}

impl RemoteProfile {
    pub fn host(&self) -> Option<String> {
        Url::parse(&self.base_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProvidersConfig {
    /// Active remote profile; `None` = the local model.
    pub active: Option<String>,
    pub remote: Vec<RemoteProfile>,
}

impl ProvidersConfig {
    pub fn find(&self, id: &str) -> Option<&RemoteProfile> {
        self.remote.iter().find(|p| p.id == id)
    }
}

/// Loads the profiles. Unlike UI settings, an unreadable value is an error,
/// not a reason to fall back to defaults: saving those defaults would lose
/// every profile and leave their tokens orphaned in the Keychain.
pub fn load<R: Runtime>(app: &AppHandle<R>) -> CommandResult<ProvidersConfig> {
    store::get_parsed(app, PROVIDERS_KEY)
        .inspect_err(|e| log::error!("remote profiles can't be read: {e}"))
        .map(Option::unwrap_or_default)
}

pub fn save<R: Runtime>(app: &AppHandle<R>, config: &ProvidersConfig) -> CommandResult<()> {
    store::put(app, PROVIDERS_KEY, config)
}

/// Validates and normalizes a base URL. HTTPS is required except for local
/// and private addresses, where plain HTTP is allowed (returns `true`).
pub fn validate_base_url(input: &str) -> Result<(String, bool), ErrorCode> {
    let url = Url::parse(input.trim()).map_err(|_| ErrorCode::ProviderInvalid)?;
    let host = url.host_str().ok_or(ErrorCode::ProviderInvalid)?.to_owned();
    if !url.username().is_empty() || url.password().is_some() || url.query().is_some() {
        return Err(ErrorCode::ProviderInvalid);
    }
    let plain_http = match url.scheme() {
        "https" => false,
        "http" if is_local_or_private(&host) => true,
        "http" => return Err(ErrorCode::ProviderInsecureUrl),
        _ => return Err(ErrorCode::ProviderInvalid),
    };
    Ok((url.as_str().trim_end_matches('/').to_owned(), plain_http))
}

fn is_local_or_private(host: &str) -> bool {
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".local") {
        return true;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => ip.is_loopback() || ip.is_private() || ip.is_link_local(),
        Ok(IpAddr::V6(ip)) => ip.is_loopback() || ip.is_unique_local(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_urls_are_accepted_and_normalized() {
        assert_eq!(
            validate_base_url(" https://gateway.example.com/v1/ "),
            Ok(("https://gateway.example.com/v1".to_owned(), false))
        );
    }

    #[test]
    fn plain_http_only_for_local_and_private_hosts() {
        for url in [
            "http://localhost:8080/v1",
            "http://127.0.0.1:8080/v1",
            "http://192.168.1.20:8000/v1",
            "http://10.0.0.5/v1",
            "http://gpu-box.local:8080/v1",
            "http://[::1]:8080/v1",
        ] {
            assert!(
                matches!(validate_base_url(url), Ok((_, true))),
                "{url} should be allowed"
            );
        }
        assert_eq!(
            validate_base_url("http://gateway.example.com/v1"),
            Err(ErrorCode::ProviderInsecureUrl)
        );
        assert_eq!(
            validate_base_url("http://8.8.8.8/v1"),
            Err(ErrorCode::ProviderInsecureUrl)
        );
    }

    #[test]
    fn rejects_malformed_urls_and_embedded_credentials() {
        for url in [
            "gateway.example.com",
            "ftp://example.com",
            "https://user:pass@example.com/v1",
            "https://example.com/v1?key=secret",
            "",
        ] {
            assert!(validate_base_url(url).is_err(), "{url} should be rejected");
        }
    }

    #[test]
    fn host_is_taken_from_the_url() {
        let profile = RemoteProfile {
            id: "a".into(),
            name: "n".into(),
            base_url: "https://ai.example.com:8443/v1".into(),
            model: "m".into(),
        };
        assert_eq!(profile.host().as_deref(), Some("ai.example.com"));
    }
}
