use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::PathBuf;

const HONBAEK_HOME_ENV: &str = "HONBAEK_HOME";
const SUPPORTED_PROVIDER: &str = "openai-compatible";

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub home: PathBuf,
    pub db: PathBuf,
    pub journal: PathBuf,
    pub config: PathBuf,
    pub socket: PathBuf,
    pub pid: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
        let base = BaseDirs::new().context("could not resolve home directory")?;
        let home = match std::env::var_os(HONBAEK_HOME_ENV).filter(|value| !value.is_empty()) {
            Some(path) => PathBuf::from(path),
            None => base.home_dir().join(".honbaek"),
        };
        Ok(Self {
            db: home.join("state.sqlite3"),
            journal: home.join("journal.jsonl"),
            config: home.join("config.toml"),
            socket: home.join("honbaek.sock"),
            pid: home.join("honbaek.pid"),
            home,
        })
    }

    pub fn ensure(&self) -> Result<()> {
        fs::create_dir_all(&self.home)
            .with_context(|| format!("failed to create {}", self.home.display()))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub default_provider: String,
    pub openai_compatible: OpenAiCompatibleConfig,
    pub daemon: DaemonConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenAiCompatibleConfig {
    pub base_url: String,
    pub model: String,
    pub api_key_env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonConfig {
    pub socket_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_provider: "openai-compatible".to_string(),
            openai_compatible: OpenAiCompatibleConfig {
                base_url: "https://api.openai.com/v1".to_string(),
                model: "gpt-5.5".to_string(),
                api_key_env: "OPENAI_API_KEY".to_string(),
            },
            daemon: DaemonConfig { socket_path: None },
        }
    }
}

impl Config {
    pub fn load(paths: &AppPaths) -> Result<Self> {
        paths.ensure()?;
        if !paths.config.exists() {
            write_default_config(paths)?;
        }

        let raw = fs::read_to_string(&paths.config)
            .with_context(|| format!("failed to read {}", paths.config.display()))?;
        reject_secret_keys(&raw, paths)?;
        let mut config: Self = toml::from_str(&raw)
            .with_context(|| format!("failed to parse {}", paths.config.display()))?;

        if let Some(default_provider) = env_override("HONBAEK_PROVIDER") {
            config.default_provider = default_provider;
        }
        if let Some(base_url) = env_override("HONBAEK_OPENAI_BASE_URL") {
            config.openai_compatible.base_url = base_url;
        }
        if let Some(model) = env_override("HONBAEK_OPENAI_MODEL") {
            config.openai_compatible.model = model;
        }
        if let Some(api_key_env) = env_override("HONBAEK_OPENAI_API_KEY_ENV") {
            config.openai_compatible.api_key_env = api_key_env;
        }
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.default_provider != SUPPORTED_PROVIDER {
            bail!(
                "unsupported provider {}; only {} is implemented",
                self.default_provider,
                SUPPORTED_PROVIDER
            );
        }
        if self.openai_compatible.base_url.trim().is_empty() {
            bail!("openai_compatible.base_url must not be empty");
        }
        if self.openai_compatible.model.trim().is_empty() {
            bail!("openai_compatible.model must not be empty");
        }
        let api_key_env = self.openai_compatible.api_key_env.trim();
        if api_key_env.is_empty() || api_key_env.contains('=') || api_key_env.contains('\0') {
            bail!("openai_compatible.api_key_env must be a valid environment variable name");
        }
        Ok(())
    }
}

fn write_default_config(paths: &AppPaths) -> Result<()> {
    let rendered = toml::to_string_pretty(&Config::default())?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(&paths.config) {
        Ok(mut file) => {
            file.write_all(rendered.as_bytes())
                .with_context(|| format!("failed to write {}", paths.config.display()))?;
            file.sync_all()
                .with_context(|| format!("failed to sync {}", paths.config.display()))?;
            Ok(())
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("failed to create {}", paths.config.display()))
        }
    }
}

fn env_override(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn reject_secret_keys(raw: &str, paths: &AppPaths) -> Result<()> {
    let value: toml::Value = toml::from_str(raw)
        .with_context(|| format!("failed to parse {}", paths.config.display()))?;
    scan_secret_keys(&value, &[])
}

fn scan_secret_keys(value: &toml::Value, path: &[&str]) -> Result<()> {
    match value {
        toml::Value::Table(table) => {
            for (key, child) in table {
                let normalized = key.to_ascii_lowercase().replace('-', "_");
                if is_forbidden_secret_key(&normalized) {
                    let mut full_path = path.join(".");
                    if !full_path.is_empty() {
                        full_path.push('.');
                    }
                    full_path.push_str(key);
                    bail!(
                        "config.toml must not contain secret key {}; set provider secrets through environment variables",
                        full_path
                    );
                }
                let mut child_path = path.to_vec();
                child_path.push(key);
                scan_secret_keys(child, &child_path)?;
            }
            Ok(())
        }
        toml::Value::Array(values) => {
            for child in values {
                scan_secret_keys(child, path)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn is_forbidden_secret_key(key: &str) -> bool {
    matches!(
        key,
        "api_key"
            | "secret"
            | "token"
            | "access_token"
            | "bearer_token"
            | "authorization"
            | "password"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_paths() -> AppPaths {
        let home = std::env::temp_dir().join(format!("honbaek-config-test-{}", Uuid::new_v4()));
        AppPaths {
            db: home.join("state.sqlite3"),
            journal: home.join("journal.jsonl"),
            config: home.join("config.toml"),
            socket: home.join("honbaek.sock"),
            pid: home.join("honbaek.pid"),
            home,
        }
    }

    #[test]
    fn default_config_writes_only_secret_env_name() {
        let paths = test_paths();
        let config = Config::load(&paths).expect("load default config");
        let raw = fs::read_to_string(&paths.config).expect("read default config");

        assert_eq!(config.openai_compatible.api_key_env, "OPENAI_API_KEY");
        assert!(raw.contains("api_key_env"));
        assert!(!raw.contains("api_key ="));
        fs::remove_dir_all(paths.home).ok();
    }

    #[test]
    fn config_rejects_raw_provider_secret_keys() {
        let paths = test_paths();
        paths.ensure().expect("create home");
        fs::write(
            &paths.config,
            r#"
default_provider = "openai-compatible"

[openai_compatible]
base_url = "https://api.openai.com/v1"
model = "gpt-5.5"
api_key_env = "OPENAI_API_KEY"
api_key = "secret"

[daemon]
socket_path = ""
"#,
        )
        .expect("write config");

        let error = Config::load(&paths).expect_err("raw api_key must fail");
        assert!(error.to_string().contains("must not contain secret key"));
        fs::remove_dir_all(paths.home).ok();
    }
}
