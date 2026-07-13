# yt — User Manual

`yt` is a command-line tool that takes a YouTube URL and gives you the video's
**transcript**, **duration**, and **comments** — as plain text or JSON. It is
built for pipelines: point it at a video, pipe the output into whatever
processes it next (e.g. `jq`, a summarizer, an LLM prompt).

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

## Usage

```
yt [OPTIONS] <URL>
```

| Option | Long form | Meaning |
|--------|-----------|---------|
| `-d` | `--duration` | Output only the duration, in whole minutes |
| `-t` | `--transcript` | Output only the transcript, as plain text |
| `-c` | `--comments` | Output the comments, as a JSON array |
| `-l <CODE>` | `--lang <CODE>` | Transcript language (default: `en`) |
| `-h` | `--help` | Show help |
| `-V` | `--version` | Show version |

Accepted URL forms: `youtube.com/watch?v=ID` (with or without `https://` and
`www.`, extra query parameters are fine), `youtu.be/ID`, and
`youtube.com/embed/ID`.

If more than one mode flag is given, precedence is
`--duration` > `--transcript` > `--comments`.

## The four output modes

### Default: everything as JSON

With no mode flags, `yt` prints one JSON object with all three fields
(this mode needs the API key):

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
  "transcript": "We're no strangers to love You know the rules and so do I ..."
}
```

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

Two failures are deliberately **not** fatal, so pipelines keep flowing:

- A missing transcript becomes the text `Transcript not available. (<reason>)`
  in the output.
- A failed comments fetch becomes an empty array plus a warning on stderr.

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
cargo test           # unit tests (URL parsing, duration math, track selection, XML parsing)
cargo run -- -t URL  # run without installing
cargo build --release
```

The whole tool lives in `src/main.rs`. Design notes are in
`docs/superpowers/specs/2026-07-13-yt-rust-port-design.md`.
