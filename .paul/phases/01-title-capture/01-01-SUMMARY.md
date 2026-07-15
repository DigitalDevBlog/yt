---
phase: 01-title-capture
plan: 01
completed: 2026-07-15
duration: ~10min
---

# Phase 1 Plan 01: Title Capture Summary

**Captured the YouTube video title via a single combined `videos` API call and
surfaced it in the default JSON output and through a new `--title` flag.**

## Objective

Fetch the video title from the YouTube Data API (folded into the existing
duration call) and expose it in output, laying the foundation for the Phase 2
`--summarize` feature which names its `<title>.md` output file from it.

## What Was Built

| File | Change |
|------|--------|
| src/main.rs | `get_duration_minutes` → `get_video_details` returning `{duration_minutes, title}`; `part=contentDetails,snippet`; new `Snippet` struct (`Default` + `serde(default)`); `--title` flag; `"title"` field added to default JSON output |

## Acceptance Criteria Results

| AC | Description | Status |
|----|-------------|--------|
| AC-1 | Title present in default JSON output | Pass |
| AC-2 | `--title` prints only the title | Pass |
| AC-3 | Single videos request (`part=contentDetails,snippet`) | Pass |
| AC-4 | Missing snippet → empty title, no panic | Pass (by construction: `Default` + `serde(default)`) |

## Verification Results

- `cargo build` — clean, no new warnings
- `cargo test` — 15 passed
- `grep` — 1 videos endpoint call, `part=contentDetails,snippet`
- Live smoke test (`dQw4w9WgXcQ`):
  - `yt --title` → `Rick Astley - Never Gonna Give You Up (Official Video) (4K Remaster)`
  - `yt <url>` JSON → keys `[comments, duration, title, transcript]`, `title` populated, `duration: 4`

## Deviations

None. Executed exactly as planned.

## Key Patterns / Decisions

- Duration and title come from one response (`items.into_iter().next()` moves the
  item so the title is taken without cloning). Every video-details branch now
  fetches once per invocation.
- Default JSON output shape changed (gained `"title"`). Acceptable pre-release
  (v0.0.0).

## Next Phase

Phase 2 — Claude Summarization: `-s`/`--summarize` runs transcript → `claude -p`
→ sanitized `<title>.md` in CWD; `.env` default prompt overridable with
`-p`/`--prompt`. Depends on this phase's title capture.

---
*Completed: 2026-07-15*
