use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use crate::config;

pub trait LLMClient: Send + Sync {
    fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        json_mode: bool,
    ) -> Result<String, Box<dyn std::error::Error>>;
}

pub struct ExternalCommandLLMClient {
    pub command_str: String,
}

impl ExternalCommandLLMClient {
    pub fn new(command_str: impl Into<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let cmd = command_str.into().trim().to_string();
        if cmd.is_empty() {
            return Err("External LLM command string cannot be empty.".into());
        }
        Ok(Self { command_str: cmd })
    }
}

impl LLMClient for ExternalCommandLLMClient {
    fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        json_mode: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut prompt_parts = Vec::new();
        if !system_prompt.trim().is_empty() {
            prompt_parts.push(format!("System Instructions:\n{}", system_prompt));
        }
        if !user_prompt.trim().is_empty() {
            prompt_parts.push(format!("Task:\n{}", user_prompt));
        }
        if json_mode {
            prompt_parts.push(
                "CRITICAL OUTPUT FORMAT REQUIREMENT:\nYou MUST respond with ONLY a valid, parseable JSON object matching the requested schema. \
                 Do NOT include markdown backticks (like ```json), commentary, or extra prose."
                    .to_string(),
            );
        }
        let full_input = prompt_parts.join("\n\n");

        #[cfg(windows)]
        let mut child = Command::new("cmd")
            .args(["/C", &self.command_str])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to execute external LLM command '{}': {}", self.command_str, e))?;

        #[cfg(not(windows))]
        let mut child = Command::new("sh")
            .args(["-c", &self.command_str])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to execute external LLM command '{}': {}", self.command_str, e))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(full_input.as_bytes())?;
        }

        let output = child.wait_with_output()?;

        if !output.status.success() {
            let stderr_str = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let err_msg = if stderr_str.is_empty() {
                format!("Process exited with code {:?}", output.status.code())
            } else {
                stderr_str
            };
            return Err(format!(
                "External LLM command failed (code {:?}): {}",
                output.status.code(),
                err_msg
            )
            .into());
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if stdout_str.is_empty() {
            return Err(format!(
                "External LLM command '{}' returned empty output.",
                self.command_str
            )
            .into());
        }

        Ok(stdout_str)
    }
}

#[allow(dead_code)]
pub struct GeminiClient {
    pub api_key: String,
    pub model: String,
}

#[allow(dead_code)]
impl GeminiClient {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}

impl LLMClient for GeminiClient {
    fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        json_mode: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if self.api_key.trim().is_empty() {
            return Err(
                "GEMINI_API_KEY is not configured. Set it in ~/.experienced-llms/config.json or as an environment variable.".into()
            );
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let combined_text = format!("System Instructions:\n{}\n\nTask:\n{}", system_prompt, user_prompt);
        let mut payload = serde_json::json!({
            "contents": [
                {
                    "role": "user",
                    "parts": [{"text": combined_text}]
                }
            ],
            "generationConfig": {
                "temperature": 0.1
            }
        });

        if json_mode {
            payload["generationConfig"]["responseMimeType"] = serde_json::json!("application/json");
        }

        let resp: serde_json::Value = ureq::post(&url)
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(60))
            .send_json(payload)
            .map_err(|e| format!("Gemini API error: {}", e))?
            .into_json()?;

        let mut output = String::new();
        if let Some(candidates) = resp.get("candidates").and_then(|c| c.as_array()) {
            if let Some(first_cand) = candidates.first() {
                if let Some(parts) = first_cand.get("content").and_then(|c| c.get("parts")).and_then(|p| p.as_array()) {
                    for part in parts {
                        if let Some(txt) = part.get("text").and_then(|t| t.as_str()) {
                            output.push_str(txt);
                        }
                    }
                }
            }
        }

        Ok(output)
    }
}

#[allow(dead_code)]
pub struct ClaudeClient {
    pub api_key: String,
    pub model: String,
}

#[allow(dead_code)]
impl ClaudeClient {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}

impl LLMClient for ClaudeClient {
    fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        _json_mode: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if self.api_key.trim().is_empty() {
            return Err(
                "ANTHROPIC_API_KEY is not configured. Set it in ~/.experienced-llms/config.json or as an environment variable.".into()
            );
        }

