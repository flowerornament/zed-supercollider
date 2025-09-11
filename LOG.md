# Activity Log

This log captures high-level actions taken by the agent for transparency and traceability.

- 2025-09-11 — Added `AGENTS.md`
  - Created initial contributor guide tailored for a Zed SuperCollider extension (structure, build/test, style, testing, PR guidelines).

- 2025-09-11 — Merged scnvim migration guidance into `AGENTS.md`
  - Rewrote to align with Rust + Tree-sitter layout; added sections for launcher, tasks, docs, and migration parity goals.
  - Linked to a forthcoming detailed migration plan.

- 2025-09-11 — Added `docs/MIGRATION_PLAN.md`
  - Captured the full Navigator plan (ADR-001, milestones, risks, prompts, acceptance criteria) separate from the concise AGENTS guide.

- 2025-09-11 — Vendored scnvim reference docs
  - Created `docs/reference/scnvim/` with `README.md`, `SOURCES.md`, `NOTICE`.
  - Pulled upstream snapshot (for contributor reference only):
    - `UPSTREAM_README.md` (from scnvim README)
    - `SCNvim.txt` (help doc)
    - `LICENSE` (GPL-3.0)
  - Pin details: repo `github.com/davidgranstrom/scnvim`, commit `8148e9b5700956b14b0202ee4b08d6856510d3fd`, license `GPL-3.0`.

- 2025-09-11 — Added `PLAN.md`
  - Step-by-step implementation plan with milestones (M1–M5), tasks, files to touch, validation, acceptance, risks, and PR sequence.

- 2025-09-11 — Reviewed official SuperCollider docs; updated semantics
  - Fetched SC docs on client/server, `CmdPeriod`, `Server`, and `ThisProcess`.
  - Updated `AGENTS.md`, `PLAN.md`, and `docs/MIGRATION_PLAN.md` to clarify:
    - Client (`sclang`) ↔ Server (`scsynth`) via OSC.
    - `hardStop` maps to `CmdPeriod.run`; `recompile` maps to `thisProcess.recompile`.
    - Boot/quit server via `s.boot`/`s.quit`.

Planned next steps (pending):
- Scaffold extension skeleton (extension.toml, Cargo.toml, src/lib.rs) and initial Tree-sitter queries (M1). — Completed
- Add CI and basic tests; create docs stubs (`docs/MIGRATION.md`, `docs/TROUBLESHOOTING.md`).

- 2025-09-11 — Scaffolded M1 skeleton
  - Added `extension.toml` (grammar pin placeholder), `Cargo.toml`, and `src/lib.rs` with minimal `zed_extension_api` extension.
  - Added language config `languages/SuperCollider/config.toml` and minimal queries (`highlights.scm`, `brackets.scm`, `indents.scm`, `outline.scm`).
  - Added starter snippets at `snippets/supercollider.json` (SynthDef, Pbind).

- 2025-09-11 — Read Zed extension docs; aligned plan
  - Confirmed Rust cdylib + `register_extension!` pattern and using latest `zed_extension_api` compatible with target Zed versions.
  - Adjusted fallback post window approach: provide user Task snippet in docs instead of bundling a `.ztask.json` file; consider an extension command to spawn `sclang` if API allows.

- 2025-09-11 — Initialized git repository and committed scaffold
  - Added `.gitignore` and removed cached artifacts; first commit with scaffold and docs.

- 2025-09-11 — M2 kick-off: LSP launcher stub
  - Added `server/launcher` Rust bin crate. Currently probes `sclang -v`; to be extended with Quark install and stdio bridge.
