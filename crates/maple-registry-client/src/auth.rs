//! Registry authentication support.
//!
//! Supports multiple credential sources:
//! - Environment variables (`MAPLE_REGISTRY_USER`/`MAPLE_REGISTRY_PASSWORD`)
//! - MAPLE credential file (`~/.maple/credentials.json`)
//! - Docker config fallback (`~/.docker/config.json`)

use std::collections::HashMap;
use std::path::PathBuf;

use base64::Engine;
use serde::{Deserialize, Serialize};

/// Authentication method for a registry.
#[derive(Debug, Clone)]
pub enum RegistryAuth {
    /// No authentication required.
    Anonymous,
    /// HTTP Basic authentication.
    Basic {
        username: String,
        password: String,
    },
    /// Bearer token authentication.
    Bearer {
        token: String,
    },
    /// OAuth2 client-credentials flow.
    OAuth2 {
        token_endpoint: String,
        client_id: String,
        client_secret: String,
        scope: String,
    },
}

/// Errors that can occur during authentication operations.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("credentials file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("failed to read credentials: {0}")]
    ReadError(String),

    #[error("failed to parse credentials: {0}")]
    ParseError(String),

    #[error("no credentials found for registry: {0}")]
    NotFound(String),

    #[error("failed to decode docker auth: {0}")]
    DockerAuthDecode(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// A serializable credential entry for `~/.maple/credentials.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialEntry {
    pub registry: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// Docker config.json structure for credential fallback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    #[serde(default)]
    pub auths: HashMap<String, DockerAuthEntry>,
}

/// A single auth entry in Docker's config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerAuthEntry {
    /// Base64-encoded "username:password"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// Decode a Docker base64-encoded `auth` field into `(username, password)`.
pub fn decode_docker_auth(encoded: &str) -> Result<(String, String), AuthError> {
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| AuthError::DockerAuthDecode(format!("base64 decode failed: {}", e)))?;

    let decoded = String::from_utf8(decoded_bytes)
        .map_err(|e| AuthError::DockerAuthDecode(format!("UTF-8 decode failed: {}", e)))?;

    let parts: Vec<&str> = decoded.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(AuthError::DockerAuthDecode(
            "expected 'username:password' format".to_string(),
        ));
    }

    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Credential store that loads and caches registry credentials.
///
/// Searches in order:
/// 1. Environment variables `MAPLE_REGISTRY_USER` / `MAPLE_REGISTRY_PASSWORD`
/// 2. `~/.maple/credentials.json`
/// 3. `~/.docker/config.json` (fallback)
pub struct CredentialStore {
    entries: Vec<CredentialEntry>,
    docker_config: Option<DockerConfig>,
    env_username: Option<String>,
    env_password: Option<String>,
}

impl CredentialStore {
    /// Load credentials from all available sources.
    pub fn load() -> Result<Self, AuthError> {
        let env_username = std::env::var("MAPLE_REGISTRY_USER").ok();
        let env_password = std::env::var("MAPLE_REGISTRY_PASSWORD").ok();

        let entries = Self::load_maple_credentials().unwrap_or_default();
        let docker_config = Self::load_docker_config().ok();

        Ok(Self {
            entries,
            docker_config,
            env_username,
            env_password,
        })
    }

