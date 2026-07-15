use anyhow::{Context, Result, anyhow};
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use quick_xml::events::Event;
use regex::Regex;
use serde::Deserialize;
use serde_json::json;
use std::cell::Cell;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Get the transcript, duration, and comments of a YouTube video.
#[derive(Parser)]
#[command(name = "yt", version)]
struct Options {
    /// Output only the duration (whole minutes)
    #[arg(short, long)]
    duration: bool,

    /// Output only the transcript
    #[arg(short, long)]
    transcript: bool,

    /// Output the comments on the video
    #[arg(short, long)]
    comments: bool,

    /// Output only the video title
    #[arg(long)]
    title: bool,

    /// Summarize the transcript with `claude -p` and write it to <title>.md
    #[arg(short, long)]
    summarize: bool,

    /// Override the summary prompt (defaults to YT_SUMMARY_PROMPT or a built-in)
    #[arg(short = 'p', long)]
    prompt: Option<String>,

    /// Language for the transcript
    #[arg(short, long, default_value = "en")]
    lang: String,

    /// YouTube video URL
    url: String,
}

fn get_video_id(url: &str) -> Option<String> {
    let re = Regex::new(
        r"(?:https?://)?(?:www\.)?(?:youtube\.com/(?:[^/\n\s]+/\S+/|(?:v|e(?:mbed)?)/|\S*?[?&]v=)|youtu\.be/)([a-zA-Z0-9_-]{11})",
    )
    .unwrap();
    re.captures(url).map(|caps| caps[1].to_string())
}

fn parse_duration_minutes(iso: &str) -> Result<u64> {
    let re = Regex::new(r"(?i)PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?").unwrap();
    let caps = re
        .captures(iso)
        .ok_or_else(|| anyhow!("invalid duration string: {iso}"))?;
    let group = |i| {
        caps.get(i)
            .map_or(0u64, |m| m.as_str().parse().unwrap_or(0))
    };
    let total_seconds = group(1) * 3600 + group(2) * 60 + group(3);
    Ok((total_seconds + 30) / 60)
}

// The watch-page "captionTracks" URLs YouTube hands to plain scrapers stopped
// working (200 with an empty body, pending a proof-of-origin token). The
// InnerTube player endpoint queried as the Android client still returns
// caption URLs that serve actual XML.
const INNERTUBE_PLAYER_URL: &str = "https://www.youtube.com/youtubei/v1/player";
const ANDROID_CLIENT_VERSION: &str = "20.10.38";
const ANDROID_USER_AGENT: &str =
    "com.google.android.youtube/20.10.38 (Linux; U; Android 11) gzip";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CaptionTrack {
    base_url: String,
    #[serde(default)]
    language_code: String,
    /// "asr" for auto-generated captions
    #[serde(default)]
    kind: String,
}

fn get_caption_tracks(
    client: &reqwest::blocking::Client,
    video_id: &str,
) -> Result<Vec<CaptionTrack>> {
    let body = json!({
        "context": {
            "client": {
                "clientName": "ANDROID",
                "clientVersion": ANDROID_CLIENT_VERSION,
                "androidSdkVersion": 30,
                "hl": "en",
            }
        },
        "videoId": video_id,
    });
    let response: serde_json::Value = client
        .post(INNERTUBE_PLAYER_URL)
        .header(reqwest::header::USER_AGENT, ANDROID_USER_AGENT)
        .json(&body)
        .send()?
        .error_for_status()?
        .json()?;
    let tracks = response
        .pointer("/captions/playerCaptionsTracklistRenderer/captionTracks")
        .cloned()
        .ok_or_else(|| anyhow!("no caption tracks found"))?;
    serde_json::from_value(tracks).context("failed to parse captionTracks JSON")
}

fn pick_caption_track<'a>(tracks: &'a [CaptionTrack], lang: &str) -> Option<&'a CaptionTrack> {
    tracks
        .iter()
        .find(|t| t.language_code == lang && t.kind != "asr")
        .or_else(|| tracks.iter().find(|t| t.language_code == lang))
        .or_else(|| tracks.iter().find(|t| t.kind != "asr"))
        .or_else(|| tracks.first())
}

