use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
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

    pub fn is_anthropic_model(model: &str) -> bool {
        let m = model.trim().to_lowercase();
        m.starts_with("anthropic/") || m.starts_with("claude-")
    }

    /// Unified streaming Referee call: routes Claude models through LiteLLM's Anthropic
    /// Messages passthrough (`/anthropic/v1/messages` with 1h `cache_control`) and DeepSeek /
    /// OpenAI / OpenRouter / Gemini models through `/v1/chat/completions` with streaming tool calls,
    /// returning a normalized Anthropic-style `AnthropicResponse` for uniform turn replay.
    #[allow(clippy::too_many_arguments)]
    pub async fn stream_referee_turn<F>(
        &self,
        model: &str,
        system_blocks: Vec<Value>,
        messages: Vec<Value>,
        tools: Vec<Value>,
        tool_choice: Option<Value>,
        max_tokens: u32,
        on_text_delta: F,
    ) -> Result<AnthropicResponse>
    where
        F: FnMut(&str),
    {
        if Self::is_anthropic_model(model) {
            self.stream_anthropic_messages(
                model,
                system_blocks,
                messages,
                tools,
                tool_choice,
                max_tokens,
                on_text_delta,
            )
            .await
        } else {
            self.stream_openai_chat(
                model,
                system_blocks,
                messages,
                tools,
                tool_choice,
                max_tokens,
                on_text_delta,
            )
            .await
        }
    }

    /// Calls LiteLLM's OpenAI-compatible `/v1/chat/completions` endpoint with SSE streaming and
    /// full tool_calls accumulation (for DeepSeek, GLM, Gemini, GPT, etc.).
    #[allow(clippy::too_many_arguments)]
    pub async fn stream_openai_chat<F>(
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
        let url = format!("{}/v1/chat/completions", self.base_url);
        let oai_messages = Self::anthropic_to_openai_messages(&system_blocks, &messages);
        let oai_tools: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.get("name").cloned().unwrap_or(Value::Null),
                        "description": t.get("description").cloned().unwrap_or(Value::String(String::new())),
                        "parameters": t.get("input_schema").cloned().unwrap_or_else(|| json!({"type": "object", "properties": {}}))
                    }
                })
            })
            .collect();

        let mut payload = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": oai_messages,
            "stream": true,
            "stream_options": { "include_usage": true }
        });
        if !oai_tools.is_empty() {
            payload["tools"] = Value::Array(oai_tools);
        }
        if let Some(tc) = tool_choice
            && tc.get("type").and_then(|v| v.as_str()) == Some("none")
        {
            payload["tool_choice"] = Value::String("none".into());
        }

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .context("Failed to connect to LiteLLM OpenAI endpoint")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LiteLLM OpenAI HTTP {status}: {body}");
        }

        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut text_acc = String::new();
        let mut reasoning_acc = String::new();
        // index -> (id, name, arguments_json)
        let mut tool_calls_acc: BTreeMap<usize, (String, String, String)> = BTreeMap::new();
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

                if let Some(u) = ev.get("usage").filter(|v| !v.is_null()) {
                    usage.input_tokens =
                        u.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    usage.output_tokens = u
                        .get("completion_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32;
                    usage.cache_read_tokens = u
                        .pointer("/prompt_tokens_details/cached_tokens")
                        .or_else(|| u.get("prompt_cache_hit_tokens"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32;
                }

                if let Some(choice) = ev.pointer("/choices/0") {
                    if let Some(fr) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                        stop_reason = if fr == "tool_calls" {
                            "tool_use".into()
                        } else {
                            fr.to_string()
                        };
                    }
                    if let Some(delta) = choice.get("delta") {
                        if let Some(rc) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
                            reasoning_acc.push_str(rc);
                        }
                        if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                            text_acc.push_str(content);
                            on_text_delta(content);
                        }
                        if let Some(tcs) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                            for tc in tcs {
                                let idx =
                                    tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                                let entry = tool_calls_acc
                                    .entry(idx)
                                    .or_insert_with(|| (String::new(), String::new(), String::new()));
                                if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                                    entry.0 = id.to_string();
                                }
                                if let Some(name) =
                                    tc.pointer("/function/name").and_then(|v| v.as_str())
                                {
                                    entry.1.push_str(name);
                                }
                                if let Some(args) =
                                    tc.pointer("/function/arguments").and_then(|v| v.as_str())
                                {
                                    entry.2.push_str(args);
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut content_blocks = Vec::new();
        if !reasoning_acc.is_empty() {
            content_blocks.push(json!({"type": "thinking", "thinking": reasoning_acc}));
        }
        if !text_acc.is_empty() {
            content_blocks.push(json!({"type": "text", "text": text_acc}));
        }
        for (idx, (id, name, args)) in tool_calls_acc {
            let call_id = if id.is_empty() {
                format!("call_{idx}")
            } else {
                id
            };
            let parsed_input: Value = if args.trim().is_empty() {
                json!({})
            } else {
                serde_json::from_str(&args).unwrap_or_else(|_| json!({}))
            };
            content_blocks.push(json!({
                "type": "tool_use",
                "id": call_id,
                "name": name,
                "input": parsed_input
            }));
        }

        Ok(AnthropicResponse {
            content_blocks,
            stop_reason,
            usage,
        })
    }

    fn anthropic_to_openai_messages(system_blocks: &[Value], messages: &[Value]) -> Vec<Value> {
        let mut out = Vec::new();
        let sys_text: String = system_blocks
            .iter()
            .filter_map(|b| b.get("text").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("\n\n");
        if !sys_text.is_empty() {
            out.push(json!({"role": "system", "content": sys_text}));
        }

        for m in messages {
            let role = m.get("role").and_then(|v| v.as_str()).unwrap_or("user");
            let Some(blocks) = m.get("content").and_then(|v| v.as_array()) else {
                if let Some(s) = m.get("content").and_then(|v| v.as_str()) {
                    out.push(json!({"role": role, "content": s}));
                }
                continue;
            };

            if role == "assistant" {
                let mut text_parts = Vec::new();
                let mut tool_calls = Vec::new();
                for b in blocks {
                    match b.get("type").and_then(|v| v.as_str()) {
                        Some("text") => {
                            if let Some(t) = b.get("text").and_then(|v| v.as_str()) {
                                text_parts.push(t);
                            }
                        }
                        Some("tool_use") => {
                            let id = b.get("id").and_then(|v| v.as_str()).unwrap_or("call_0");
                            let name = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            let args = serde_json::to_string(
                                b.get("input").unwrap_or(&json!({})),
                            )
                            .unwrap_or_else(|_| "{}".into());
                            tool_calls.push(json!({
                                "id": id,
                                "type": "function",
                                "function": {"name": name, "arguments": args}
                            }));
                        }
                        _ => {}
                    }
                }
                let content = text_parts.join("\n");
                let mut msg = json!({
                    "role": "assistant",
                    "content": if content.is_empty() { Value::Null } else { Value::String(content) }
                });
                if !tool_calls.is_empty() {
                    msg["tool_calls"] = Value::Array(tool_calls);
                }
                out.push(msg);
            } else {
                // User message may contain tool_result blocks (which map to OpenAI `role: "tool"`)
                // and/or text blocks.
                let mut text_parts = Vec::new();
                for b in blocks {
                    match b.get("type").and_then(|v| v.as_str()) {
                        Some("tool_result") => {
                            let tid = b
                                .get("tool_use_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("call_0");
                            let content = b
                                .get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            out.push(json!({
                                "role": "tool",
                                "tool_call_id": tid,
                                "content": content
                            }));
                        }
                        Some("text") => {
                            if let Some(t) = b.get("text").and_then(|v| v.as_str()) {
                                text_parts.push(t);
                            }
                        }
                        _ => {}
                    }
                }
                if !text_parts.is_empty() {
                    out.push(json!({
                        "role": "user",
                        "content": text_parts.join("\n\n")
                    }));
                }
            }
        }
        out
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
                                    if let Some(t) =
                                        delta.get("thinking").and_then(|v| v.as_str())
                                        && let Some(b) = blocks.get_mut(idx)
                                    {
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
                                        && let Some(b) = blocks.get_mut(idx)
                                    {
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
                            && let Some(b) = blocks.get_mut(idx)
                        {
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
            && !fb.trim().is_empty()
            && fb != primary_model
        {
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
                warn!(
                    "Worker {m} returned empty text at max_tokens={max_tokens}; retrying wider ceiling"
                );
            }
        }
        anyhow::bail!("All worker/summarizer attempts returned empty or failed")
    }
}
