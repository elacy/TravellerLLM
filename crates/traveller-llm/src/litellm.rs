use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::warn;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageStats {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_read_tokens: i32,
    pub cache_creation_tokens: i32,
}

#[derive(Debug, Clone)]
pub struct AnthropicResponse {
    pub content_blocks: Vec<Value>,
    pub stop_reason: String,
    pub usage: UsageStats,
}

#[derive(Clone)]
pub struct LiteLlmClient {
    http: Client,
    base_url: String,
    api_key: String,
}

impl LiteLlmClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_default(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }

    /// Calls LiteLLM's Anthropic Messages passthrough (`/anthropic/v1/messages`) with streaming SSE,
    /// supporting prompt caching (`cache_control`), thinking blocks, and native tool_use blocks.
    #[allow(clippy::too_many_arguments)]
    pub async fn stream_anthropic_messages<F>(
        &self,
        model: &str,
        system_blocks: Vec<Value>,
        messages: Vec<Value>,
        tools: Vec<Value>,
        tool_choice: Option<Value>,
        max_tokens: u32,
        mut on_text_delta: F,
    ) -> Result<AnthropicResponse>
    where
        F: FnMut(&str),
    {
        // Strip leading "anthropic/" prefix if passed to the /anthropic passthrough endpoint
        let api_model = model.strip_prefix("anthropic/").unwrap_or(model);
        let url = format!("{}/anthropic/v1/messages", self.base_url);

        let mut payload = json!({
            "model": api_model,
            "max_tokens": max_tokens,
            "system": system_blocks,
            "messages": messages,
            "stream": true,
        });
        if !tools.is_empty() {
            payload["tools"] = Value::Array(tools);
        }
        if let Some(tc) = tool_choice {
            payload["tool_choice"] = tc;
        }

        let resp = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "prompt-caching-2024-07-31")
            .json(&payload)
            .send()
            .await
            .context("Failed to connect to LiteLLM Anthropic endpoint")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LiteLLM Anthropic HTTP {status}: {body}");
        }

        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut blocks: Vec<Value> = Vec::new();
        let mut partial_json: std::collections::HashMap<usize, String> =
            std::collections::HashMap::new();
        let mut stop_reason = String::from("end_turn");
        let mut usage = UsageStats::default();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.context("Stream read error")?;
            buf.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(pos) = buf.find('\n') {
                let line = buf[..pos].trim_end_matches('\r').to_string();
                buf.drain(..=pos);

                let Some(data) = line.strip_prefix("data: ") else {
                    continue;
                };
                if data == "[DONE]" || data.is_empty() {
                    continue;
                }
                let Ok(ev) = serde_json::from_str::<Value>(data) else {
                    continue;
                };
                let ev_type = ev.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match ev_type {
                    "message_start" => {
                        if let Some(u) = ev.pointer("/message/usage") {
                            usage.input_tokens =
                                u.get("input_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                            usage.cache_read_tokens = u
                                .get("cache_read_input_tokens")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0)
                                as i32;
                            usage.cache_creation_tokens = u
                                .get("cache_creation_input_tokens")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0)
                                as i32;
                        }
                    }
                    "content_block_start" => {
                        let idx = ev.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                        if let Some(cb) = ev.get("content_block") {
                            while blocks.len() <= idx {
                                blocks.push(Value::Null);
                            }
                            blocks[idx] = cb.clone();
                            if cb.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                                partial_json.insert(idx, String::new());
                            }
                        }
                    }
                    "content_block_delta" => {
                        let idx = ev.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                        if let Some(delta) = ev.get("delta") {
                            let dtype = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            match dtype {
                                "text_delta" => {
                                    if let Some(t) = delta.get("text").and_then(|v| v.as_str()) {
                                        on_text_delta(t);
                                        if let Some(b) = blocks.get_mut(idx) {
                                            let cur = b
                                                .get("text")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");
                                            b["text"] = Value::String(format!("{cur}{t}"));
                                        }
                                    }
                                }
                                "thinking_delta" => {
                                    if let Some(t) = delta.get("thinking").and_then(|v| v.as_str())
                                        && let Some(b) = blocks.get_mut(idx) {
                                            let cur = b
                                                .get("thinking")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");
                                            b["thinking"] = Value::String(format!("{cur}{t}"));
                                        }
                                }
                                "signature_delta" => {
                                    if let Some(s) =
                                        delta.get("signature").and_then(|v| v.as_str())
                                        && let Some(b) = blocks.get_mut(idx) {
                                            let cur = b
                                                .get("signature")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");
                                            b["signature"] = Value::String(format!("{cur}{s}"));
                                        }
                                }
                                "input_json_delta" => {
                                    if let Some(pj) =
                                        delta.get("partial_json").and_then(|v| v.as_str())
                                    {
                                        partial_json.entry(idx).or_default().push_str(pj);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    "content_block_stop" => {
                        let idx = ev.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                        if let Some(raw) = partial_json.remove(&idx)
                            && let Some(b) = blocks.get_mut(idx) {
                                let parsed: Value = if raw.trim().is_empty() {
                                    json!({})
                                } else {
                                    serde_json::from_str(&raw).unwrap_or_else(|_| json!({}))
                                };
                                b["input"] = parsed;
                            }
                    }
                    "message_delta" => {
                        if let Some(sr) = ev.pointer("/delta/stop_reason").and_then(|v| v.as_str())
                        {
                            stop_reason = sr.to_string();
                        }
                        if let Some(out) =
                            ev.pointer("/usage/output_tokens").and_then(|v| v.as_i64())
                        {
                            usage.output_tokens = out as i32;
                        }
                    }
                    _ => {}
                }
            }
        }

        let content_blocks = blocks.into_iter().filter(|b| !b.is_null()).collect();
        Ok(AnthropicResponse {
            content_blocks,
            stop_reason,
            usage,
        })
    }

    /// Cheap worker/summarizer completion via LiteLLM's OpenAI-compatible `/v1/chat/completions`,
    /// mirroring anthropic-pipe's escalating ceiling + fallback model pattern.
    pub async fn worker_complete(
        &self,
        primary_model: &str,
        fallback_model: Option<&str>,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<(String, String)> {
        let budgets = [2048u32, 4600, 9200];
        let mut models = vec![primary_model];
        if let Some(fb) = fallback_model
            && !fb.trim().is_empty() && fb != primary_model {
                models.push(fb);
            }

        let url = format!("{}/v1/chat/completions", self.base_url);
        for m in models {
            for &max_tokens in &budgets {
                let payload = json!({
                    "model": m,
                    "max_tokens": max_tokens,
                    "temperature": 0.2,
                    "messages": [
                        {"role": "system", "content": system_prompt},
                        {"role": "user", "content": user_prompt}
                    ]
                });
                let resp = match self
                    .http
                    .post(&url)
                    .bearer_auth(&self.api_key)
                    .json(&payload)
                    .send()
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("Worker call to {m} failed: {e}");
                        break;
                    }
                };
                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    warn!("Worker call to {m} HTTP {status}: {body}");
                    break;
                }
                let data: Value = resp.json().await?;
                let text = data
                    .pointer("/choices/0/message/content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !text.is_empty() {
                    return Ok((text, m.to_string()));
                }
                warn!("Worker {m} returned empty text at max_tokens={max_tokens}; retrying wider ceiling");
            }
        }
        anyhow::bail!("All worker/summarizer attempts returned empty or failed")
    }
}