    /// Get authentication for a specific registry.
    pub fn get_auth(&self, registry: &str) -> RegistryAuth {
        // 1. Check environment variables (global override)
        if let (Some(user), Some(pass)) = (&self.env_username, &self.env_password) {
            return RegistryAuth::Basic {
                username: user.clone(),
                password: pass.clone(),
            };
        }

        // 2. Check MAPLE credentials file
        for entry in &self.entries {
            if entry.registry == registry {
                if let Some(ref token) = entry.token {
                    return RegistryAuth::Bearer {
                        token: token.clone(),
                    };
                }
                if let (Some(ref user), Some(ref pass)) = (&entry.username, &entry.password) {
                    return RegistryAuth::Basic {
                        username: user.clone(),
                        password: pass.clone(),
                    };
                }
            }
        }

        // 3. Check Docker config fallback
        if let Some(ref docker_config) = self.docker_config {
            if let Some(auth_entry) = docker_config.auths.get(registry) {
                // Try base64-encoded auth field first
                if let Some(ref auth_b64) = auth_entry.auth {
                    if let Ok((user, pass)) = decode_docker_auth(auth_b64) {
                        return RegistryAuth::Basic {
                            username: user,
                            password: pass,
                        };
                    }
                }
                // Try explicit username/password
                if let (Some(ref user), Some(ref pass)) =
                    (&auth_entry.username, &auth_entry.password)
                {
                    return RegistryAuth::Basic {
                        username: user.clone(),
                        password: pass.clone(),
                    };
                }
            }
        }

        RegistryAuth::Anonymous
    }

    /// Store a credential entry, appending to the in-memory list.
    pub fn store(&mut self, registry: &str, auth: RegistryAuth) {
        // Remove any existing entry for this registry
        self.entries.retain(|e| e.registry != registry);

        let entry = match auth {
            RegistryAuth::Basic { username, password } => CredentialEntry {
                registry: registry.to_string(),
                username: Some(username),
                password: Some(password),
                token: None,
            },
            RegistryAuth::Bearer { token } => CredentialEntry {
                registry: registry.to_string(),
                username: None,
                password: None,
                token: Some(token),
            },
            RegistryAuth::Anonymous => return,
            RegistryAuth::OAuth2 { .. } => {
                // OAuth2 credentials are not persisted — they use runtime token exchange
                return;
            }
        };

        self.entries.push(entry);
    }

    fn load_maple_credentials() -> Result<Vec<CredentialEntry>, AuthError> {
        let home = dirs_path()?;
        let creds_path = home.join(".maple").join("credentials.json");
        if !creds_path.exists() {
            return Ok(Vec::new());
        }
        let data = std::fs::read_to_string(&creds_path)?;
        let entries: Vec<CredentialEntry> =
            serde_json::from_str(&data).map_err(|e| AuthError::ParseError(e.to_string()))?;
        Ok(entries)
    }

    fn load_docker_config() -> Result<DockerConfig, AuthError> {
        let home = dirs_path()?;
        let docker_path = home.join(".docker").join("config.json");
        if !docker_path.exists() {
            return Err(AuthError::FileNotFound(docker_path));
        }
        let data = std::fs::read_to_string(&docker_path)?;
        let config: DockerConfig =
            serde_json::from_str(&data).map_err(|e| AuthError::ParseError(e.to_string()))?;
        Ok(config)
    }
}

/// Apply authentication to request headers.
pub fn apply_auth(
    headers: &mut reqwest::header::HeaderMap,
    auth: &RegistryAuth,
) {
    match auth {
        RegistryAuth::Anonymous => {}
        RegistryAuth::Basic { username, password } => {
            let credentials = format!("{}:{}", username, password);
            let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Basic {}", encoded)
                    .parse()
                    .expect("valid header value"),
            );
        }
        RegistryAuth::Bearer { token } => {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token)
                    .parse()
                    .expect("valid header value"),
            );
        }
        RegistryAuth::OAuth2 { .. } => {
            // OAuth2 token exchange should be performed before calling apply_auth.
            // The resolved bearer token should be passed as RegistryAuth::Bearer.
            tracing::warn!("OAuth2 auth requires token exchange before apply_auth");
        }
    }
}

