# Project State

## Project Reference

See: .paul/PROJECT.md (updated 2026-07-15)

**Core value:** Anyone can pipe a YouTube video's transcript and metadata into
their LLM/AI workflows with a single command.
**Current focus:** v0.1 milestone COMPLETE — all 3 phases shipped

## Current Position

Milestone: v0.1 LLM Workflow Extensions — ✅ Complete
Phase: 3 of 3 (Progress UI) — Complete
Plan: 03-01 complete
Status: Milestone complete — ready for next milestone or release
Last activity: 2026-07-15 — Phase 3 complete; v0.1 milestone done

Progress:
- Milestone: [██████████] 100% (3 of 3 phases)

## Loop Position

Current loop state:
```
PLAN ──▶ APPLY ──▶ UNIFY
  ✓        ✓        ✓     [Loop complete — milestone v0.1 done]
```

## Phase Results

- Phase 1 (Title Capture): title via combined videos call + `--title` flag.
  15 tests, live smoke test passed. → 01-01-SUMMARY.md
- Phase 2 (Claude Summarization): `-s`/`--summarize` → `claude -p` → `<title>.md`,
  `-p`/`--prompt`, `.env` prompt, sanitize_filename, stepped progress UI.
  20 tests, negative + live + user TTY runs passed. Deviation: stepped progress
  UI pulled forward from Phase 3 per user feedback. → 02-01-SUMMARY.md
- Phase 3 (Progress UI): stepped display on the default run + measured bars
  (transcript bytes, comment pagination) that stay put once finished; wired into
  summarize too. Deviations: "bar on every step" + "finished bars stay put" UX
  refinements per user feedback. → 03-01-SUMMARY.md

## Git State

Last commit: 768536e (Phases 1–2). UNCOMMITTED: Phase 3 changes to src/main.rs
(progress UI) + .paul/ artifacts. Installed `yt` is up to date with Phase 3.
Commit deferred per user preference (no auto-commit; no Claude co-author trailer).

## Accumulated Context

### Decisions

| Decision | Phase | Impact |
|----------|-------|--------|
| Summarize via `claude -p`, not the API | Phase 2 | Zero per-call cost; requires `claude` on PATH |
| Summary output is file-only → sanitized `<title>.md` in CWD | Phase 2 | Needs Phase 1 title; path printed to stderr |
| Summary prompt: `.env` default, override with `-p`/`--prompt` | Phase 2 | Flag `-s`/`--summarize` (proposed, supersedes `--cs`) |
| Progress UI renders to stderr | Phase 3 | Keeps stdout pipeable |
| Measured bars only where a total exists; else elapsed/byte counter | Phase 3 | No bare spinners; finished bars stay put |
| Comment pagination capped at 5 pages (500) | Phase 3 | Bounds runtime; bar length = min(commentCount, 500) |
| Title via API `part=snippet` (combined with duration) | Phase 1 | Nearly free — one videos call returns duration + title |

### Deferred Issues

None yet.

### Blockers/Concerns

None yet.

## Session Continuity

Last session: 2026-07-15
Stopped at: v0.1 milestone complete (Phases 1–3 shipped, verified)
Next action: Commit/push Phase 3, then start a new milestone (/paul:milestone) or release
Resume file: .paul/ROADMAP.md

---
*STATE.md — Updated after every significant action*
