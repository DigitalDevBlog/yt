# Project State

## Project Reference

See: .paul/PROJECT.md (updated 2026-07-15)

**Core value:** Anyone can pipe a YouTube video's transcript and metadata into
their LLM/AI workflows with a single command.
**Current focus:** v0.1 — Phase 3 (Progress UI), ready to plan

## Current Position

Milestone: v0.1 LLM Workflow Extensions
Phase: 3 of 3 (Progress UI) — Not started
Plan: Not started
Status: Ready to plan
Last activity: 2026-07-15 — Phase 2 complete, transitioned to Phase 3

Progress:
- Milestone: [███████░░░] 67% (2 of 3 phases)
- Phase 3: [░░░░░░░░░░] 0%

## Loop Position

Current loop state:
```
PLAN ──▶ APPLY ──▶ UNIFY
  ✓        ✓        ✓     [Loop complete — ready for next PLAN]
```

## Phase Results

- Phase 1 (Title Capture): title via combined videos call + `--title` flag.
  15 tests, live smoke test passed. → 01-01-SUMMARY.md
- Phase 2 (Claude Summarization): `-s`/`--summarize` → `claude -p` → `<title>.md`,
  `-p`/`--prompt`, `.env` prompt, sanitize_filename, stepped progress UI.
  20 tests, negative + live + user TTY runs passed. Deviation: stepped progress
  UI pulled forward from Phase 3 per user feedback. → 02-01-SUMMARY.md

## Git State

Uncommitted: src/main.rs + Cargo.toml (Phases 1–2) + .paul/ artifacts — commit
deferred per user preference (no auto-commit; no Claude co-author trailer)

## Accumulated Context

### Decisions

| Decision | Phase | Impact |
|----------|-------|--------|
| Summarize via `claude -p`, not the API | Phase 2 | Zero per-call cost; requires `claude` on PATH |
| Summary output is file-only → sanitized `<title>.md` in CWD | Phase 2 | Needs Phase 1 title; path printed to stderr |
| Summary prompt: `.env` default, override with `-p`/`--prompt` | Phase 2 | Flag `-s`/`--summarize` (proposed, supersedes `--cs`) |
| Progress UI renders to stderr | Phase 3 | Keeps stdout pipeable |
| Title via API `part=snippet` (combined with duration) | Phase 1 | Nearly free — one videos call returns duration + title |

### Deferred Issues

None yet.

### Blockers/Concerns

None yet.

## Session Continuity

Last session: 2026-07-15
Stopped at: Phase 2 complete, ready to plan Phase 3 (Progress UI)
Next action: /paul:plan for Phase 3
Resume file: .paul/ROADMAP.md

---
*STATE.md — Updated after every significant action*