        let url = "https://api.anthropic.com/v1/messages";
        let payload = serde_json::json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": system_prompt,
            "messages": [
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.1
        });

        let resp: serde_json::Value = ureq::post(url)
            .set("Content-Type", "application/json")
            .set("x-api-key", &self.api_key)
            .set("anthropic-version", "2023-06-01")
            .timeout(std::time::Duration::from_secs(60))
            .send_json(payload)
            .map_err(|e| format!("Claude API error: {}", e))?
            .into_json()?;

        let mut output = String::new();
        if let Some(content_blocks) = resp.get("content").and_then(|c| c.as_array()) {
            for b in content_blocks {
                if b.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(txt) = b.get("text").and_then(|t| t.as_str()) {
                        output.push_str(txt);
                    }
                }
            }
        }

        Ok(output)
    }
}

#[allow(dead_code)]
pub struct OpenAIClient {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

#[allow(dead_code)]
impl OpenAIClient {
    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            base_url: base_url.into(),
        }
    }
}

impl LLMClient for OpenAIClient {
    fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        json_mode: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if self.api_key.trim().is_empty() && !self.base_url.contains("localhost") {
            return Err(
                "OPENAI_API_KEY is not configured. Set it in ~/.experienced-llms/config.json or as an environment variable.".into()
            );
        }

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut payload = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.1
        });

        if json_mode {
            payload["response_format"] = serde_json::json!({"type": "json_object"});
        }

        let mut req = ureq::post(&url)
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(60));

        if !self.api_key.trim().is_empty() {
            req = req.set("Authorization", &format!("Bearer {}", self.api_key));
        }

        let resp: serde_json::Value = req
            .send_json(payload)
            .map_err(|e| format!("OpenAI API error: {}", e))?
            .into_json()?;

        let content = resp
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|f| f.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|txt| txt.as_str())
            .unwrap_or_default()
            .to_string();

        Ok(content)
    }
}

#[allow(dead_code)]
pub struct OllamaClient {
    pub base_url: String,
    pub model: String,
}

#[allow(dead_code)]
impl OllamaClient {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
        }
    }
}

impl LLMClient for OllamaClient {
    fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        json_mode: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));
        let mut payload = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "stream": false,
            "options": {
                "temperature": 0.1
            }
        });

        if json_mode {
            payload["format"] = serde_json::json!("json");
        }

        let resp: serde_json::Value = ureq::post(&url)
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(120))
            .send_json(payload)
            .map_err(|e| format!("Ollama connection error at {}: {}", self.base_url, e))?
            .into_json()?;

        let content = resp
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|txt| txt.as_str())
            .unwrap_or_default()
            .to_string();

        Ok(content)
    }
}

#[derive(Clone, Default)]
pub struct MockLLMClient {
    pub response_text: String,
    pub calls: Arc<Mutex<Vec<(String, String, bool)>>>,
}

impl MockLLMClient {
    pub fn new(response_text: impl Into<String>) -> Self {
        Self {
            response_text: response_text.into(),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl LLMClient for MockLLMClient {
    fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        json_mode: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if let Ok(mut lock) = self.calls.lock() {
            lock.push((system_prompt.to_string(), user_prompt.to_string(), json_mode));
        }
        Ok(self.response_text.clone())
    }
}

pub fn get_llm_client(
    external_cmd: Option<&str>,
) -> Result<Box<dyn LLMClient>, Box<dyn std::error::Error>> {
    if let Some(cmd) = external_cmd {
        let trimmed = cmd.trim();
        if !trimmed.is_empty() {
            let actual_cmd = trimmed.strip_prefix("cmd:").unwrap_or(trimmed);
            return Ok(Box::new(ExternalCommandLLMClient::new(actual_cmd)?));
        }
    }

    let cfg = config::load_config();
    if let Some(ref configured_cmd) = cfg.external_llm_cmd {
        let trimmed = configured_cmd.trim();
        if !trimmed.is_empty() {
            let actual_cmd = trimmed.strip_prefix("cmd:").unwrap_or(trimmed);
            return Ok(Box::new(ExternalCommandLLMClient::new(actual_cmd)?));
        }
    }

    Err("No external LLM CLI command configured. Provide --llm '<command>' (e.g. --llm \"agy\" or --llm \"ollama run mistral\") or configure it via 'experienced-llms config --llm \"<command>\"'.".into())
}