/// Extracts the spoken text from caption XML. Handles both the legacy
/// `<transcript><text>` format and the current `<timedtext format="3"><p>` one.
fn transcript_xml_to_text(xml: &str) -> Result<String> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut depth_in_cue = 0u32;
    let mut parts: Vec<String> = Vec::new();
    let is_cue = |name: &[u8]| name == b"text" || name == b"p";
    loop {
        match reader.read_event().context("failed to parse transcript XML")? {
            Event::Start(e) if is_cue(e.name().as_ref()) => depth_in_cue += 1,
            Event::End(e) if is_cue(e.name().as_ref()) => depth_in_cue = depth_in_cue.saturating_sub(1),
            Event::Text(e) if depth_in_cue > 0 => {
                let text = e.unescape().context("failed to unescape transcript text")?;
                let decoded = html_escape::decode_html_entities(text.as_ref());
                let normalized = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
                if !normalized.is_empty() {
                    parts.push(normalized);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(parts.join(" "))
}

/// Download and decode the transcript. When `progress` is Some and the caption
/// response reports a Content-Length, the byte download advances a measured bar;
/// otherwise it reads normally (leaving any spinner running).
fn fetch_transcript(
    client: &reqwest::blocking::Client,
    video_id: &str,
    lang: &str,
    progress: Option<(&Steps, usize)>,
) -> Result<String> {
    let tracks = get_caption_tracks(client, video_id)?;
    let track = pick_caption_track(&tracks, lang).ok_or_else(|| anyhow!("no caption tracks found"))?;
    let response = client
        .get(&track.base_url)
        .header(reqwest::header::USER_AGENT, ANDROID_USER_AGENT)
        .send()?
        .error_for_status()?;
    let xml = match progress {
        Some((steps, i)) => {
            // Determinate byte bar when the feed reports a length, else a live
            // byte counter — either way the download shows measured movement.
            let pb = match response.content_length() {
                Some(len) => steps.set_bytes(i, len),
                None => steps.set_bytes_counter(i),
            };
            let mut buf = String::new();
            pb.wrap_read(response).read_to_string(&mut buf)?;
            buf
        }
        None => response.text()?,
    };
    if xml.is_empty() {
        return Err(anyhow!("caption track returned an empty response"));
    }
    transcript_xml_to_text(&xml)
}

fn get_transcript(client: &reqwest::blocking::Client, video_id: &str, lang: &str) -> Result<String> {
    fetch_transcript(client, video_id, lang, None)
}

#[derive(Deserialize)]
struct VideosResponse {
    #[serde(default)]
    items: Vec<VideoItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoItem {
    content_details: ContentDetails,
    #[serde(default)]
    snippet: Snippet,
    #[serde(default)]
    statistics: Statistics,
}

#[derive(Deserialize)]
struct ContentDetails {
    duration: String,
}

#[derive(Deserialize, Default)]
struct Snippet {
    #[serde(default)]
    title: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Statistics {
    #[serde(default)]
    comment_count: Option<String>,
}

struct VideoDetails {
    duration_minutes: u64,
    title: String,
    comment_count: Option<u64>,
}

fn get_video_details(
    client: &reqwest::blocking::Client,
    api_key: &str,
    video_id: &str,
) -> Result<VideoDetails> {
    let response: VideosResponse = client
        .get("https://www.googleapis.com/youtube/v3/videos")
        .query(&[
            ("part", "contentDetails,snippet,statistics"),
            ("id", video_id),
            ("key", api_key),
        ])
        .send()?
        .error_for_status()
        .context("error getting video details")?
        .json()?;
    let item = response
        .items
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("video not found"))?;
    let duration_minutes = parse_duration_minutes(&item.content_details.duration)?;
    let comment_count = item
        .statistics
        .comment_count
        .and_then(|c| c.parse::<u64>().ok());
    Ok(VideoDetails {
        duration_minutes,
        title: item.snippet.title,
        comment_count,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommentThreadsResponse {
    #[serde(default)]
    items: Vec<CommentThread>,
    #[serde(default)]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct CommentThread {
    snippet: ThreadSnippet,
    replies: Option<Replies>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadSnippet {
    top_level_comment: Comment,
}

#[derive(Deserialize)]
struct Comment {
    snippet: CommentSnippet,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommentSnippet {
    text_display: String,
}

#[derive(Deserialize)]
struct Replies {
    #[serde(default)]
    comments: Vec<Comment>,
}

fn get_comments(
    client: &reqwest::blocking::Client,
    api_key: &str,
    video_id: &str,
) -> Vec<String> {
    let result: Result<CommentThreadsResponse> = (|| {
        Ok(client
            .get("https://www.googleapis.com/youtube/v3/commentThreads")
            .query(&[
                ("part", "snippet,replies"),
                ("videoId", video_id),
                ("textFormat", "plainText"),
                ("maxResults", "100"),
                ("key", api_key),
            ])
            .send()?
            .error_for_status()?
            .json()?)
    })();

    let response = match result {
        Ok(response) => response,
        Err(err) => {
            eprintln!("Failed to fetch comments: {err}");
            return Vec::new();
        }
    };

    let mut comments = Vec::new();
    for thread in response.items {
        comments.push(thread.snippet.top_level_comment.snippet.text_display);
        if let Some(replies) = thread.replies {
            for reply in replies.comments {
                comments.push(format!("    - {}", reply.snippet.text_display));
            }
        }
    }
    comments
}

const MAX_COMMENT_PAGES: usize = 5;

/// Fetch comments across up to `MAX_COMMENT_PAGES` pages. When `progress` and
/// `target` (the video's commentCount) are known, advances a count-measured bar;
/// otherwise leaves any spinner running.
fn get_comments_paginated(
    client: &reqwest::blocking::Client,
    api_key: &str,
    video_id: &str,
    target: Option<u64>,
    progress: Option<(&Steps, usize)>,
) -> Vec<String> {
    let bar_len = target.map(|t| t.min((MAX_COMMENT_PAGES * 100) as u64));
    let bar: Option<&ProgressBar> = match (progress, bar_len) {
        (Some((steps, i)), Some(len)) => Some(steps.set_count(i, len)),
        (Some((steps, i)), None) => Some(steps.bar(i)),
        (None, _) => None,
    };

    let mut comments = Vec::new();
    let mut page_token: Option<String> = None;
    for _ in 0..MAX_COMMENT_PAGES {
        let mut query = vec![
            ("part", "snippet,replies".to_string()),
            ("videoId", video_id.to_string()),
            ("textFormat", "plainText".to_string()),
            ("maxResults", "100".to_string()),
            ("key", api_key.to_string()),
        ];
        if let Some(ref tok) = page_token {
            query.push(("pageToken", tok.clone()));
        }

        let result: Result<CommentThreadsResponse> = (|| {
            Ok(client
                .get("https://www.googleapis.com/youtube/v3/commentThreads")
                .query(&query)
                .send()?
                .error_for_status()?
                .json()?)
        })();

        let response = match result {
            Ok(response) => response,
            Err(err) => {
                if comments.is_empty() {
                    eprintln!("Failed to fetch comments: {err}");
                }
                break;
            }
        };

        for thread in response.items {
            comments.push(thread.snippet.top_level_comment.snippet.text_display);
            if let Some(replies) = thread.replies {
                for reply in replies.comments {
                    comments.push(format!("    - {}", reply.snippet.text_display));
                }
            }
        }

        if let Some(pb) = bar {
            let pos = match bar_len {
                Some(len) => (comments.len() as u64).min(len),
                None => comments.len() as u64,
            };
            pb.set_position(pos);
        }

        match response.next_page_token {
            Some(tok) => page_token = Some(tok),
            None => break,
        }
    }
    comments
}

const DEFAULT_SUMMARY_PROMPT: &str =
    "Summarize the key points and main takeaways of this video transcript as concise bullet points.";

/// Resolve the summary prompt: explicit --prompt wins, then YT_SUMMARY_PROMPT
/// from the environment (or ~/.config/yt/.env), then a built-in default.
fn resolve_prompt(cli: Option<String>) -> String {
    if let Some(p) = cli {
        return p;
    }
    match std::env::var("YT_SUMMARY_PROMPT") {
        Ok(p) if !p.trim().is_empty() => p,
        _ => DEFAULT_SUMMARY_PROMPT.to_string(),
    }
}

/// Turn a video title into a safe filename base (no extension). Strips
/// filesystem-illegal and control characters, collapses whitespace, trims, and
/// caps length. Falls back to the video id when nothing usable remains.
fn sanitize_filename(title: &str, video_id: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let mut name = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    name = name
        .trim_matches(|c: char| c == '.' || c.is_whitespace())
        .to_string();
    const MAX: usize = 100;
    if name.chars().count() > MAX {
        name = name.chars().take(MAX).collect::<String>().trim_end().to_string();
    }
    if name.is_empty() {
        video_id.to_string()
    } else {
        name
    }
}

/// A vertical, connected list of workflow steps rendered to stderr:
///
/// ```text
/// ◦ Fetching video details ✓
/// |
/// ◦ Downloading transcript ⠹
/// ```
///
/// Each step is pending, then spins while active, then shows ✓ (done) or ✗ (fail).
/// What indicator a step currently shows — determines how `done` freezes the
/// finished line so the completed bar/counter stays put on screen.
#[derive(Clone, Copy)]
enum StepKind {
    Elapsed,
    Bytes,
    BytesCounter,
    Count,
}

struct Steps {
    _mp: MultiProgress,
    bars: Vec<ProgressBar>,
    titles: Vec<String>,
    kinds: Vec<Cell<StepKind>>,
    _connectors: Vec<ProgressBar>,
}

impl Steps {
    fn new(titles: &[&str]) -> Steps {
        let mp = MultiProgress::new();
        let mut bars = Vec::new();
        let mut connectors = Vec::new();
        for (i, title) in titles.iter().enumerate() {
            let pb = mp.add(ProgressBar::new_spinner());
            pb.set_style(Steps::pending_style());
            pb.set_prefix("◦");
            pb.set_message(title.to_string());
            pb.tick();
            bars.push(pb);
            if i + 1 < titles.len() {
                let conn = mp.add(ProgressBar::new_spinner());
                conn.set_style(Steps::connector_style());
                conn.finish_with_message("|");
                connectors.push(conn);
            }
        }
        Steps {
            _mp: mp,
            kinds: titles.iter().map(|_| Cell::new(StepKind::Elapsed)).collect(),
            bars,
            titles: titles.iter().map(|t| t.to_string()).collect(),
            _connectors: connectors,
        }
    }

    fn pending_style() -> ProgressStyle {
        ProgressStyle::with_template("{prefix} {msg}").unwrap()
    }

    fn active_style() -> ProgressStyle {
        ProgressStyle::with_template("{prefix} {msg} [{elapsed}] {spinner}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
    }

    fn connector_style() -> ProgressStyle {
        ProgressStyle::with_template("{msg}").unwrap()
    }

    /// The underlying bar for step `i`, so a fetch can set_position it.
    fn bar(&self, i: usize) -> &ProgressBar {
        &self.bars[i]
    }

    fn start(&self, i: usize) {
        self.kinds[i].set(StepKind::Elapsed);
        let pb = &self.bars[i];
        pb.reset_elapsed();
        pb.set_style(Steps::active_style());
        pb.enable_steady_tick(Duration::from_millis(100));
    }

    /// Switch step `i` to a determinate byte bar of the given length.
    fn set_bytes(&self, i: usize, len: u64) -> &ProgressBar {
        self.kinds[i].set(StepKind::Bytes);
        let pb = &self.bars[i];
        pb.set_style(
            ProgressStyle::with_template("{prefix} {msg} [{bar:20.cyan/blue}] {bytes}/{total_bytes}")
                .unwrap()
                .progress_chars("=> "),
        );
        pb.set_length(len);
        pb
    }

    /// Switch step `i` to an indeterminate byte counter (no known Content-Length).
    fn set_bytes_counter(&self, i: usize) -> &ProgressBar {
        self.kinds[i].set(StepKind::BytesCounter);
        let pb = &self.bars[i];
        pb.set_style(
            ProgressStyle::with_template("{prefix} {msg} [{elapsed}] {decimal_bytes} {spinner}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
        );
        pb
    }

    /// Switch step `i` to a determinate count bar of the given length.
    fn set_count(&self, i: usize, len: u64) -> &ProgressBar {
        self.kinds[i].set(StepKind::Count);
        let pb = &self.bars[i];
        pb.set_style(
            ProgressStyle::with_template("{prefix} {msg} [{bar:20.cyan/blue}] {pos}/{len}")
                .unwrap()
                .progress_chars("=> "),
        );
        pb.set_length(len);
        pb
    }

    /// Freeze step `i` as finished, keeping its bar/counter visible with a ✓.
    fn done(&self, i: usize) {
        let template = match self.kinds[i].get() {
            StepKind::Elapsed => "{prefix} {msg} [{elapsed}] ✓",
            StepKind::Bytes => "{prefix} {msg} [{bar:20.green}] {bytes}/{total_bytes} ✓",
            StepKind::BytesCounter => "{prefix} {msg} {decimal_bytes} ✓",
            StepKind::Count => "{prefix} {msg} [{bar:20.green}] {pos}/{len} ✓",
        };
        let pb = &self.bars[i];
        pb.set_style(
            ProgressStyle::with_template(template)
                .unwrap()
                .progress_chars("=> "),
        );
        pb.finish();
    }

    fn fail(&self, i: usize) {
        let pb = &self.bars[i];
        pb.set_style(ProgressStyle::with_template("{prefix} {msg} ✗").unwrap());
        pb.finish_with_message(self.titles[i].clone());
    }
}

/// Pipe the transcript to `claude -p "<prompt>"` and return its stdout summary.
fn run_claude(prompt: &str, transcript: &str) -> Result<String> {
    let mut child = Command::new("claude")
        .args(["-p", prompt])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                anyhow!(
                    "the `claude` CLI is required for --summarize; install it and \
                     ensure it is on your PATH"
                )
            } else {
                anyhow!("failed to start claude: {err}")
            }
        })?;

    // Write the transcript to claude's stdin from a separate thread so stdout can
    // be read concurrently — avoids a pipe deadlock on large transcripts.
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("failed to open claude stdin"))?;
    let transcript_owned = transcript.to_string();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(transcript_owned.as_bytes());
    });

    let output = child.wait_with_output().context("failed to run claude")?;
    let _ = writer.join();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "claude exited with status {}: {}",
            output.status,
            stderr.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Run the summarize pipeline with a stepped progress display: fetch video
/// details (for the title), download the transcript, summarize with claude, and
/// write the result to `<sanitized-title>.md` in the current directory.
fn run_summary(
    client: &reqwest::blocking::Client,
    api_key: &str,
    video_id: &str,
    lang: &str,
    prompt: &str,
) -> Result<PathBuf> {
    let steps = Steps::new(&[
        "Fetching video details",
        "Downloading transcript",
        "Summarizing with claude",
    ]);

    steps.start(0);
    let details = match get_video_details(client, api_key, video_id) {
        Ok(d) => {
            steps.done(0);
            d
        }
        Err(e) => {
            steps.fail(0);
            return Err(e);
        }
    };

    steps.start(1);
    let transcript = match fetch_transcript(client, video_id, lang, Some((&steps, 1))) {
        Ok(t) => {
            steps.done(1);
            t
        }
        Err(e) => {
            steps.fail(1);
            return Err(e).context("cannot summarize: transcript unavailable");
        }
    };

    steps.start(2);
    let summary = match run_claude(prompt, &transcript) {
        Ok(s) => {
            steps.done(2);
            s
        }
        Err(e) => {
            steps.fail(2);
            return Err(e);
        }
    };

    let path = PathBuf::from(format!("{}.md", sanitize_filename(&details.title, video_id)));
    std::fs::write(&path, summary).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

fn main() -> Result<()> {
    let options = Options::parse();

    let home = std::env::var("HOME").context("could not determine home directory")?;
    let env_file = format!("{home}/.config/yt/.env");
    let _ = dotenvy::from_path(&env_file);
    // Only the duration and comments modes talk to the YouTube Data API; the
    // transcript comes from the InnerTube endpoint and needs no key.
    let api_key = std::env::var("YOUTUBE_API_KEY").ok();
    let require_key = || {
        api_key.clone().ok_or_else(|| {
            anyhow!(
                "YOUTUBE_API_KEY not set. Export it, or store it with \
                 `mkdir -p ~/.config/yt && echo YOUTUBE_API_KEY=\"[Your API Key]\" >> ~/.config/yt/.env`."
            )
        })
    };

    let video_id = get_video_id(&options.url).ok_or_else(|| anyhow!("Invalid YouTube URL"))?;

    let client = reqwest::blocking::Client::new();

    let transcript_or_message = |client: &reqwest::blocking::Client| {
        get_transcript(client, &video_id, &options.lang)
            .unwrap_or_else(|err| format!("Transcript not available. ({err})"))
    };

    if options.duration {
        let details = get_video_details(&client, &require_key()?, &video_id)?;
        println!("{}", details.duration_minutes);
    } else if options.title {
        let details = get_video_details(&client, &require_key()?, &video_id)?;
        println!("{}", details.title);
    } else if options.transcript {
        println!("{}", transcript_or_message(&client));
    } else if options.comments {
        let comments = get_comments(&client, &require_key()?, &video_id);
        println!("{}", serde_json::to_string_pretty(&comments)?);
    } else if options.summarize {
        let api_key = require_key()?;
        let prompt = resolve_prompt(options.prompt.clone());
        let path = run_summary(&client, &api_key, &video_id, &options.lang, &prompt)?;
        eprintln!("Wrote {}", path.display());
    } else {
        let api_key = require_key()?;
        let steps = Steps::new(&[
            "Fetching video details",
            "Downloading transcript",
            "Fetching comments",
        ]);

        steps.start(0);
        let details = match get_video_details(&client, &api_key, &video_id) {
            Ok(d) => {
                steps.done(0);
                d
            }
            Err(e) => {
                steps.fail(0);
                return Err(e);
            }
        };

        steps.start(1);
        let transcript = match fetch_transcript(&client, &video_id, &options.lang, Some((&steps, 1))) {
            Ok(t) => {
                steps.done(1);
                t
            }
            Err(err) => {
                steps.fail(1);
                format!("Transcript not available. ({err})")
            }
        };

        steps.start(2);
        let comments =
            get_comments_paginated(&client, &api_key, &video_id, details.comment_count, Some((&steps, 2)));
        steps.done(2);

        drop(steps);

        let output = json!({
            "title": details.title,
            "transcript": transcript,
            "duration": details.duration_minutes,
            "comments": comments,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_illegal_chars() {
        assert_eq!(
            sanitize_filename("A/B:C*D?\"E<F>G|H\\I", "vid123"),
            "A B C D E F G H I"
        );
    }

    #[test]
    fn sanitize_collapses_whitespace_and_trims() {
        assert_eq!(sanitize_filename("  hello   world  ", "vid123"), "hello world");
    }

    #[test]
    fn sanitize_empty_title_falls_back_to_video_id() {
        assert_eq!(sanitize_filename("   ", "vid123"), "vid123");
        assert_eq!(sanitize_filename("///", "vid123"), "vid123");
    }

    #[test]
    fn sanitize_trims_dots() {
        assert_eq!(sanitize_filename("...title...", "vid123"), "title");
    }

    #[test]
    fn sanitize_truncates_long_titles() {
        let long = "a".repeat(250);
        let out = sanitize_filename(&long, "vid123");
        assert_eq!(out.chars().count(), 100);
    }

    #[test]
    fn video_id_from_watch_url() {
        assert_eq!(
            get_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ").as_deref(),
            Some("dQw4w9WgXcQ")
        );
    }

    #[test]
    fn video_id_from_watch_url_with_extra_params() {
        assert_eq!(
            get_video_id("https://www.youtube.com/watch?list=PL123&v=dQw4w9WgXcQ&t=42s").as_deref(),
            Some("dQw4w9WgXcQ")
        );
    }

    #[test]
    fn video_id_from_short_url() {
        assert_eq!(
            get_video_id("https://youtu.be/dQw4w9WgXcQ").as_deref(),
            Some("dQw4w9WgXcQ")
        );
    }

    #[test]
    fn video_id_from_embed_url() {
        assert_eq!(
            get_video_id("https://www.youtube.com/embed/dQw4w9WgXcQ").as_deref(),
            Some("dQw4w9WgXcQ")
        );
    }

    #[test]
    fn video_id_without_scheme() {
        assert_eq!(
            get_video_id("youtube.com/watch?v=dQw4w9WgXcQ").as_deref(),
            Some("dQw4w9WgXcQ")
        );
    }

    #[test]
    fn video_id_rejects_non_youtube_url() {
        assert_eq!(get_video_id("https://example.com/watch?v=dQw4w9WgXcQ"), None);
        assert_eq!(get_video_id("not a url"), None);
    }

    #[test]
    fn duration_hours_minutes_seconds() {
        assert_eq!(parse_duration_minutes("PT1H2M30S").unwrap(), 63); // 62.5 rounds up
    }

    #[test]
    fn duration_minutes_only() {
        assert_eq!(parse_duration_minutes("PT15M").unwrap(), 15);
    }

    #[test]
    fn duration_seconds_round_to_nearest_minute() {
        assert_eq!(parse_duration_minutes("PT29S").unwrap(), 0);
        assert_eq!(parse_duration_minutes("PT30S").unwrap(), 1);
        assert_eq!(parse_duration_minutes("PT59S").unwrap(), 1);
    }

    #[test]
    fn duration_rejects_garbage() {
        assert!(parse_duration_minutes("1h30m").is_err());
    }

    fn track(lang: &str, kind: &str, url: &str) -> CaptionTrack {
        CaptionTrack {
            base_url: url.to_string(),
            language_code: lang.to_string(),
            kind: kind.to_string(),
        }
    }

    #[test]
    fn caption_track_selection_prefers_manual_in_requested_lang() {
        let tracks = vec![
            track("en", "asr", "en-auto"),
            track("en", "", "en-manual"),
            track("nl", "", "nl-manual"),
        ];
        assert_eq!(pick_caption_track(&tracks, "en").unwrap().base_url, "en-manual");
        assert_eq!(pick_caption_track(&tracks, "nl").unwrap().base_url, "nl-manual");
        // requested lang only has auto-generated: take it
        let auto_only = vec![track("en", "asr", "en-auto"), track("nl", "", "nl-manual")];
        assert_eq!(pick_caption_track(&auto_only, "en").unwrap().base_url, "en-auto");
        // unknown lang: fall back to first manual track, then first
        assert_eq!(pick_caption_track(&tracks, "de").unwrap().base_url, "en-manual");
        assert!(pick_caption_track(&[], "en").is_none());
    }

    #[test]
    fn caption_tracks_parse_from_innertube_json() {
        let value: serde_json::Value = serde_json::from_str(
            r#"[{"baseUrl":"https://example.com/en","languageCode":"en","kind":"asr","name":{"runs":[{"text":"English"}]}}]"#,
        )
        .unwrap();
        let tracks: Vec<CaptionTrack> = serde_json::from_value(value).unwrap();
        assert_eq!(tracks[0].base_url, "https://example.com/en");
        assert_eq!(tracks[0].kind, "asr");
    }

    #[test]
    fn transcript_legacy_text_format_decodes_entities() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><transcript><text start="0" dur="2">hello &amp;#39;world&amp;#39;</text><text start="2" dur="2">it&#39;s fine</text></transcript>"#;
        assert_eq!(
            transcript_xml_to_text(xml).unwrap(),
            "hello 'world' it's fine"
        );
    }

    #[test]
    fn transcript_timedtext_p_format() {
        let xml = "<?xml version=\"1.0\" encoding=\"utf-8\" ?><timedtext format=\"3\">\n<body>\n<p t=\"0\" d=\"2\">♪ We&#39;re no strangers to love ♪</p>\n<p t=\"2\" d=\"2\">♪ You know the rules\nand so do I ♪</p>\n</body>\n</timedtext>";
        assert_eq!(
            transcript_xml_to_text(xml).unwrap(),
            "♪ We're no strangers to love ♪ ♪ You know the rules and so do I ♪"
        );
    }

    #[test]
    fn transcript_xml_skips_empty_text_nodes() {
        let xml = "<transcript><text start=\"0\" dur=\"1\">   </text><text start=\"1\" dur=\"1\">ok</text></transcript>";
        assert_eq!(transcript_xml_to_text(xml).unwrap(), "ok");
    }
}
