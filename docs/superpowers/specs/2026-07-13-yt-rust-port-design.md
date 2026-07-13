# yt — Rust Port Design

**Date:** 2026-07-13
**Status:** Approved

## Goal

Convert the existing Go CLI (`yt.go`) into a Rust project that replaces it in this
repo. Same CLI interface and output shapes, with three deliberate improvements
(see "Improvements over the Go version").

`yt` takes a YouTube URL and outputs the video's transcript, duration in minutes,
and optionally its comments — as plain text or JSON — for use in pipelines
(e.g. fabric).

## Decisions

- **Replace, don't coexist:** `yt.go`, `go.mod`, `go.sum` are deleted; the repo
  becomes a Cargo binary crate named `yt` with a single `src/main.rs`.
- **Improved port, not 1:1:** keep the interface, fix known quirks.
- **Blocking HTTP, direct REST:** call YouTube Data API v3 endpoints directly
  with `reqwest::blocking` and an API key. No async runtime, no generated
  Google client crate.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` (derive) | CLI parsing |
| `reqwest` (blocking, json, rustls-tls) | HTTP |
| `serde`, `serde_json` | JSON in/out |
| `regex` | video-ID, captionTracks, ISO-8601 duration |
| `dotenvy` | load `~/.config/fabric/.env` |
| `quick-xml` | transcript XML parsing |
| `html-escape` | decode HTML entities in transcript text |
| `anyhow` | error handling |

## CLI

```
yt [OPTIONS] <URL>

  -d, --duration     Output only the duration (minutes, integer)
  -t, --transcript   Output only the transcript
  -c, --comments     Output the comments on the video (JSON array)
  -l, --lang <CODE>  Language for the transcript [default: en]
```

Go's `flag` package accepts `--duration` as well as `-duration`, so existing
double-dash invocations keep working. Short aliases are new.

## Behavior

1. **Config:** load `~/.config/fabric/.env` via `dotenvy`; read `YOUTUBE_API_KEY`.
   Missing file or key → error with the same remediation hint as the Go version
   (`echo YOUTUBE_API_KEY="[Your API Key]" >> ~/.config/fabric/.env`).
2. **Video ID:** extracted with the same regex as the Go version
   (`youtube.com/watch?v=`, `youtu.be/`, embed/v/e forms; 11-char ID).
   Invalid URL → error.
3. **Duration:** `GET https://www.googleapis.com/youtube/v3/videos?part=contentDetails&id={id}&key={key}`,
   parse `items[0].contentDetails.duration` (ISO-8601 like `PT1H2M30S`) with a
   regex, output whole minutes.
4. **Transcript:** fetch `https://www.youtube.com/watch?v={id}`, regex out
   `"captionTracks":(\[.*?\])`, deserialize to a list of
   `{baseUrl, languageCode}`. Pick the track whose `languageCode` matches
   `--lang`; fall back to the first track. Fetch the track XML and concatenate
   the `<text>` node contents, space-separated, entity-decoded.
   Any transcript failure is not fatal: the output contains
   `Transcript not available. (<reason>)` instead, matching Go.
5. **Comments:** fetched when `--comments` is set *or* in the default combined
   mode. (In Go, only `--comments` triggered the fetch, so the combined JSON
   always had `"comments": null` — improvement #4 below.)
   `GET https://www.googleapis.com/youtube/v3/commentThreads?part=snippet,replies&videoId={id}&textFormat=plainText&maxResults=100&key={key}`.
   Top-level comment text, followed by replies prefixed with four spaces and
   `- `, exactly like the Go version. First page only (max 100 threads), also
   like the Go version. Fetch failure → empty list with a warning on stderr.
6. **Output selection (same precedence as Go):**
   - `--duration` → prints the integer minutes
   - else `--transcript` → prints the transcript text
   - else `--comments` → prints the comments as a pretty JSON array
   - else → pretty JSON object `{"transcript": ..., "duration": ..., "comments": [...]}`

## Improvements over the Go version

1. `--lang` actually selects the caption track (Go parsed it but never used it).
2. Duration math is correct: total seconds rounded to minutes. (Go computed
   `hours*60 + minutes + seconds/60` with integer division, silently dropping
   seconds.)
3. Transcript text is HTML-entity-decoded (Go printed raw `&amp;#39;`-style
   entities).
4. The default combined JSON output actually contains the comments (Go never
   fetched them in that mode, so the field was always `null`).

Errors use `anyhow` with context and exit non-zero via `Result` from `main`,
replacing Go's `log.Fatal` calls.

## Structure

Single `src/main.rs` (~250 lines), mirroring the original's shape:

- `get_video_id(url) -> Option<String>` — pure, regex
- `parse_duration_minutes(iso: &str) -> Result<u64>` — pure, regex
- `extract_caption_tracks(html) -> Result<Vec<CaptionTrack>>` — pure, regex + serde
- `pick_caption_track(tracks, lang) -> Option<&CaptionTrack>` — pure
- `transcript_xml_to_text(xml) -> Result<String>` — pure, quick-xml + entity decode
- `get_transcript(client, video_id, lang) -> Result<String>` — HTTP + the above
- `get_duration(client, api_key, video_id) -> Result<u64>` — HTTP
- `get_comments(client, api_key, video_id) -> Vec<String>` — HTTP, non-fatal
- `main()` — clap, config, orchestration, output

## Testing

Unit tests (in `main.rs`) for every pure function: video-ID regex against the
URL forms above plus non-matches; ISO-8601 duration parsing incl. rounding;
captionTracks extraction and language selection incl. fallback; transcript XML
→ text incl. entity decoding.

Network paths are verified manually: `cargo run -- <url>` against a real video,
checking all four output modes.
