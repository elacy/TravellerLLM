# TravellerLLM Engineering & Testing Conventions

## Testing Standard
- **Public APIs only:** All tests must live in external integration test suites (`crates/*/tests/*.rs` for Rust, or end-to-end / API tests for server & frontend) so they only call public methods and public types — never inline `#[cfg(test)]` modules reaching into private helpers.
- **Behavioral, end-to-end tests over micro-tests:** Write tests that exercise meaningful, multi-step Traveller workflows (e.g., full character skill checks with augments/wounds/XP progression, ship validation + maintenance/mortgage cycles, sensor detection vs. stealth scenarios) rather than shallow 2-line tests covering trivial getters or individual helper lines.

## Rules & Copyright Hygiene
- **Exact latest Mongoose Traveller 2e rules:** Verify rules against the latest PDFs (Core 2022, CSC 2023, High Guard 2024, Companion 2024, Vehicle Handbook 2026) before implementing mechanics.
- **Public repository safety:** Never commit copyrighted book text, PDFs, or extracted catalogs (`packs/local/` is gitignored). Only engine code, schemas, open starter data (`packs/starter-srd/`), and extraction tooling belong in Git.
