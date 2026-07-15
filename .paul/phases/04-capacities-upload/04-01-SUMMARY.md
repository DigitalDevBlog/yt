---
phase: 04-capacities-upload
plan: 01
completed: 2026-07-15
duration: ~90min (incl. API research + two live-caught fixes)
---

# Phase 4 Plan 01: Upload to Capacities Summary

**Added `-u`/`--upload`: after summarizing, create an Atomic Note in Capacities
(video title + summary body) and append a link to it at the bottom of the Inbox
page — as a 4th step in the stepped progress UI.**

## Objective

Capture each watched video as an atomic node in a personal knowledge base
(v0.3 "Integrations" milestone).

## What Was Built

| File | Change |
|------|--------|
| src/main.rs | Capacities v2.0 REST client (`GET /space`, `GET /space/structures`, `POST /object/markdown`, `POST /objects/search`, `POST /blocks/append` + a verified read-back) and `capacities_publish` orchestrator; `-u`/`--upload` flag (implies `-s`); 4th "Uploading to Capacities" step in `run_summary`; graceful, non-crashing failure |
| Cargo.toml / Cargo.lock | version 0.2.0 → 0.3.0 (new feature) |
| MANUAL.md, README.md | documented `--upload`, Capacities config, errors |

## Acceptance Criteria Results

| AC | Description | Status |
|----|-------------|--------|
| AC-1 | `--upload` creates an Atomic Note (title + markdown body) | Pass (live) |
| AC-2 | link appended to the bottom of the "Inbox" page | Pass (live, verified read-back) |
| AC-3 | `--upload` implies `--summarize` (writes `<title>.md`) | Pass |
| AC-4 | graceful failure keeps the `.md`, no panic | Pass (first live run failed cleanly) |
| AC-5 | "Uploading to Capacities" progress step | Pass |

## Verification Results

- `cargo build` clean; `cargo test` — 20 passed
- Live: Atomic Note created (correct title + summary), exactly **one** link at
  the bottom of the Inbox (no duplicates), test notes hard-deleted afterward
- Negative: missing key / API error → `✗` on the step, `.md` kept, exit 0

## Deviations (both caught by live testing — the write paths were spec-only)

1. **`/blocks/append` body shape.** First live run returned HTTP 400: `position`
   must be an object `{ "type": "end" }` (not the string `"end"`), and the block
   type discriminator is `"EntityBlock"` (not `"entity"`). Fixed from the OpenAPI.
2. **Trust-but-verify the append.** A later real run reported success but the link
   was absent — Capacities returned 200 for an append that didn't persist. Fixed:
   `capacities_append_entity` now reads the page back to confirm the link landed
   and retries (checking before each retry to avoid duplicates), failing honestly
   if it can't confirm.

## Key Patterns / Decisions

- Capacities token is **space-bound** (no spaceId in requests); `CAPACITIES_IO_SPACE_ID`
  is a name sanity-check against `GET /space`.
- "Atomic Note" type and "Inbox" page resolved **by title at runtime** (no hardcoded UUIDs).
- The link is a native `EntityBlock` reference appended at `position: end`.

## Next

Deferred to a future phase: multiple spaces + multiple API keys. Phase 4 is the
only phase of milestone v0.3 → milestone complete.
