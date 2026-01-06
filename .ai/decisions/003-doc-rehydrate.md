---
title: "ADR-003: Doc Rehydration for Early LSP Events"
created: 2026-01-05
updated: 2026-01-05
status: Accepted
purpose: "Architectural decision to cache and rehydrate document state when LSP events arrive before didOpen"
---

# ADR-003: Doc Rehydration for Early LSP Events

## Context

Zed sometimes delivers `didOpen`/`didChange` before TextDocumentProvider registers. Without an open doc, definition/references/hover/outline return nil because `doc.isOpen` and `doc.string` are unset.

## Decision

Queue pending `didOpen`/`didChange` events and cache last `didOpen` payload per URI. Providers rehydrate document text from cache when doc isn't open or has no string.

## Rationale

Ensures LSP lookups have text even if events arrive before provider registration, avoiding user-visible null results.

## Consequences

Relies on cached didOpen text; stale content possible if didChange is missed. Non-standard LSP behavior but necessary for race condition handling.
