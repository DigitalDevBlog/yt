# yt

CLI that takes a YouTube URL and outputs the video's title, transcript, duration
in minutes, and comments — as plain text or JSON — can summarize the video with
Claude, and can upload that summary to [Capacities](https://capacities.io) as an
atomic note. Written in Rust, converted from the original Go version at
[danielmiessler/yt](https://github.com/danielmiessler/yt).

## Install

```sh
cargo install --path .
```

## Usage

```
yt [OPTIONS] <URL>

  -d, --duration       Output only the duration (whole minutes)
  -t, --transcript     Output only the transcript
  -c, --comments       Output the comments on the video (JSON array)
      --title          Output only the video title
  -s, --summarize      Summarize the transcript with Claude, writing <title>.md
  -p, --prompt <TEXT>  Override the summary prompt (with --summarize)
  -u, --upload         Summarize, then upload to Capacities (implies --summarize)
  -l, --lang <CODE>    Language for the transcript [default: en]
```

With no flags, outputs a JSON object with all four fields:
`{"title": ..., "transcript": ..., "duration": ..., "comments": [...]}`.

The default, `--summarize`, and `--upload` modes show a stepped progress display
on stderr (stdout stays clean JSON). `--summarize` writes the summary to a
`<title>.md` file; `--upload` also creates a Capacities Atomic Note and links it
from your Inbox page.

## Configuration

The duration and comments modes use the YouTube Data API v3 and need an API
key, either as a `YOUTUBE_API_KEY` environment variable or stored in
`~/.config/yt/.env`:

```sh
mkdir -p ~/.config/yt
echo 'YOUTUBE_API_KEY="[Your API Key]"' >> ~/.config/yt/.env
```

The transcript mode needs no API key.

`--summarize` uses the **Claude Code CLI** (the `claude` binary on your `PATH`)
— your Claude subscription, no Anthropic API key or per-call cost. Set a default
summary prompt with `YT_SUMMARY_PROMPT` in the same `.env`.

`--upload` uses the **Capacities REST API** — set `CAPACITIES_IO_API_KEY` (a
Capacities API token) in the same `.env`; your space needs an "Atomic Note"
object type and an "Inbox" page.

See [MANUAL.md](MANUAL.md) for full documentation.

## License

[MIT](LICENSE), with attribution to the original Go version by Daniel Miessler
and contributors.
