# Improvements

Prioritized improvements for the extension.

## High Priority

**1. Custom Notification Warning**
- Issue: Zed logs show "unhandled notification supercollider/serverStatus"
- Solution: Remove custom notification or use standard LSP progress
- File: Search for `supercollider/serverStatus` in quark
- Effort: 1-2 hours

**2. Graceful Shutdown**
- Issue: Timeout warnings on shutdown, potential zombie processes
- Solution: Proper shutdown sequence in launcher
- File: `server/launcher/src/main.rs`
- Effort: 2-3 hours

**3. Better Error Messages**
- Issue: Generic errors like "launcher not found"
- Solution: Provide troubleshooting steps in error messages
- Files: `src/lib.rs`, `server/launcher/src/main.rs`
- Effort: 2-3 hours

**4. Config Validation**
- Issue: Could regress config.toml with invalid fields
- Solution: Pre-commit hook + CI validation
- File: `.git/hooks/pre-commit`
- Effort: 1 hour

## Medium Priority

**5. LSPDatabase Caching**
- Issue: Linear search on every lookup
- Solution: LRU cache for frequent symbols
- File: `server/quark/LanguageServer.quark/LSPDatabase.sc`
- Impact: <10ms p95 (from ~50ms)
- Effort: 3-4 hours

**6. Comprehensive Logging**
- Issue: Inconsistent logging across components
- Solution: Use tracing in Rust, Log() in SuperCollider
- Impact: Easier debugging
- Effort: 4-5 hours

**7. Message Batching**
- Issue: Individual UDP sends for each message
- Solution: Batch small messages (10ms window or 10 messages)
- File: `server/launcher/src/main.rs`
- Effort: 3-4 hours

**8. Type Safety**
- Issue: String parsing and JSON manipulation
- Solution: Use `lsp-types` crate
- File: `server/launcher/src/main.rs`
- Effort: 5-6 hours

## Nice to Have

**9. Progress Notifications**
- Feature: Show "Indexing..." in Zed status bar
- File: `server/quark/LanguageServer.quark/LSP.sc`
- Effort: 2-3 hours

**10. Signature Help**
- Feature: Parameter hints while typing
- File: `server/quark/.../SignatureHelpProvider.sc`
- Effort: 4-5 hours

**11. Fuzzy Symbol Search**
- Feature: Cmd+T with fuzzy matching
- File: `server/quark/.../WorkspaceSymbolProvider.sc`
- Effort: 3-4 hours

**12. Hot Reload**
- Feature: Reload quark without restarting sclang
- File: `server/quark/LanguageServer.quark/`
- Effort: 4-5 hours

## Infrastructure

**13. CI Pipeline**
- Tests, linting, config validation
- File: `.github/workflows/ci.yml`
- Effort: 3-4 hours

**14. Integration Tests**
- End-to-end tests with real sclang
- File: `tests/integration_test.rs`
- Effort: 5-6 hours

**15. Benchmarks**
- Track performance over time
- File: `benches/performance.rs`
- Effort: 3-4 hours

## Future Features

**16. Inline Diagnostics**
- Red squiggles for syntax errors
- Requires: New DiagnosticsProvider
- Effort: 6-8 hours

**17. Enhanced Workspace Symbols**
- Search methods, not just classes
- Effort: 4-5 hours

**18. Configuration Schema**
- Auto-complete for settings in Zed
- File: `schema/supercollider-config.json`
- Effort: 2-3 hours

## Implementation Roadmap

**Phase 1: Stability (v0.1.0)**
- Custom notification fix
- Graceful shutdown
- Better error messages
- Config validation
- CI setup

**Phase 2: Performance**
- LSPDatabase caching
- Message batching
- Comprehensive logging
- Benchmarks

**Phase 3: Polish**
- Progress notifications
- Signature help
- Fuzzy search
- Integration tests

## Quick Wins

High impact, low effort improvements:
1. Custom notification fix (1-2h)
2. Config validation (1h)
3. Better error messages (2-3h)
4. Progress notifications (2-3h)

Total: ~8 hours for major quality improvements

## Metrics

**Performance targets:**
- Startup: <2s (current: 2-3s)
- Definition lookup: <100ms p95
- Memory: <50MB steady state

**Reliability targets:**
- Crash rate: 0 per 1000 operations
- Successful shutdown: 100%

**Code quality targets:**
- Test coverage: 70%+
- Clippy warnings: 0
- Documentation: All public APIs

## Deferred

Not doing yet (focus on core features first):
- Custom UI panes (Zed API limitation)
- Visual flash on eval (Zed API limitation)
- Inline result display (Zed API limitation)
- scsynth integration (complex, low ROI)
- Debug console (solve with logging first)

## Reference

For detailed implementation examples, see `.ai/improvements-detailed.md`.
