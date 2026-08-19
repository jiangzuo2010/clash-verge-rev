use anyhow::{Context as _, Result, bail};
use reqwest::{Client, Url, redirect};
use std::{sync::Arc, time::Duration};

pub fn secure_enterprise_client() -> Result<Client> {
    Ok(Client::builder()
        .tls_backend_preconfigured(static_webpki_tls_config()?)
        .no_proxy()
        .redirect(redirect::Policy::none())
        .tcp_keepalive(Duration::from_secs(60))
        .pool_max_idle_per_host(0)
        .pool_idle_timeout(None)
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()?)
}

pub fn validate_enterprise_https_url(url: &str, label: &str) -> Result<()> {
    let parsed = Url::parse(url).with_context(|| format!("invalid {label}"))?;
    if parsed.scheme() == "https" {
        return Ok(());
    }

    if parsed.scheme() == "http" && parsed.host_str().is_some_and(is_loopback_host) {
        return Ok(());
    }

    bail!("{label} must use HTTPS");
}

pub fn is_loopback_enterprise_url(url: &str) -> Result<bool> {
    let parsed = Url::parse(url).context("invalid enterprise url")?;
    Ok(parsed.host_str().is_some_and(is_loopback_host))
}

fn static_webpki_tls_config() -> Result<rustls::ClientConfig> {
    let root_store = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()?
        .with_root_certificates(root_store)
        .with_no_client_auth();

    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(config)
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enterprise_url_accepts_https() {
        assert!(validate_enterprise_https_url("https://api-t.taxspace.cn/gateway/api", "policy url").is_ok());
    }

    #[test]
    fn enterprise_url_accepts_loopback_http_for_local_development() {
        assert!(validate_enterprise_https_url("http://127.0.0.1:18080", "policy url").is_ok());
        assert!(validate_enterprise_https_url("http://localhost:18080", "policy url").is_ok());
    }

    #[test]
    fn enterprise_url_rejects_public_http() {
        assert!(validate_enterprise_https_url("http://api-t.taxspace.cn/gateway/api", "policy url").is_err());
    }

    #[test]
    fn enterprise_url_detects_loopback() {
        assert!(matches!(is_loopback_enterprise_url("http://127.0.0.1:18080"), Ok(true)));
        assert!(matches!(
            is_loopback_enterprise_url("https://api-t.taxspace.cn/gateway/api"),
            Ok(false)
        ));
    }
}
