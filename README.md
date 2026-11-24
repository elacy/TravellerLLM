# TravellerLLM

A standalone, single-player **Mongoose Traveller 2nd Edition (2022–2026 Updates)** RPG engine with a **Rust** rules backend, **React** Command Deck frontend, **PostgreSQL** authoritative game state, **Hindsight** episodic narrative memory, and cost-tiered **LiteLLM** orchestration.

## Architecture

- **`crates/traveller-rules`** — Pure, deterministic Rust rules engine (zero network/DB dependencies):
  - `2D6` / Boon-Bane `3D6` task resolution with Effect, Timeframes, and Task Chains
  - Exact **Wafer Jack / Neural Link / Neural Jack + Expert Software** bonus calculation
  - Per-skill activity XP (`Companion 2024` doubling curve: `1 / 1 / 2 / 4 / 8 / 16 / 32`) for PCs, hired crew, and NPCs
  - **Imperial Calendar** (`DDD-YYYY HH:MM`), 1-week Jumps (`168 ± 6D6h`), 28-day Maintenance Periods
  - **Ship Mortgages** (`Price / 240`, Ship Shares, Career Benefits, Skipping on Debts checks) & **Ship Maintenance** (`0.1%/yr ÷ 12`, annual overhaul, Poor Maintenance `2D` critical hit table)
  - **High Guard 2024** Ship Validator (tonnage, power plant budget, fuel, hardpoints/firmpoints, crew)
  - **High Guard 2024** Sensor Detection & Stealth calculator (sensor grade, TL diff, active/passive, thrust, power plant, transponder, stealth coating, armor mods)
  - **Trade, Freight, Passengers, Mail, Speculative Cargo, Personal vs. Company Accounts, Personal & Spacecraft Combat**
- **`crates/traveller-db`** — SQLx PostgreSQL repositories & migrations.
- **`crates/traveller-llm`** — LiteLLM orchestration modeled after `anthropic-pipe`:
  - Anthropic Messages API with 1-hour prompt caching (`cache_control: ephemeral`)
  - Post-breakpoint ephemeral `<traveller_state>` and `<story_ledger>` injection (never breaks cache)
  - Hierarchical PostgreSQL-backed context compression (cheap summarizer + fallback model)
  - Full turn transcript replay (`thinking`, `tool_use`, `tool_result`)
  - Per-campaign **Hindsight** memory bank sync & `supersede_last_reply`
  - Tool budget enforcement with clean wrap-up round
- **`crates/traveller-server`** — Axum REST + SSE server serving the compiled React SPA.
- **`frontend/`** — React + TypeScript + Vite + Tailwind Command Deck UI.

## Copyright & Data Packs

This repository is public and contains **no copyrighted Mongoose Publishing text or catalogs**. Engine mechanics, schemas, and a minimal open starter pack (`packs/starter-srd/`) are included. Personal book data packs extracted from legally owned PDFs belong in `packs/local/` (gitignored) and are mounted into the Docker container at `/data/packs`.

## Quick Start (Docker / NAS)

```bash
cp .env.example .env
# Edit DATABASE_URL, LITELLM_BASE_URL, LITELLM_API_KEY, HINDSIGHT_BASE_URL
docker compose up -d --build
```

Open `http://localhost:3080` (or `http://nas.lacy.ie:3080`).

## Local Development

```bash
# Backend
cargo test
cargo run -p traveller-server

# Frontend (in another terminal)
cd frontend && npm install && npm run dev
```
