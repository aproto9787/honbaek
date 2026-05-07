use crate::config::Config;
use crate::domain::Baek;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const PROVIDER_NAME: &str = "openai-compatible";
const DIRECT_API_KEY_ENV: &str = "HONBAEK_OPENAI_API_KEY";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct ProviderPlan {
    pub summary: String,
    pub tokens_in: u64,
    pub tokens_out: u64,
}

pub trait ProviderAdapter: Send + Sync {
    fn baek(&self) -> Baek;
    fn plan(&self, task: &str) -> Result<Option<ProviderPlan>>;
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider {
    base_url: String,
    model: String,
    api_key_env: String,
}

impl OpenAiCompatibleProvider {
    pub fn from_config(config: &Config) -> Self {
        Self {
            base_url: config.openai_compatible.base_url.clone(),
            model: config.openai_compatible.model.clone(),
            api_key_env: config.openai_compatible.api_key_env.clone(),
        }
    }

    fn api_key(&self) -> Option<String> {
        self.secret_env_names()
            .into_iter()
            .find_map(read_secret_env)
    }

    fn secret_source(&self) -> Option<String> {
        self.secret_env_names()
            .into_iter()
            .find(|name| read_secret_env(name).is_some())
            .map(ToString::to_string)
    }

    fn secret_env_names(&self) -> Vec<&str> {
        if self.api_key_env == DIRECT_API_KEY_ENV {
            vec![DIRECT_API_KEY_ENV]
        } else {
            vec![DIRECT_API_KEY_ENV, self.api_key_env.as_str()]
        }
    }
}

impl ProviderAdapter for OpenAiCompatibleProvider {
    fn baek(&self) -> Baek {
        Baek {
            provider: PROVIDER_NAME.to_string(),
            model: self.model.clone(),
            base_url: self.base_url.clone(),
            api_key_env: self.api_key_env.clone(),
            secret_source: self.secret_source(),
            configured: self.secret_source().is_some(),
        }
    }

    fn plan(&self, task: &str) -> Result<Option<ProviderPlan>> {
        let Some(api_key) = self.api_key() else {
            return Ok(None);
        };

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "You are 魄, the provider substrate for 혼백강령. Return a concise local execution plan.".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: task.to_string(),
                },
            ],
            temperature: 0.2,
        };

        let endpoint = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let response: ChatResponse = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()?
            .post(endpoint)
            .bearer_auth(api_key)
            .json(&request)
            .send()?
            .error_for_status()?
            .json()?;

        let summary = response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .unwrap_or_else(|| "provider returned no plan".to_string());
        let usage = response.usage.unwrap_or_default();
        Ok(Some(ProviderPlan {
            summary,
            tokens_in: usage.prompt_tokens,
            tokens_out: usage.completion_tokens,
        }))
    }
}

fn read_secret_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Default, Deserialize)]
struct ChatUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baek_exposes_boundary_without_secret_value() {
        let provider = OpenAiCompatibleProvider::from_config(&Config::default());
        let baek = provider.baek();

        assert_eq!(baek.provider, PROVIDER_NAME);
        assert_eq!(baek.base_url, "https://api.openai.com/v1");
        assert_eq!(baek.api_key_env, "OPENAI_API_KEY");
        assert!(baek.secret_source.is_none());
    }
}
