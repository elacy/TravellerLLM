use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tracing::{info, warn};
use traveller_db::Db;
use traveller_rules::universe::{build_spatial_memory_scope, parse_uwp};
use uuid::Uuid;

use crate::hindsight::HindsightClient;
use crate::litellm::LiteLlmClient;
use crate::tools::{execute_tool, tool_specs};

const SUPERSEDED_PLACEHOLDER: &str =
    "[An earlier version of this reply was withdrawn after the player's out-of-character correction. The rewrite follows.]";
const MAX_TOOL_ROUNDS: usize = 8;
const RECENT_MESSAGES_PROTECT: usize = 16;
const SUMMARY_BLOCK_SIZE: usize = 10;

const REFEREE_SYSTEM_PROMPT: &str = r#"You are the Referee for a single-player Mongoose Traveller 2nd Edition (2022/2024 Update) campaign.

Core principles:
- NEVER invent dice results, skill totals, credits, fuel, or ship damage in prose. Always call the deterministic engine tools (`roll_task`, `roll_dice`, `sensor_check`, `advance_time`, `record_transaction`, `award_xp`, `validate_ship`, `system_traffic`).
- Treat `<traveller_state>` and `<story_ledger>` as authoritative ground truth. Each campaign has strictly ONE Player Character (plus any hired Crew and NPCs).
- Whenever the player arrives in a new star system (or asks about ships in-system), ALWAYS state explicitly how many ships are detected in-system from `<traveller_state>` / `system_traffic` and describe notable contacts (orbit, class, transponder status, disposition).
- When adventure Modules are loaded (listed in `<traveller_state>`), call `recall_memory` whenever the player investigates module factions, patrons, locations, or secrets — all module knowledge files have been ingested into Hindsight memory.
- Keep the Third Imperium grounded, gritty, and mechanically real: 4-week ship mortgages, maintenance periods, refined vs. unrefined fuel, law levels, and sensor signatures drive the drama.
- Award per-skill XP through `roll_task` (or `award_xp`) whenever characters (player, hired crew, or recurring NPCs) put their skills to meaningful test.
- Use `update_ledger` when key facts, NPCs, or plot threads shift. Use `supersede_last_reply` if the player gives an out-of-character correction.
- Be vivid, concise, and tactical — stop cleanly once the scene, dialogue, or mechanical outcome has been narrated. NEVER end your response with "What do you do?", "What's your next move?", or a bulleted list of suggested player actions."#;

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub default_referee_model: String,
    pub default_worker_model: String,
    pub summarizer_fallback_model: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TurnEvent {
    Status { message: String },
    ToolCall { id: String, name: String, input: Value },
    ToolResult { id: String, name: String, result: String, is_error: bool },
    StateChanged,
    ClearDraft,
    Delta { text: String },
    Done {
        message_id: i64,
        input_tokens: i32,
        output_tokens: i32,
        cache_read_tokens: i32,
        cache_creation_tokens: i32,
    },
    Error { message: String },
}

#[derive(Clone)]
pub struct Orchestrator {
    db: Db,
    litellm: LiteLlmClient,
    hindsight: HindsightClient,
    config: LlmConfig,
}

impl Orchestrator {
    pub fn new(
        db: Db,
        litellm: LiteLlmClient,
        hindsight: HindsightClient,
        config: LlmConfig,
    ) -> Self {
        Self {
            db,
            litellm,
            hindsight,
            config,
        }
    }

