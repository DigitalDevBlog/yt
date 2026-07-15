# yt — User Manual

`yt` is a command-line tool that takes a YouTube URL and gives you the video's
**title**, **transcript**, **duration**, and **comments** — as plain text or
JSON — can **summarize** the video with Claude, and can **upload** that summary
to [Capacities](https://capacities.io) as an atomic note. It is built for
pipelines: point it at a video, pipe the output into whatever processes it next
(e.g. `jq`, a summarizer, an LLM prompt).

---

## Installation

You need a Rust toolchain (https://rustup.rs). From the project root:

```sh
cargo install --path .
```

This builds a release binary and puts `yt` on your `PATH`
(in `~/.cargo/bin`). Alternatively, run it in place with
`cargo run --quiet -- <args>`.

## Configuration

Two of the three features (duration and comments) use the **YouTube Data API
v3** and need an API key. The transcript feature needs **no key**.

1. Get an API key from the [Google Cloud Console](https://console.cloud.google.com/)
   (create a project, enable *YouTube Data API v3*, create an API key).
2. Store it where `yt` looks for it:

   ```sh
   mkdir -p ~/.config/yt
   echo 'YOUTUBE_API_KEY="[Your API Key]"' >> ~/.config/yt/.env
   ```

A `YOUTUBE_API_KEY` environment variable set in your shell also works and
takes effect even without the file.

### Summarization (`--summarize`)

The `--summarize` mode shells out to the **Claude Code CLI** (`claude`), so it
uses your existing Claude subscription — no Anthropic API key and no per-call
cost. You need the `claude` binary on your `PATH`
(https://claude.com/claude-code).

The summary prompt is resolved in this order:

1. `--prompt "<text>"` on the command line
2. `YT_SUMMARY_PROMPT` from `~/.config/yt/.env` (or the environment)
3. A built-in default (*"Summarize the key points and main takeaways of this
   video transcript as concise bullet points."*)

To set your own default prompt, add it alongside the API key:

```sh
echo 'YT_SUMMARY_PROMPT="Summarize this video as 5 bullet points."' >> ~/.config/yt/.env
```

### Upload to Capacities (`--upload`)

`--upload` (short: `-u`) sends the summary to [Capacities](https://capacities.io)
as an **Atomic Note** and links it from your **Inbox** page. It uses the
Capacities REST API (v2.0), so you need an API token (Settings → Capacities API
in the desktop app):

```sh
echo 'CAPACITIES_IO_API_KEY="[Your Capacities token]"' >> ~/.config/yt/.env
```

The token is **space-bound** — it maps to exactly one Capacities space. Two
optional settings:

```sh
# sanity check: warn if the token's space title isn't this
echo 'CAPACITIES_IO_SPACE_ID="Research & Study"' >> ~/.config/yt/.env
# the Page to link new notes from (default: "Inbox")
echo 'CAPACITIES_IO_INBOX_PAGE="Inbox"' >> ~/.config/yt/.env
```

Your space must already contain an object type named **"Atomic Note"** and a
Page named **"Inbox"** — both are resolved by name at runtime.

## Usage

```
yt [OPTIONS] <URL>
```

| Option | Long form | Meaning |
|--------|-----------|---------|
| `-d` | `--duration` | Output only the duration, in whole minutes |
| `-t` | `--transcript` | Output only the transcript, as plain text |
| `-c` | `--comments` | Output the comments, as a JSON array |
| | `--title` | Output only the video title |
| `-s` | `--summarize` | Summarize the transcript with Claude, writing `<title>.md` |
| `-p <TEXT>` | `--prompt <TEXT>` | Override the summary prompt (used with `--summarize`) |
| `-u` | `--upload` | Summarize, then upload the summary to Capacities (implies `--summarize`) |
| `-l <CODE>` | `--lang <CODE>` | Transcript language (default: `en`) |
| `-h` | `--help` | Show help |
| `-V` | `--version` | Show version |

Accepted URL forms: `youtube.com/watch?v=ID` (with or without `https://` and
`www.`, extra query parameters are fine), `youtu.be/ID`, and
`youtube.com/embed/ID`.

If more than one mode flag is given, precedence is
`--duration` > `--title` > `--transcript` > `--comments` >
`--summarize`/`--upload`.

## Output modes

### Default: everything as JSON

With no mode flags, `yt` prints one JSON object with the title, transcript,
duration, and comments (this mode needs the API key):

```sh
yt "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```

```json
{
  "comments": [
    "A top-level comment",
    "    - a reply to it",
    "Another top-level comment"
  ],
  "duration": 4,
  "title": "Rick Astley - Never Gonna Give You Up (Official Video) (4K Remaster)",
  "transcript": "We're no strangers to love You know the rules and so do I ..."
}
```

While it runs, a stepped **progress display** is shown on *stderr* (see
[Progress display](#progress-display)), so redirecting or piping stdout stays
clean:

```sh
yt "https://youtu.be/dQw4w9WgXcQ" | jq .title
```

Unlike the single-purpose `--comments` mode (one page of 100), the default mode
**paginates comments** across up to 5 pages (≈500), so its `comments` array is
usually larger.

### `--title`: the video title

```sh
yt --title "https://youtu.be/dQw4w9WgXcQ"
# Rick Astley - Never Gonna Give You Up (Official Video) (4K Remaster)
```

Prints just the title (needs the API key — it comes from the same Data API call
as the duration).

### `--transcript`: plain text, no API key needed

```sh
yt -t "https://youtu.be/dQw4w9WgXcQ"
```

Prints the spoken text as one line of plain text, HTML entities decoded.
Handy directly in a pipeline:

```sh
yt -t "https://youtu.be/dQw4w9WgXcQ" | llm "summarize this transcript"
```

If the video has no captions (or YouTube refuses to serve them), the tool
still exits successfully and prints
`Transcript not available. (<reason>)` instead.

### `--duration`: minutes as a bare integer

```sh
yt -d "https://youtu.be/dQw4w9WgXcQ"
# 4
```

The value is the video length rounded to the nearest minute (a 2 m 30 s video
reports 3; a 29-second video reports 0).

### `--comments`: JSON array

```sh
yt -c "https://youtu.be/dQw4w9WgXcQ"
```

Prints up to the first 100 comment threads as a JSON array of strings.
Top-level comments appear as-is; replies follow their parent, prefixed with
four spaces and `- `. If comments are disabled on the video or the fetch
fails, the reason is printed to stderr and the array is empty.

(This single-purpose mode fetches one page. The default combined mode paginates
up to ≈500 — see [Default](#default-everything-as-json).)

### `--summarize`: transcript → Claude → `<title>.md`

```sh
yt --summarize "https://youtu.be/dQw4w9WgXcQ"
# (writes) Rick Astley - Never Gonna Give You Up (Official Video) (4K Remaster).md
```

`--summarize` (short: `-s`) runs the whole pipeline: it downloads the
transcript, pipes it to `claude -p "<prompt>"`, and writes the returned summary
to a Markdown file **in the current directory**, named after the video title.
It needs the API key (for the title) and the `claude` CLI (see
[Summarization](#summarization---summarize)).

- The output is **file-only** — nothing is printed to stdout. The path written
  is reported on stderr (`Wrote <file>.md`).
- The filename is sanitized (illegal characters removed, length-capped); if the
  title is empty it falls back to the video ID.
- Override the prompt for one run with `-p` / `--prompt`:

  ```sh
  yt -s -p "List the 3 most important takeaways" "https://youtu.be/dQw4w9WgXcQ"
  ```

### `--upload`: summary → Capacities Atomic Note

```sh
yt --upload "https://youtu.be/dQw4w9WgXcQ"
# Uploaded to Capacities: <note-id> (linked in Inbox)
```

`--upload` (short: `-u`) does everything `--summarize` does — including writing
the local `<title>.md` — and then:

1. creates a new **Atomic Note** in your Capacities space, with the video title
   as the note title and the summary as its markdown body, and
2. appends a **link to that note** at the bottom of your **Inbox** page.

It needs `CAPACITIES_IO_API_KEY` configured (see
[Upload to Capacities](#upload-to-capacities---upload)). The upload runs as a
fourth progress step and **fails gracefully**: if the token is missing or the
API errors, the step shows `✗` with a message on stderr, but your `<title>.md`
is still written and the run does not abort.

## Progress display

The `--summarize`, `--upload`, and default combined modes show a live,
multi-step progress display on **stderr** while they work — one line per step,
with a `◦` bullet and a `|` connector (`--upload` adds a final
"Uploading to Capacities" step):

```
◦ Fetching video details [0s] ✓
|
◦ Downloading transcript 63.2 KiB ✓
|
◦ Fetching comments [====================] 500/500 ✓
```

Steps that have a known total (comment pagination, and the transcript download
when the server reports a length) show a filled bar; others show an elapsed
timer or a live byte counter. Finished steps stay put, keeping their bar with a
`✓`. Because it's all on stderr, it never contaminates the JSON/summary on
stdout, and it automatically disappears when output is piped or redirected
(non-terminal).

## Choosing the transcript language

`--lang` takes a BCP-47 language code as YouTube uses them: `en`, `nl`, `de-DE`,
`pt-BR`, ...

```sh
yt -t -l nl "https://youtu.be/dQw4w9WgXcQ"
```

Track selection order:

1. A **human-made** caption track in the requested language
2. Any track in the requested language (auto-generated speech recognition)
3. Any human-made track in another language
4. The first available track, whatever it is

So you always get *a* transcript if one exists; the requested language is a
preference, not a requirement. Note that the code must match exactly:
if a video's track is labeled `de-DE`, `-l de` will not match it (you'll get
the fallback instead).

## Errors and exit codes

`yt` exits `0` on success and non-zero with a message on stderr when:

- **No URL / unrecognized flags** — usage error from the CLI parser (exit 2).
- **`Invalid YouTube URL`** — no 11-character video ID could be extracted.
- **`YOUTUBE_API_KEY not set ...`** — you used a mode that needs the API
  key (duration, comments, or the default combined mode) without configuring
  one. The message includes the command to fix it.
- **`error getting video details` / `video not found`** — the Data API call
  failed (bad key, quota exceeded) or the video ID doesn't exist.
- **``the `claude` CLI is required for --summarize ...``** — you used
  `--summarize` without the `claude` binary on your `PATH`. No file is written.
- **`cannot summarize: transcript unavailable`** — `--summarize` needs a
  transcript; the video has none, so nothing is sent to Claude.

Two failures are deliberately **not** fatal, so pipelines keep flowing:

- A missing transcript becomes the text `Transcript not available. (<reason>)`
  in the output.
- A failed comments fetch becomes an empty array plus a warning on stderr.
- A failed Capacities upload (`--upload`) prints `Capacities upload failed: ...`
  on stderr but still writes the local `<title>.md`, and the run exits `0`.

## Troubleshooting

**"Transcript not available. (no caption tracks found)"**
The video genuinely has no captions, or YouTube did not expose them to the
player API. Try another video to confirm the tool itself works.

**"Transcript not available. (caption track returned an empty response)"**
YouTube served the track list but refused the track content. This usually
means YouTube changed its access rules again (see note below); retrying later
sometimes helps.

**Duration or comments fail with HTTP 403**
Your API key is invalid, restricted to the wrong APIs, or you have exhausted
the daily YouTube Data API quota (10,000 units by default; a comments call
costs 1 unit, so quota problems usually come from elsewhere).

**The transcript is in the wrong language**
The requested language has no track and you received a fallback. Check
YouTube's caption menu on the video to see which languages exist, and match
the code exactly (`de-DE` vs `de`).

### A note on how transcripts are fetched

There is no official API for transcripts. `yt` asks YouTube's internal
*InnerTube* player endpoint (the same one the mobile apps use) for the caption
track list, then downloads the track XML. This is the part of the tool most
likely to break if YouTube changes its internals — the older technique of
scraping caption URLs out of the watch-page HTML (used by the original Go
version of this tool) already stopped working this way. If transcripts
suddenly fail for every video, the fetch strategy likely needs updating again.

## Development

```sh
cargo test           # unit tests (URL parsing, duration math, track selection, XML parsing, filename sanitization)
cargo run -- -t URL  # run without installing
cargo build --release
```

The whole tool lives in `src/main.rs`. Design notes are in
`docs/superpowers/specs/2026-07-13-yt-rust-port-design.md`.
