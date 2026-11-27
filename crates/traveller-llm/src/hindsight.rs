use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tracing::warn;

#[derive(Clone)]
pub struct HindsightClient {
    http: Client,
    base_url: String,
    api_key: Option<String>,
    enabled: bool,
}

impl HindsightClient {
    pub fn new(base_url: String, api_key: Option<String>, enabled: bool) -> Self {
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.filter(|k| !k.trim().is_empty()),
            enabled,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled && !self.base_url.is_empty()
    }

    pub fn message_hash(role: &str, content: &str) -> String {
        let mut h = Sha256::new();
        h.update(role.as_bytes());
        h.update(b":");
        h.update(content.as_bytes());
        let bytes = h.finalize();
        bytes.iter().take(12).map(|b| format!("{b:02x}")).collect()
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(key) = &self.api_key {
            req.bearer_auth(key)
        } else {
            req
        }
    }

    pub async fn ensure_bank(&self, bank_id: &str, campaign_name: &str) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }
        let url = format!("{}/v1/default/banks/{}", self.base_url, bank_id);
        let body = json!({
            "name": format!("TravellerLLM: {campaign_name}"),
            "mission": "Episodic memory for a single-player Mongoose Traveller 2e campaign: NPCs, factions, trade leads, promises, discoveries, and ship history."
        });
        let resp = self.apply_auth(self.http.put(&url).json(&body)).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            warn!("Hindsight ensure_bank {bank_id} returned {status}: {text}");
        }
        Ok(())
    }

    pub async fn retain_message(
        &self,
        bank_id: &str,
        role: &str,
        content: &str,
        content_hash: &str,
    ) -> Result<()> {
        if !self.is_enabled() || content.trim().is_empty() {
            return Ok(());
        }
        let url = format!("{}/v1/default/banks/{}/memories", self.base_url, bank_id);
        let doc_id = format!("msg-{content_hash}");
        let body = json!({
            "async": true,
            "items": [{
                "document_id": doc_id,
                "content": format!("[{role}] {content}"),
                "tags": ["chat", role]
            }]
        });
        let resp = self.apply_auth(self.http.post(&url).json(&body)).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            warn!("Hindsight retain_message {doc_id} returned {status}: {text}");
        }
        Ok(())
    }

    /// Chunk and ingest a module knowledge file (Markdown/text lore, patron brief, faction dossier)
    /// into a campaign's Hindsight bank so the Referee can recall module facts & secrets on demand.
    pub async fn retain_knowledge_document(
        &self,
        bank_id: &str,
        module_slug: &str,
        filename: &str,
        title: &str,
        content: &str,
    ) -> Result<usize> {
        if !self.is_enabled() || content.trim().is_empty() {
            return Ok(0);
        }
        let chunks = chunk_markdown(content, 3500);
        let url = format!("{}/v1/default/banks/{}/memories", self.base_url, bank_id);
        let safe_file: String = filename
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
            .collect();

        let mut items = Vec::with_capacity(chunks.len());
        for (idx, chunk) in chunks.iter().enumerate() {
            let doc_id = format!("mod-{module_slug}-{safe_file}-p{}", idx + 1);
            items.push(json!({
                "document_id": doc_id,
                "content": format!("[Module: {module_slug} | File: {filename} | {title} (part {}/{})]\n\n{}", idx + 1, chunks.len(), chunk),
                "tags": ["module", module_slug, filename]
            }));
        }

        let body = json!({
            "async": true,
            "items": items
        });
        let resp = self.apply_auth(self.http.post(&url).json(&body)).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            warn!("Hindsight retain_knowledge_document {module_slug}/{filename} returned {status}: {text}");
        }
        Ok(chunks.len())
    }

    pub async fn delete_message(&self, bank_id: &str, content_hash: &str) -> Result<bool> {
        if !self.is_enabled() {
            return Ok(false);
        }
        let doc_id = format!("msg-{content_hash}");
        let url = format!(
            "{}/v1/default/banks/{}/documents/{}",
            self.base_url, bank_id, doc_id
        );
        let resp = self.apply_auth(self.http.delete(&url)).send().await?;
        Ok(resp.status().is_success())
    }

    pub async fn recall(&self, bank_id: &str, query: &str, max_tokens: u32) -> Result<String> {
        if !self.is_enabled() {
            return Ok("Hindsight memory is disabled.".to_string());
        }
        let url = format!(
            "{}/v1/default/banks/{}/memories/recall",
            self.base_url, bank_id
        );
        let body = json!({
            "query": query,
            "budget": "mid",
            "max_tokens": max_tokens,
            "types": ["world", "experience"]
        });
        let resp = self
            .apply_auth(self.http.post(&url).json(&body))
            .send()
            .await
            .context("Hindsight recall request failed")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Hindsight recall HTTP {status}: {text}");
        }
        let data: Value = resp.json().await?;
        let mut lines = Vec::new();
        if let Some(results) = data.get("results").and_then(|r| r.as_array()) {
            for item in results {
                if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                    lines.push(format!("- {t}"));
                }
            }
        }
        if lines.is_empty() {
            Ok("No matching memories found in Hindsight.".to_string())
        } else {
            Ok(lines.join("\n"))
        }
    }
}

fn chunk_markdown(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for para in text.split("\n\n") {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !current.is_empty() && current.len() + trimmed.len() + 2 > max_chars {
            chunks.push(std::mem::take(&mut current));
        }
        if trimmed.len() > max_chars {
            // Split oversized paragraphs cleanly by characters
            let mut buf = String::new();
            for ch in trimmed.chars() {
                buf.push(ch);
                if buf.len() >= max_chars {
                    chunks.push(std::mem::take(&mut buf));
                }
            }
            if !buf.is_empty() {
                current = buf;
            }
        } else {
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(trimmed);
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    if chunks.is_empty() {
        chunks.push(text.trim().to_string());
    }
    chunks
}
