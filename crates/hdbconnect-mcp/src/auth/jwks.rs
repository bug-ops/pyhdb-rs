//! JWKS fetching and caching

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::Deserialize;
use url::Url;

use super::error::{AuthError, Result};

/// JSON Web Key
#[derive(Debug, Clone, Deserialize)]
pub struct Jwk {
    /// Key ID
    #[serde(default)]
    pub kid: Option<String>,
    /// Key type (RSA, EC)
    pub kty: String,
    /// Algorithm
    #[serde(default)]
    pub alg: Option<String>,
    /// Key usage
    #[serde(default)]
    pub r#use: Option<String>,
    // RSA components
    #[serde(default)]
    pub n: Option<String>,
    #[serde(default)]
    pub e: Option<String>,
    // EC components
    #[serde(default)]
    pub crv: Option<String>,
    #[serde(default)]
    pub x: Option<String>,
    #[serde(default)]
    pub y: Option<String>,
}

/// JSON Web Key Set
#[derive(Debug, Clone, Deserialize)]
pub struct JwkSet {
    pub keys: Vec<Jwk>,
}

/// JWKS entry with metadata
#[derive(Clone)]
struct JwkEntry {
    key: jsonwebtoken::DecodingKey,
    algorithm: jsonwebtoken::Algorithm,
}

impl std::fmt::Debug for JwkEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwkEntry")
            .field("algorithm", &self.algorithm)
            .finish_non_exhaustive()
    }
}

/// Thread-safe JWKS cache
pub struct JwksCache {
    keys: RwLock<HashMap<String, JwkEntry>>,
    unnamed_keys: RwLock<Vec<JwkEntry>>,
    jwks_uri: Url,
    client: reqwest::Client,
    ttl: Duration,
    last_refresh: RwLock<Option<Instant>>,
}

impl std::fmt::Debug for JwksCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwksCache")
            .field("jwks_uri", &self.jwks_uri)
            .field("ttl", &self.ttl)
            .field("keys_count", &self.keys.read().len())
            .finish_non_exhaustive()
    }
}

impl JwksCache {
    pub fn new(jwks_uri: Url, ttl: Duration) -> Self {
        Self {
            keys: RwLock::new(HashMap::new()),
            unnamed_keys: RwLock::new(Vec::new()),
            jwks_uri,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("valid client"),
            ttl,
            last_refresh: RwLock::new(None),
        }
    }

    /// Get key for validation
    pub async fn get_key(
        &self,
        kid: Option<&str>,
        alg: jsonwebtoken::Algorithm,
    ) -> Result<jsonwebtoken::DecodingKey> {
        if self.needs_refresh() {
            self.refresh().await?;
        }

        if let Some(kid) = kid {
            let result = {
                let keys = self.keys.read();
                keys.get(kid)
                    .filter(|entry| entry.algorithm == alg)
                    .map(|entry| entry.key.clone())
            };
            if let Some(key) = result {
                return Ok(key);
            }
            return Err(AuthError::KeyNotFound(kid.to_string()));
        }

        // Try unnamed keys first
        {
            let unnamed = self.unnamed_keys.read();
            for entry in unnamed.iter() {
                if entry.algorithm == alg {
                    return Ok(entry.key.clone());
                }
            }
        }

        // Try all named keys by algorithm
        {
            let keys = self.keys.read();
            for entry in keys.values() {
                if entry.algorithm == alg {
                    return Ok(entry.key.clone());
                }
            }
        }

        Err(AuthError::NoMatchingKey)
    }

    /// Refresh JWKS from remote
    pub async fn refresh(&self) -> Result<()> {
        tracing::debug!(jwks_uri = %self.jwks_uri, "Refreshing JWKS");

        let response = self
            .client
            .get(self.jwks_uri.clone())
            .send()
            .await
            .map_err(AuthError::JwksFetch)?;

        let jwks: JwkSet = response
            .json()
            .await
            .map_err(|e| AuthError::JwksParse(e.to_string()))?;

        let mut keys = HashMap::new();
        let mut unnamed = Vec::new();

        for jwk in jwks.keys {
            if let Some((key, alg)) = decode_jwk(&jwk)? {
                let entry = JwkEntry {
                    key,
                    algorithm: alg,
                };

                if let Some(kid) = &jwk.kid {
                    keys.insert(kid.clone(), entry);
                } else {
                    unnamed.push(entry);
                }
            }
        }

        *self.keys.write() = keys;
        *self.unnamed_keys.write() = unnamed;
        *self.last_refresh.write() = Some(Instant::now());

        tracing::info!(
            keys_count = self.keys.read().len(),
            "JWKS refreshed successfully"
        );

        Ok(())
    }