    pub async fn run_turn(
        &self,
        campaign_id: Uuid,
        user_text: String,
        tx: mpsc::Sender<TurnEvent>,
    ) -> Result<()> {
        let camp = self
            .db
            .get_campaign(campaign_id)
            .await?
            .context("Campaign not found")?;

        let _ = tx
            .send(TurnEvent::Status {
                message: "Preparing state & memory…".into(),
            })
            .await;

        // 1. Persist incoming player message
        let user_hash = HindsightClient::message_hash("user", &user_text);
        self.db
            .insert_message(campaign_id, "user", &user_text, &user_hash, None, 0, 0, 0, 0, 0.0)
            .await?;

        // Build spatial/polity/government Hindsight scope for current system
        let sys_rec = self.db.get_system_by_name(&camp.current_system_name).await.ok().flatten();
        let uwp = parse_uwp(&camp.current_system_uwp)
            .unwrap_or_else(|_| parse_uwp("A788899-C").expect("valid uwp"));
        let spatial_scope = build_spatial_memory_scope(
            &camp.current_system_name,
            sys_rec.as_ref().map(|s| s.sector_name.as_str()),
            &uwp,
            sys_rec.as_ref().map(|s| s.allegiance.as_str()).unwrap_or("Im"),
        );

        // Ensure Hindsight bank & retain player message asynchronously with spatial/polity tags
        {
            let hs = self.hindsight.clone();
            let bank = camp.hindsight_bank_id.clone();
            let cname = camp.name.clone();
            let txt = user_text.clone();
            let h = user_hash.clone();
            let tags = spatial_scope.all_tags.clone();
            let ents = spatial_scope.entities.clone();
            tokio::spawn(async move {
                let _ = hs.ensure_bank(&bank, &cname).await;
                let _ = hs.retain_message(&bank, "user", &txt, &h, tags, ents).await;
            });
        }

        // 2. Concurrent pre-turn loads (including automatic spatial/polity-weighted Hindsight recall!)
        let regional_query = format!(
            "{} {} {}",
            camp.current_system_name,
            sys_rec.as_ref().map(|s| s.sector_name.as_str()).unwrap_or(""),
            user_text.chars().take(180).collect::<String>()
        );
        let (state_res, msgs_res, legs_res, summaries_res, regional_mem_res) = tokio::join!(
            self.db.render_state_snapshot(campaign_id),
            self.db.list_messages(campaign_id),
            self.db.load_turn_legs_map(campaign_id),
            self.db.list_summaries(campaign_id),
            self.hindsight.recall_scoped(
                &camp.hindsight_bank_id,
                &regional_query,
                &spatial_scope.local_tags,
                &spatial_scope.polity_tags,
                900
            ),
        );
        let state_xml = state_res?;
        let messages = msgs_res?;
        let legs_map = legs_res?;
        let summaries = summaries_res?;
        let regional_mem = regional_mem_res.unwrap_or_default();

        // 3. Build cache-stable conversation messages:
        //    [Compressed summaries block (cached)] + [Recent messages with replayed turn_legs]
        //    + [Ephemeral <traveller_state> & <story_ledger> AFTER last cache breakpoint]
        let mut api_messages: Vec<Value> = Vec::new();
        let covered_up_to_id = summaries
            .iter()
            .map(|s| s.to_message_id)
            .max()
            .unwrap_or(0);

        if !summaries.is_empty() {
            let mut summary_text = String::from("<campaign_summary>\n");
            for s in &summaries {
                summary_text.push_str(&format!("[Block {}]\n{}\n\n", s.block_index, s.summary_text));
            }
            summary_text.push_str("</campaign_summary>");
            api_messages.push(json!({
                "role": "user",
                "content": [{
                    "type": "text",
                    "text": summary_text,
                    "cache_control": {"type": "ephemeral", "ttl": "1h"}
                }]
            }));
            api_messages.push(json!({
                "role": "assistant",
                "content": [{"type": "text", "text": "Understood — campaign history loaded."}]
            }));
        }

        let recent: Vec<_> = messages
            .iter()
            .filter(|m| m.id > covered_up_to_id)
            .collect();

        for (i, m) in recent.iter().enumerate() {
            let is_last = i + 1 == recent.len();
            if m.role == "assistant" {
                let text = if m.superseded {
                    SUPERSEDED_PLACEHOLDER.to_string()
                } else {
                    m.content.clone()
                };
                // Replay stored tool legs verbatim so Referee never re-derives earlier checks
                if !m.superseded
                    && let Some(legs) = legs_map.get(&m.id) {
                        for leg in legs {
                            api_messages.push(json!({
                                "role": "assistant",
                                "content": leg.assistant_blocks
                            }));
                            if let Some(arr) = leg.tool_results.as_array()
                                && !arr.is_empty() {
                                    api_messages.push(json!({
                                        "role": "user",
                                        "content": leg.tool_results
                                    }));
                                }
                        }
                    }
                api_messages.push(json!({
                    "role": "assistant",
                    "content": [{"type": "text", "text": text}]
                }));
            } else {
                let mut blocks = vec![json!({"type": "text", "text": m.content})];
                // Place conversation cache breakpoint on the user's own text right before dynamic suffix
                if is_last {
                    blocks[0]["cache_control"] = json!({"type": "ephemeral", "ttl": "1h"});
                    blocks.push(json!({"type": "text", "text": &state_xml}));
                    if camp.story_ledger.has_content() {
                        blocks.push(json!({"type": "text", "text": camp.story_ledger.render_xml()}));
                    }
                    if !regional_mem.trim().is_empty() {
                        blocks.push(json!({
                            "type": "text",
                            "text": format!("<regional_memory>\nAutomatically recalled memories weighted by current system, sector, polity, and government type:\n\n{}\n</regional_memory>", regional_mem.trim())
                        }));
                    }
                }
                api_messages.push(json!({
                    "role": "user",
                    "content": blocks
                }));
            }
        }

        let system_blocks = vec![json!({
            "type": "text",
            "text": REFEREE_SYSTEM_PROMPT,
            "cache_control": {"type": "ephemeral", "ttl": "1h"}
        })];
        let tools = tool_specs(self.hindsight.is_enabled());

        // 4. Multi-round tool loop (with MAX_TOOL_ROUNDS wrap-up budget)
        let mut recorded_legs: Vec<(Value, Value)> = Vec::new();
        let mut final_text = String::new();
        let mut total_in = 0i32;
        let mut total_out = 0i32;
        let mut total_cache_read = 0i32;
        let mut total_cache_create = 0i32;

        for round in 0..=MAX_TOOL_ROUNDS {
            let wrap_up = round == MAX_TOOL_ROUNDS;
            let tool_choice = if wrap_up {
                api_messages.push(json!({
                    "role": "user",
                    "content": [{"type": "text", "text": "[Tool budget reached for this turn. Summarize all results so far and reply to the player without calling more tools.]"}]
                }));
                Some(json!({"type": "none"}))
            } else {
                None
            };

            let _ = tx
                .send(TurnEvent::Status {
                    message: if round == 0 {
                        format!("Referee ({}) thinking…", camp.referee_model)
                    } else {
                        format!("Tool round {}…", round + 1)
                    },
                })
                .await;

            let mut round_text = String::new();
            let resp = self
                .litellm
                .stream_referee_turn(
                    &camp.referee_model,
                    system_blocks.clone(),
                    api_messages.clone(),
                    tools.clone(),
                    tool_choice,
                    8192,
                    |delta| {
                        round_text.push_str(delta);
                        let _ = tx.try_send(TurnEvent::Delta {
                            text: delta.to_string(),
                        });
                    },
                )
                .await?;

            total_in += resp.usage.input_tokens;
            total_out += resp.usage.output_tokens;
            total_cache_read += resp.usage.cache_read_tokens;
            total_cache_create += resp.usage.cache_creation_tokens;

            let tool_uses: Vec<&Value> = resp
                .content_blocks
                .iter()
                .filter(|b| b.get("type").and_then(|v| v.as_str()) == Some("tool_use"))
                .collect();

            if tool_uses.is_empty() || wrap_up {
                final_text = round_text;
                break;
            }

            // Clear any pre-tool scratch text streamed during this tool round so the
            // frontend only displays the final Referee response cleanly.
            if !round_text.is_empty() {
                let _ = tx.send(TurnEvent::ClearDraft).await;
            }

            // Execute each tool call deterministically in Rust
            let mut tool_results = Vec::new();
            for tu in tool_uses {
                let id = tu
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("tool")
                    .to_string();
                let name = tu
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let input = tu.get("input").cloned().unwrap_or_else(|| json!({}));

                let _ = tx
                    .send(TurnEvent::ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                    })
                    .await;

                let (content, is_error, changed) =
                    match execute_tool(&self.db, &self.hindsight, campaign_id, &name, &input).await
                    {
                        Ok(r) => (r.output_json, false, r.state_changed),
                        Err(e) => (format!("Tool error ({name}): {e}"), true, false),
                    };

                let _ = tx
                    .send(TurnEvent::ToolResult {
                        id: id.clone(),
                        name: name.clone(),
                        result: content.clone(),
                        is_error,
                    })
                    .await;
                if changed {
                    let _ = tx.send(TurnEvent::StateChanged).await;
                }

                let mut res_block = json!({
                    "type": "tool_result",
                    "tool_use_id": id,
                    "content": content
                });
                if is_error {
                    res_block["is_error"] = Value::Bool(true);
                }
                tool_results.push(res_block);
            }

            let assistant_blocks_val = Value::Array(resp.content_blocks);
            let tool_results_val = Value::Array(tool_results);
            recorded_legs.push((assistant_blocks_val.clone(), tool_results_val.clone()));

            api_messages.push(json!({
                "role": "assistant",
                "content": assistant_blocks_val
            }));
            api_messages.push(json!({
                "role": "user",
                "content": tool_results_val
            }));
        }