/// Get the user's home directory path.
fn dirs_path() -> Result<PathBuf, AuthError> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .map_err(|_| {
            AuthError::ReadError("could not determine home directory".to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_entry_serde_roundtrip() {
        let entry = CredentialEntry {
            registry: "registry.maple.ai".to_string(),
            username: Some("alice".to_string()),
            password: Some("secret123".to_string()),
            token: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: CredentialEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.registry, "registry.maple.ai");
        assert_eq!(parsed.username.as_deref(), Some("alice"));
        assert_eq!(parsed.password.as_deref(), Some("secret123"));
        assert!(parsed.token.is_none());
    }

    #[test]
    fn test_credential_entry_serde_with_token() {
        let entry = CredentialEntry {
            registry: "ghcr.io".to_string(),
            username: None,
            password: None,
            token: Some("ghp_abc123".to_string()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: CredentialEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.registry, "ghcr.io");
        assert!(parsed.username.is_none());
        assert!(parsed.password.is_none());
        assert_eq!(parsed.token.as_deref(), Some("ghp_abc123"));
    }

    #[test]
    fn test_credential_entry_serde_skip_none_fields() {
        let entry = CredentialEntry {
            registry: "example.com".to_string(),
            username: None,
            password: None,
            token: Some("tok".to_string()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        // username and password should not appear in JSON
        assert!(!json.contains("username"));
        assert!(!json.contains("password"));
        assert!(json.contains("token"));
    }

    #[test]
    fn test_decode_docker_auth_valid() {
        // base64("testuser:testpass") = "dGVzdHVzZXI6dGVzdHBhc3M="
        let encoded = base64::engine::general_purpose::STANDARD
            .encode("testuser:testpass");
        let (user, pass) = decode_docker_auth(&encoded).unwrap();
        assert_eq!(user, "testuser");
        assert_eq!(pass, "testpass");
    }

    #[test]
    fn test_decode_docker_auth_with_colon_in_password() {
        let encoded = base64::engine::general_purpose::STANDARD
            .encode("user:pass:with:colons");
        let (user, pass) = decode_docker_auth(&encoded).unwrap();
        assert_eq!(user, "user");
        assert_eq!(pass, "pass:with:colons");
    }

    #[test]
    fn test_decode_docker_auth_invalid_no_colon() {
        let encoded = base64::engine::general_purpose::STANDARD.encode("nocolon");
        let result = decode_docker_auth(&encoded);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_docker_auth_invalid_base64() {
        let result = decode_docker_auth("!!!not-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_docker_config_serde() {
        let json = r#"{
            "auths": {
                "registry.maple.ai": {
                    "auth": "dGVzdHVzZXI6dGVzdHBhc3M="
                },
                "ghcr.io": {
                    "username": "bot",
                    "password": "token123"
                }
            }
        }"#;
        let config: DockerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.auths.len(), 2);
        assert!(config.auths.contains_key("registry.maple.ai"));
        assert!(config.auths.contains_key("ghcr.io"));

        let maple_auth = &config.auths["registry.maple.ai"];
        assert_eq!(
            maple_auth.auth.as_deref(),
            Some("dGVzdHVzZXI6dGVzdHBhc3M=")
        );

        let ghcr_auth = &config.auths["ghcr.io"];
        assert_eq!(ghcr_auth.username.as_deref(), Some("bot"));
        assert_eq!(ghcr_auth.password.as_deref(), Some("token123"));
    }

    #[test]
    fn test_apply_auth_basic() {
        let mut headers = reqwest::header::HeaderMap::new();
        let auth = RegistryAuth::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        };
        apply_auth(&mut headers, &auth);
        let auth_header = headers
            .get(reqwest::header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(auth_header.starts_with("Basic "));
        // Decode and verify
        let encoded = auth_header.strip_prefix("Basic ").unwrap();
        let decoded_bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        let decoded = String::from_utf8(decoded_bytes).unwrap();
        assert_eq!(decoded, "user:pass");
    }

    #[test]
    fn test_apply_auth_bearer() {
        let mut headers = reqwest::header::HeaderMap::new();
        let auth = RegistryAuth::Bearer {
            token: "my-token-123".to_string(),
        };
        apply_auth(&mut headers, &auth);
        let auth_header = headers
            .get(reqwest::header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(auth_header, "Bearer my-token-123");
    }

    #[test]
    fn test_apply_auth_anonymous() {
        let mut headers = reqwest::header::HeaderMap::new();
        apply_auth(&mut headers, &RegistryAuth::Anonymous);
        assert!(headers.get(reqwest::header::AUTHORIZATION).is_none());
    }
}
