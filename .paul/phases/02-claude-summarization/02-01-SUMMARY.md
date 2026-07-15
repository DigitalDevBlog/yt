---
phase: 02-claude-summarization
plan: 01
completed: 2026-07-15
duration: ~35min
---

# Phase 2 Plan 01: Claude Summarization Summary

**Added `-s`/`--summarize`: fetches the transcript, pipes it to `claude -p`, and
writes the summary to a sanitized `<title>.md` — with a stepped progress display.**

## Objective

Turn `yt` into a transcript-to-summary pipeline using the subscription-backed
`claude` CLI (zero API cost), with the prompt sourced from `.env` and
overridable per run.

## What Was Built

| File | Change |
|------|--------|
| Cargo.toml | Added `indicatif = "0.17"` |
| src/main.rs | `-s`/`--summarize` + `-p`/`--prompt` flags; `resolve_prompt` (CLI → `YT_SUMMARY_PROMPT` → built-in default); `sanitize_filename`; `run_claude` (subprocess, stdin-from-thread, stderr captured); `run_summary` pipeline; `Steps` stepped-progress UI (◦ bullet, \| connector, spinner→✓/✗) |

## Acceptance Criteria Results

| AC | Description | Status |
|----|-------------|--------|
| AC-1 | Summarize writes `<title>.md`, stdout empty, path on stderr | Pass (live run) |
| AC-2 | Prompt resolves CLI → env → default; `-p` overrides | Pass (default path live; override code-verified) |
| AC-3 | Filename sanitization + video-id fallback | Pass (5 unit tests) |
| AC-4 | Missing `claude` → clean error, no partial file | Pass (negative test) |
| AC-5 | Stepped progress display (◦ / \| / spinner→✓) | Pass (user-verified in TTY) |

## Verification Results

- `cargo build` — clean; `cargo test` — 20 passed (15 prior + 5 new)
- Negative test (claude off PATH): clean error, exit 1, no file
- Live run: wrote `Rick Astley - ... .md` with a real summary, stdout empty
- User TTY run (`-s` on a real video): all 3 steps animated to ✓

## Deviations

- **Spec change (mid-APPLY, per user feedback):** the multi-step progress UI was
  originally deferred to Phase 3 ("single spinner only"). Pulled the *stepped*
  display into the summarize flow now (AC-5 added). Generalizing it to other
  modes and adding MEASURED bars remains Phase 3.

## Key Patterns / Decisions

- Transcript → `claude` via stdin (written from a thread to avoid pipe deadlock);
  prompt passed as `-p` arg. claude stderr captured and surfaced only on failure
  (keeps the progress display clean).
- Progress renders to stderr and auto-hides on non-TTY (piped output stays clean).

## Next Phase

Phase 3 — Progress UI: generalize the `Steps` display across modes and add
MEASURED progress (comment pagination, transcript bytes).
