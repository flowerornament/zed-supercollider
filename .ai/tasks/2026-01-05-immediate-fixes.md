---
title: "Immediate Fixes (P0) – Stability & Correctness"
date: 2026-01-05
priority: P0
status: active
owners: [team]
updated: 2026-01-05
---

# Immediate Fixes (P0) – Stability & Correctness

Single-source checklist for shipping the highest-priority fixes. Use this when triaging or landing urgent changes. Keep code + docs in sync and link evidence for each item.

## Status Log
- 2026-01-05: Status set to active; checklist items remain in flight.

## Scope & Link
- Highest-priority fixes to ship immediately. See umbrella: `.ai/tasks/2026-01-05-execution-plan.md`.

## Capability Hygiene
- **Launcher initialize response**
  - Remove `serverStatus`, `documentSymbolProvider`, and on-type formatting from default capabilities.
  - Ensure completion/hover/definition/references/selectionRange/folding/codeLens/codeAction/workspaceSymbol/executeCommand remain accurate.
  - Verification: capture initialize response once (`test_lsp.sh` or Zed logs) and confirm providers match advertised behavior.
- **Quark providers**
  - Only advertise formatting when `formatterEnabled` is true; return nil options otherwise.
  - Keep debug logs behind `SCLANG_LSP_DEBUG`; no unconditional `postln`.
  - Verification: open `.scd` file, confirm no documentSymbol requests in logs; formatting disabled unless explicitly enabled.

## Dev Launcher Selection
- Use dev launcher only if `Cargo.toml` exists **and** `server/launcher/target/release/sc_launcher` exists.
- Log clearly when dev binary is missing and suggest `cargo build --release` in `server/launcher`.
- Precedence order: settings binary path → PATH → dev binary (if built).
- Verification: fresh clone without build does not ENOENT; built tree auto-selects dev binary.

## Safe Startup/Shutdown
- Remove global `pkill -9 sclang` on startup; never kill user processes preemptively.
- On stdin close: send LSP `shutdown` + `exit`, close stdin, wait with timeout, TERM then KILL only if needed; log each step.
- Ensure bridges/HTTP server exit when shutdown flag set; no orphaned sclang/scsynth/sc_launcher.
- Verification: close stdin in test harness, check processes with `pgrep sclang`/`pgrep scsynth`; confirm clean exit logs.

## Probe JSON Correctness
- Build probe output with `serde_json::json!` then `to_string()`; escape quotes/backslashes safely.
- Include sclang path and version; non-zero exit on failure with stderr surfaced.
- Verification: `sc_launcher --mode probe` prints valid JSON (parse via `jq`), includes path/version.

## Logging Defaults
- Default to stderr only; gate file logs behind `SC_LAUNCHER_DEBUG_LOGS`.
- Use `SC_TMP_DIR` or `TMPDIR` for all files (post log, stdin log, startup log); never hardcode `/tmp`.
- Post window log should omit LSP protocol chatter; only user-facing output.
- Verification: run once with/without `SC_LAUNCHER_DEBUG_LOGS`; confirm file paths respect env and post log is readable.

## Tasks Cleanup
- Inline curl eval/control tasks; no absolute paths; configurable via `SC_HTTP_PORT`.
- Show HTTP status (`-w 'HTTP %{http_code}'`) for quick feedback.
- Remove `.zed/eval.sh`; ensure runnables still map to eval task via tags.
- Emergency cleanup task: TERM-first then KILL sc processes; clear logs in temp dir; avoid global nukes.
- Verification: spot-check eval + one control task for status output; emergency task should only target sc processes.

## Config Guardrails
- `scripts/validate-config.sh` should fail on banned keys (`opt_into_language_servers`, `scope_opt_in_language_servers`, etc.) and missing required keys.
- Wire into CI/pre-commit (TODO); run locally after config edits.
- Verification: script passes on current tree; inject banned key to confirm failure.

## Error Messaging Polish
- Slash command `supercollider-check-setup` reports stdout/stderr and adds troubleshooting when failing (settings snippet + quark install hint).
- Missing launcher error: actionable guidance (set `lsp.binary.path`, add to PATH, install quark).
- Log when dev binary absent to prevent silent ENOENT.
- Verification: run slash command once with/without launcher; ensure output is actionable.

## Quark Safety
- Close files with `File.use` in `LSPDatabase.renderMethodRange`; default to `[0,0]` on errors.
- Coerce `includeDeclaration` to Bool in `FindReferencesProvider`; gate debug logging via env.
- Remove `ServerStatusNotification` entirely; ensure no remaining references.
- Verification: run references with includeDeclaration true/false; no “Non Boolean in test”; no serverStatus notifications in logs.

## Carryover Quick Wins
- Keep `languages/SuperCollider/config.toml` minimal with `word_characters = ["a-zA-Z0-9_?"]`.
- README includes setup/usage/troubleshooting; notes HTTP eval is fire-and-forget (results in Post Window).
- Verification: `scripts/validate-config.sh` passes; README steps align with launcher behavior.
