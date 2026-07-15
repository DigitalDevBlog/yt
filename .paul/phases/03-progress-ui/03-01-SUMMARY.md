---
phase: 03-progress-ui
plan: 01
completed: 2026-07-15
duration: ~50min (incl. UX iterations)
---

# Phase 3 Plan 01: Progress UI Summary

**Brought the stepped progress display to the default run and made every step
show a persistent indicatif indicator (byte/count bars + elapsed) that stays put
once finished.**

## Objective

Generalize the `Steps` display beyond summarize and add measured progress for the
steps we control (transcript bytes, comment pagination), keeping stdout clean.

## What Was Built

| File | Change |
|------|--------|
| src/main.rs | `Steps` gained per-step `StepKind` tracking + `set_bytes`/`set_bytes_counter`/`set_count`; `done()` freezes the finished bar/counter in place with a green ✓ (no longer collapses); active steps show `[{elapsed}]`; `fetch_transcript(Option<(&Steps,usize)>)` streams via `wrap_read` (byte bar when Content-Length known, live byte counter otherwise); `get_comments_paginated` (up to `MAX_COMMENT_PAGES = 5`) with count bar from `statistics.commentCount`; default run + summarize flow wired through `Steps` |

## Acceptance Criteria Results

| AC | Description | Status |
|----|-------------|--------|
| AC-1 | Default run shows stepped progress; JSON still on stdout | Pass (user-verified TTY) |
| AC-2 | Transcript byte bar / live counter fallback | Pass (counter shown: caption feed sends no Content-Length) |
| AC-3 | Comment pagination + count bar (capped at 500) | Pass (561→500-cap bar; 561 comments fetched) |
| AC-4 | stdout clean; -d/-t/-c/--title/-s unchanged | Pass (verified headless) |

## Verification Results

- `cargo build` clean; `cargo test` — 20 passed
- Default run: clean JSON on stdout (`jq` parses), 561 comments (multi-page)
- `-c`/`-d`/`-t`/`--title` unchanged; `-s` summarize flow now also shows byte
  counter + elapsed
- User TTY run: finished steps stay put — `563 KiB ✓`, `[====] 500/500 ✓`, `[0s] ✓`

## Deviations

- **UX refinements beyond the original plan (per user feedback during APPLY):**
  1. "Visible bar on every step" — added `[{elapsed}]` to active/unmeasurable
     steps and a live byte counter for transcript when Content-Length is absent.
  2. "Bars stay put once finished" — `done()` now freezes each step keeping its
     bar/counter (green ✓) instead of collapsing to a plain checkmark line.
  3. Wired the summarize flow's transcript step to the byte counter too (was a
     plain spinner) for consistency.
- Reinstalled `yt` (`cargo install --path .`) mid-phase after discovering the
  installed binary was the stale Phase 2 build.

## Key Patterns / Decisions

- Per-step `Cell<StepKind>` lets `done()` pick the correct "finished" style.
- Determinate bars only where a total exists (byte length, commentCount);
  everything else degrades to an elapsed timer or byte counter — never a bare
  spinner.

## Next Phase

None — this is the last phase of milestone v0.1.