        // 5. Persist Referee reply + turn_legs
        let reply_hash = HindsightClient::message_hash("assistant", &final_text);
        let saved = self
            .db
            .insert_message(
                campaign_id,
                "assistant",
                &final_text,
                &reply_hash,
                Some(&camp.referee_model),
                total_in,
                total_out,
                total_cache_read,
                total_cache_create,
                0.0,
            )
            .await?;
        if !recorded_legs.is_empty() {
            self.db
                .save_turn_legs(campaign_id, saved.id, &recorded_legs)
                .await?;
        }

        let _ = tx
            .send(TurnEvent::Done {
                message_id: saved.id,
                input_tokens: total_in,
                output_tokens: total_out,
                cache_read_tokens: total_cache_read,
                cache_creation_tokens: total_cache_create,
            })
            .await;

        // 6. Post-turn async background tasks (zero player latency):
        //    a) Retain assistant reply to Hindsight
        //    b) Build Level-1 summary block with cheap worker model if uncompressed tail > threshold
        let hs = self.hindsight.clone();
        let bank = camp.hindsight_bank_id.clone();
        let reply_clone = final_text.clone();
        let reply_tags = spatial_scope.all_tags.clone();
        let reply_ents = spatial_scope.entities.clone();
        tokio::spawn(async move {
            let _ = hs
                .retain_message(&bank, "assistant", &reply_clone, &reply_hash, reply_tags, reply_ents)
                .await;
        });

