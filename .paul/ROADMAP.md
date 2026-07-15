# Roadmap: yt

## Overview

`yt` is a working Rust CLI that outputs a YouTube video's transcript, duration,
and comments for LLM/AI workflows. This milestone extends it with video title
capture, Claude-powered summarization (via the subscription `claude -p` CLI),
and a step-based progress UI — turning a fetch tool into a richer transcript-to-
summary pipeline.

## Current Milestone

**v0.1 LLM Workflow Extensions** (v0.1.0)
Status: In progress
Phases: 2 of 3 complete

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with [INSERTED])

Phases execute in numeric order: 1 → 2 → 3

| Phase | Name | Plans | Status | Completed |
|-------|------|-------|--------|-----------|
| 1 | Title Capture | 1 | ✅ Complete | 2026-07-15 |
| 2 | Claude Summarization | 1 | ✅ Complete | 2026-07-15 |
| 3 | Progress UI | TBD | Not started | - |

## Phase Details

### Phase 1: Title Capture

**Goal:** Surface the YouTube video title in both text and JSON output.
**Depends on:** Nothing (first phase)
**Research:** Unlikely (extends an API call already made)

**Scope:**
- Add `snippet` to the `videos` API call (`part=contentDetails,snippet`)
- Parse `snippet.title` from the response
- Include the title in text output and the JSON structure

### Phase 2: Claude Summarization

**Goal:** A `-s` / `--summarize` flag that runs the full pipeline — fetch
transcript → pipe to `claude -p` → write the summary to a `<title>.md` file in
the current directory — at zero API cost.
**Depends on:** Phase 1 (needs the captured title to name the output file)
**Research:** Likely (subprocess invocation, prompt design, error handling when
`claude` is missing)
**Research topics:** `claude -p` invocation/flags, streaming vs. buffered output,
handling absent binary and long transcripts, filename sanitization.

**Scope:**
- `-s` / `--summarize` flag (proposed; confirm at plan time — supersedes the
  originally-sketched `--cs`)
- `-p` / `--prompt "<text>"` to override the summary prompt for one run
- Default summary prompt read from `~/.config/yt/.env` (e.g. `YT_SUMMARY_PROMPT`)
- Shell out to `claude -p "<prompt>"` with the transcript on stdin; capture the
  summary from stdout
- Sanitize the video title into a safe filename → write summary to
  `<title>.md` in the current working directory
- File-only output (nothing to stdout); print the written path to stderr
- Spinner on stderr while `claude` runs (rich progress UI deferred to Phase 3)
- Clear error if the `claude` binary is not on PATH

**Decisions (locked during planning):**
- Summarize via `claude -p` subprocess, not the Anthropic API (cost)
- Output is file-only, written to CWD, filename derived from sanitized title
- Prompt: `.env` default, overridable with `-p`/`--prompt`

### Phase 3: Progress UI

**Goal:** Generalize the stepped progress display beyond the summarize flow and
add MEASURED progress bars for steps we control.
**Depends on:** Phase 2 (reuses the `Steps` stepped display built there)
**Research:** Likely (measuring pagination/byte progress with indicatif)
**Research topics:** measuring comment pagination and transcript byte download,
keeping stdout clean.

**Note:** The stepped display itself (◦ bullet, | connector, spinner→✓/✗ on
stderr, via `indicatif`) already shipped in Phase 2 for the summarize flow.

**Scope:**
- Reuse the `Steps` helper for other multi-step modes (e.g. the default
  transcript+duration+comments run)
- Add MEASURED progress bars for steps we control: comment pagination (pages
  fetched / target) and transcript download (bytes when content-length known)
- Keep spinners for non-measurable steps; all progress to stderr

---
*Roadmap created: 2026-07-15*
*Last updated: 2026-07-15*
