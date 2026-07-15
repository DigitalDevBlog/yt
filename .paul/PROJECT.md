# yt

## What This Is

A fast Rust CLI that takes a YouTube URL and outputs the video's transcript,
duration, and comments as plain text or JSON — designed to feed AI/LLM
pipelines. Converted from the original Go version at
[danielmiessler/yt](https://github.com/danielmiessler/yt). The next stage
extends it with Claude-powered summarization, video title capture, and a
step-based progress UI.

## Core Value

Anyone can pipe a YouTube video's transcript and metadata into their LLM/AI
workflows with a single command.

## Current State

| Attribute | Value |
|-----------|-------|
| Type | Application (CLI) |
| Version | 0.1.0 |
| Status | v0.1 complete (title, summarization, progress UI shipped) |
| Last Updated | 2026-07-15 |

## Requirements

### Core Features

- Extract video transcript (via InnerTube, no API key)
- Report video duration in minutes (via YouTube Data API)
- Fetch video comments (via YouTube Data API)
- Output as plain text or JSON

### Validated (Shipped)

- [x] Transcript extraction — 0.0.0
- [x] Duration reporting — 0.0.0
- [x] Comment fetching — 0.0.0
- [x] Text / JSON output modes — 0.0.0
- [x] Video title capture (default JSON + `--title`) — Phase 1
- [x] Claude summarization (`-s`/`--summarize` → `claude -p` → `<title>.md`;
      prompt from `.env`, override with `-p`; stepped progress UI) — Phase 2
- [x] Progress UI on the default run: stepped display with measured bars
      (transcript bytes, comment pagination) that stay put once finished — Phase 3

### Active (In Progress)

None.

### Planned (Next)

None — milestone v0.1 complete.

### Out of Scope

- Summarization via the Anthropic API — excluded to avoid per-call cost;
  use the subscription-backed `claude -p` CLI instead.

## Target Users

**Primary:** Developers and AI power-users building LLM/AI workflows.
- Comfortable on the command line
- Pipe tool output into other tools (LLMs, scripts, notes)
- Value speed and clean, parseable output

## Context

**Technical Context:**
- Transcript path uses YouTube's InnerTube Android client (no API key).
- Duration and comments use the official YouTube Data API v3 (requires an
  API key).
- Planned summarization shells out to the `claude` binary (Claude Code CLI),
  which must be on PATH.

## Constraints

### Technical Constraints

- Summarization depends on the `claude` binary being installed and on PATH.
- Progress output must go to stderr so stdout stays pipeable.
- Duration and comments require a valid YouTube Data API key.

### Business Constraints

- Keep cost of use at zero for summarization (subscription CLI, not API).

## Key Decisions

| Decision | Rationale | Date | Status |
|----------|-----------|------|--------|
| Summarize via `claude -p` subprocess, not the API | Avoids per-call cost; uses existing subscription | 2026-07-15 | Active |
| Transcript to `claude` via stdin (written from a thread) | Avoids stdin/stdout pipe deadlock on large transcripts | 2026-07-15 | Active |
| Use `indicatif` for progress; stepped UI pulled into Phase 2 | User wanted step visibility in the summarize flow | 2026-07-15 | Active |
| Progress UI renders to stderr | Keeps stdout clean for piping | 2026-07-15 | Active |
| Title via YouTube API `part=snippet` | Nearly free — extends a call already made for duration | 2026-07-15 | Active |

## Success Metrics

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Title present in text + JSON output | Yes | Yes | Achieved |
| Summarization works with zero API cost | Yes | Yes | Achieved |
| Progress visible without polluting stdout | Yes | Yes | Achieved |

## Tech Stack / Tools

| Layer | Technology | Notes |
|-------|------------|-------|
| Language | Rust | Existing codebase |
| HTTP | reqwest (blocking) | Existing |
| XML/JSON parsing | quick-xml, serde_json | Existing |
| Transcript source | YouTube InnerTube (Android client) | No API key |
| Metadata source | YouTube Data API v3 | Duration, comments, title |
| Summarization | `claude` CLI (`claude -p`) | Subprocess, subscription-backed |
| Progress UI | indicatif 0.17 | Stepped display (MultiProgress) on stderr |

---
*Created: 2026-07-15*
*Last updated: 2026-07-15 after Phase 3 (Progress UI) — milestone v0.1 complete*
