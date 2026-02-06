title: "Distribution Proposal – Zed SuperCollider"
created: 2026-01-07
updated: 2026-01-11
priority: proposal
status: draft
owners: [team]
purpose: "Proposal to make the extension installable for SuperCollider users without cloning the repo, while keeping dev ergonomics."
---

# Distribution Proposal – Zed SuperCollider

## Context
- The repo currently combines the Zed extension, the Rust launcher bridge, the vendored LanguageServer.quark, and dev-only helpers (.zed tasks, tree-sitter source). This works when Zed is opened on the repo, but not for users editing their own SC projects.
- Launcher discovery assumes the current worktree is this repo (dev binary fallback at server/launcher/target/release/sc_launcher, vendored quark under server/quark/LanguageServer.quark, .zed/tasks.json). End users will instead have the launcher installed elsewhere and no repo-relative paths.
- Goal: cleanly separate what ships in the extension vs. what ships with the launcher vs. what users must install themselves (SuperCollider app), and document/setup a reproducible install path.

## Principles
- Keep the Zed extension self-contained: grammar .wasm, queries, snippets/keymaps, runnables, packaged tasks if feasible. No dependence on per-project .zed folders.
- Treat the launcher + quark as external dependencies discoverable via PATH or explicit settings; avoid requiring a repo checkout for runtime.
- Prefer deterministic discovery over heuristics; log actionable help when components are missing.
- Preserve dev ergonomics (vendored quark, dev launcher fallback) without leaking dev-only defaults into the published experience.

## Proposed Plan
- Extension packaging
  - Ship runnables-backed tasks inside the extension (or provide a tiny installer that writes to ~/.config/zed/tasks.json) so users do not need .zed/tasks.json in each project. Keep .zed/ as dev-only convenience.
  - Clean extension metadata: repository URL, authorship, versioning scheme, and clarify README/install steps (what the extension provides vs. what the launcher/quark provide).
  - Audit language config/queries/snippets for shipping: keep tree-sitter .wasm only; ensure grammar source and dev build artifacts stay dev-only.
- Launcher distribution
  - Publish sc_launcher as a standalone binary (options: Homebrew tap, GitHub release tarball, cargo install) with clear install command; align README + extension help text with that path.
  - Adjust launcher discovery in src/lib.rs to prefer settings.path > PATH, and reserve dev fallback solely for repo checkouts (no noisy stderr when absent).
  - Add a lightweight `sc_launcher --mode probe` health check users can run post-install; wire slash-command help to it.
- LanguageServer.quark strategy (pick one)
  - Option A (simplest): require users to install LanguageServer via Quarks; remove vendored include-path logic from release docs; keep vendored copy for tests/dev.
  - Option B (better UX): ship the quark adjacent to the launcher (e.g., share/LanguageServer.quark) and resolve it relative to the launcher executable; keep Quarks.install as fallback.
  - Option C (most self-contained): embed the quark in the launcher and extract to a temp/share dir on first run; maintain version sync with vendored source.
  - Whichever option is chosen, align launcher_not_found_help + setup docs and remove mixed-mode behavior that depends on cwd.
- Tasks/runnables UX
  - Ensure runnables.scm tags map to packaged tasks; expose keybindings/snippets for eval/boot/stop without requiring custom project config.
  - Provide a single “Open Post Window” task tied to SC_TMP_DIR/TMPDIR logs; document where eval results appear.
- Documentation and support flow
  - Write a user-facing INSTALL.md covering prerequisites (SuperCollider app), launcher install, quark install strategy, and Zed settings snippet.
  - Add a troubleshooting section keyed to logs (sc_launcher_startup.log, sclang_post.log) and common errors (missing quark, missing sclang, port in use).
  - Keep a separate DEV.md capturing how to use the vendored quark and dev launcher fallback when hacking on the repo.
- Release/verification
  - CI job to build sc_launcher release binary + tree-sitter .wasm, and optionally publish artifacts; validate extension package does not include dev-only files.
  - Smoketests: launcher probe works, LSP initialize succeeds, HTTP eval endpoint responds 202, packaged tasks fire over HTTP without .zed/.
  - Versioning policy: tag launcher + extension together or note compatibility matrix; document env vars (SC_HTTP_PORT, SC_TMP_DIR, SC_LAUNCHER_DEBUG*).

## Open Decisions to Lock
- Choose quark distribution path (A/B/C above) and make launcher/quark discovery deterministic.
- Decide primary install channel for sc_launcher (brew tap vs. cargo install vs. release tarball) and align help text + scripts accordingly.
- Decide whether packaged tasks live in extension bundle or a one-time installer writes them globally; retire per-project .zed dependency either way.
