# Project State

## Project Reference

See: .paul/PROJECT.md (updated 2026-07-15)

**Core value:** Anyone can pipe a YouTube video's transcript and metadata into
their LLM/AI workflows with a single command.
**Current focus:** v0.3 milestone COMPLETE — Capacities upload shipped (0.3.0)

## Current Position

Milestone: v0.3 Integrations — ✅ Complete
Phase: 4 (Upload to Capacities) — Complete
Plan: 04-01 complete
Status: Milestone complete — ready for release or next milestone
Last activity: 2026-07-15 — Phase 4 complete; v0.3 milestone done (0.3.0)

Progress:
- v0.1: [██████████] 100% (0.2.0) · v0.3: [██████████] 100% (0.3.0)

## Loop Position

Current loop state:
```
PLAN ──▶ APPLY ──▶ UNIFY
  ✓        ✓        ✓     [Loop complete — v0.3 milestone done]
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
- Phase 4 (Upload to Capacities): `-u`/`--upload` → Atomic Note + link at the
  bottom of the Inbox page (Capacities v2.0 REST). 20 tests, live end-to-end
  verified. Two live-caught deviations: append body shape (position object +
  EntityBlock) and trust-but-verify read-back on append. Shipped as 0.3.0.
  → 04-01-SUMMARY.md

## Git State

Last commit: 367058b (through Phase 3 + version 0.2.0). UNCOMMITTED: Phase 4
(Capacities upload) — src/main.rs, MANUAL.md, README.md, Cargo.toml/lock (0.3.0),
.paul/ artifacts. Installed `yt` is 0.3.0 (current). No Claude co-author trailer.

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
Stopped at: v0.3 milestone complete (Phase 4 — Capacities upload — shipped as 0.3.0)
Next action: Commit/push Phase 4, then release (tag v0.3.0) or next milestone
Resume file: .paul/ROADMAP.md

---
*STATE.md — Updated after every significant action*
