# yt

CLI that takes a YouTube URL and outputs the video's transcript, duration in
minutes, and comments — as plain text or JSON. Written in Rust, converted from
the original Go version at [danielmiessler/yt](https://github.com/danielmiessler/yt).

## Install

```sh
cargo install --path .
```

## Usage

```
yt [OPTIONS] <URL>

  -d, --duration     Output only the duration (whole minutes)
  -t, --transcript   Output only the transcript
  -c, --comments     Output the comments on the video (JSON array)
  -l, --lang <CODE>  Language for the transcript [default: en]
```

With no flags, outputs a JSON object with all three:
`{"transcript": ..., "duration": ..., "comments": [...]}`.

## Configuration

The duration and comments modes use the YouTube Data API v3 and need an API
key in `~/.config/fabric/.env`:

```sh
echo 'YOUTUBE_API_KEY="[Your API Key]"' >> ~/.config/fabric/.env
```

The transcript mode needs no API key.
