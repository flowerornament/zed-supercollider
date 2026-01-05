# Decision: Doc Rehydration for Early LSP Events

## Context
- Zed sometimes delivers `didOpen`/`didChange` before the TextDocumentProvider registers.
- Without an open doc, definition/references/hover/outline return `nil` because `doc.isOpen` and `doc.string` are unset.

## Decision
- Queue `didOpen`/`didChange` in `TextDocumentProvider.queuePending`, keyed by method string.
- Cache the last `didOpen` payload per URI (`lastOpenByUri`).
- In providers (definition/references/documentSymbol/hover), if the doc isn’t open or has no string, rehydrate from `lastOpenByUri` and mark `isOpen_(true)` before lookup.
- Log at `info` during verification; lower to `warning` once validated.

## Rationale
- Ensures lookups have text even if events arrive before provider registration.
- Avoids user-visible null results in go-to-definition/references/hover/outline.

## Consequences
- Slightly non-standard LSP behavior; relies on cached didOpen text, so stale content is possible if didChange is missed.
- Additional logging during verification; should be reduced once stable.

## Status
- Implemented 2026-01-06.