    fn needs_refresh(&self) -> bool {
        self.last_refresh
            .read()
            .is_none_or(|t| t.elapsed() > self.ttl)
    }

    #[cfg(test)]
    pub fn keys_count(&self) -> usize {
        self.keys.read().len() + self.unnamed_keys.read().len()
    }
}

fn decode_jwk(jwk: &Jwk) -> Result<Option<(jsonwebtoken::DecodingKey, jsonwebtoken::Algorithm)>> {
    let alg = match jwk.alg.as_deref() {
        Some("RS256") => jsonwebtoken::Algorithm::RS256,
        Some("RS384") => jsonwebtoken::Algorithm::RS384,
        Some("RS512") => jsonwebtoken::Algorithm::RS512,
        Some("ES256") => jsonwebtoken::Algorithm::ES256,
        Some("ES384") => jsonwebtoken::Algorithm::ES384,
        None => {
            // Infer from key type
            match jwk.kty.as_str() {
                "RSA" => jsonwebtoken::Algorithm::RS256,
                "EC" => match jwk.crv.as_deref() {
                    Some("P-256") => jsonwebtoken::Algorithm::ES256,
                    Some("P-384") => jsonwebtoken::Algorithm::ES384,
                    _ => return Ok(None),
                },
                _ => return Ok(None),
            }
        }
        _ => return Ok(None),
    };

    let key = match jwk.kty.as_str() {
        "RSA" => {
            let n = jwk
                .n
                .as_ref()
                .ok_or_else(|| AuthError::JwksParse("Missing 'n' in RSA key".into()))?;
            let e = jwk
                .e
                .as_ref()
                .ok_or_else(|| AuthError::JwksParse("Missing 'e' in RSA key".into()))?;
            jsonwebtoken::DecodingKey::from_rsa_components(n, e)
                .map_err(|e| AuthError::JwksParse(format!("Invalid RSA components: {e}")))?
        }
        "EC" => {
            let x = jwk
                .x
                .as_ref()
                .ok_or_else(|| AuthError::JwksParse("Missing 'x' in EC key".into()))?;
            let y = jwk
                .y
                .as_ref()
                .ok_or_else(|| AuthError::JwksParse("Missing 'y' in EC key".into()))?;
            jsonwebtoken::DecodingKey::from_ec_components(x, y)
                .map_err(|e| AuthError::JwksParse(format!("Invalid EC components: {e}")))?
        }
        other => {
            tracing::debug!(kty = other, "Skipping unsupported key type");
            return Ok(None);
        }
    };

    Ok(Some((key, alg)))
}

/// Background JWKS refresh task builder
pub struct JwksRefreshTask {
    cache: Arc<JwksCache>,
    interval: Duration,
}

impl std::fmt::Debug for JwksRefreshTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwksRefreshTask")
            .field("cache", &self.cache)
            .field("interval", &self.interval)
            .finish()
    }
}

impl JwksRefreshTask {
    #[must_use]
    pub const fn new(cache: Arc<JwksCache>, interval: Duration) -> Self {
        Self { cache, interval }
    }

    pub fn spawn(
        self,
        shutdown: tokio_util::sync::CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(self.interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Err(e) = self.cache.refresh().await {
                            tracing::warn!(error = %e, "Background JWKS refresh failed");
                        }
                    }
                    () = shutdown.cancelled() => {
                        tracing::debug!("JWKS refresh task shutting down");
                        break;
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwk_set_deserialize() {
        let json = r#"{
            "keys": [
                {
                    "kid": "key1",
                    "kty": "RSA",
                    "alg": "RS256",
                    "n": "test_n",
                    "e": "AQAB"
                }
            ]
        }"#;
        let jwks: JwkSet = serde_json::from_str(json).unwrap();
        assert_eq!(jwks.keys.len(), 1);
        assert_eq!(jwks.keys[0].kid, Some("key1".to_string()));
        assert_eq!(jwks.keys[0].kty, "RSA");
    }

    #[test]
    fn test_jwk_ec_deserialize() {
        let json = r#"{
            "keys": [
                {
                    "kid": "ec-key",
                    "kty": "EC",
                    "alg": "ES256",
                    "crv": "P-256",
                    "x": "test_x",
                    "y": "test_y"
                }
            ]
        }"#;
        let jwks: JwkSet = serde_json::from_str(json).unwrap();
        assert_eq!(jwks.keys.len(), 1);
        assert_eq!(jwks.keys[0].crv, Some("P-256".to_string()));
    }

    #[test]
    fn test_jwks_cache_needs_refresh_initially() {
        let cache = JwksCache::new(
            Url::parse("https://example.com/.well-known/jwks.json").unwrap(),
            Duration::from_secs(3600),
        );
        assert!(cache.needs_refresh());
    }
}