        let orch = self.clone();
        let worker_model = camp.worker_model.clone();
        tokio::spawn(async move {
            if let Err(e) = orch
                .maybe_compress_history(campaign_id, &worker_model)
                .await
            {
                warn!("Background context compression failed: {e}");
            }
        });

        Ok(())
    }

    async fn maybe_compress_history(&self, campaign_id: Uuid, worker_model: &str) -> Result<()> {
        let msgs = self.db.list_messages(campaign_id).await?;
        let summaries = self.db.list_summaries(campaign_id).await?;
        let covered = summaries.iter().map(|s| s.to_message_id).max().unwrap_or(0);
        let uncovered: Vec<_> = msgs.iter().filter(|m| m.id > covered && !m.superseded).collect();

        if uncovered.len() < RECENT_MESSAGES_PROTECT + SUMMARY_BLOCK_SIZE {
            return Ok(());
        }

        let block = &uncovered[..SUMMARY_BLOCK_SIZE];
        let next_block_idx = summaries
            .iter()
            .filter(|s| s.level == 1)
            .map(|s| s.block_index + 1)
            .max()
            .unwrap_or(0);

        let mut transcript = String::new();
        for m in block {
            transcript.push_str(&format!("{}: {}\n\n", m.role.to_uppercase(), m.content));
        }

        let (summary_text, used_model) = self
            .litellm
            .worker_complete(
                worker_model,
                self.config.summarizer_fallback_model.as_deref(),
                "You compress Traveller RPG session logs into tight, factual summaries preserving exact NPC names, locations, promises, debts, items, injuries, and unresolved hooks. No prose fluff.",
                &transcript,
            )
            .await?;

        self.db
            .upsert_summary(
                campaign_id,
                1,
                next_block_idx,
                block.first().unwrap().id,
                block.last().unwrap().id,
                &summary_text,
                &used_model,
            )
            .await?;
        info!(
            "Compressed block {} (msgs {}..={}) using {}",
            next_block_idx,
            block.first().unwrap().id,
            block.last().unwrap().id,
            used_model
        );
        Ok(())
    }
}
