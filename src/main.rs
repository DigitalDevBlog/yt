use anyhow::{Context, Result, anyhow};
use clap::Parser;
use quick_xml::events::Event;
use regex::Regex;
use serde::Deserialize;
use serde_json::json;

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

fn get_transcript(client: &reqwest::blocking::Client, video_id: &str, lang: &str) -> Result<String> {
    let tracks = get_caption_tracks(client, video_id)?;
    let track = pick_caption_track(&tracks, lang).ok_or_else(|| anyhow!("no caption tracks found"))?;
    let xml = client
        .get(&track.base_url)
        .header(reqwest::header::USER_AGENT, ANDROID_USER_AGENT)
        .send()?
        .error_for_status()?
        .text()?;
    if xml.is_empty() {
        return Err(anyhow!("caption track returned an empty response"));
    }
    transcript_xml_to_text(&xml)
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
}

#[derive(Deserialize)]
struct ContentDetails {
    duration: String,
}

fn get_duration_minutes(
    client: &reqwest::blocking::Client,
    api_key: &str,
    video_id: &str,
) -> Result<u64> {
    let response: VideosResponse = client
        .get("https://www.googleapis.com/youtube/v3/videos")
        .query(&[("part", "contentDetails"), ("id", video_id), ("key", api_key)])
        .send()?
        .error_for_status()
        .context("error getting video details")?
        .json()?;
    let item = response.items.first().ok_or_else(|| anyhow!("video not found"))?;
    parse_duration_minutes(&item.content_details.duration)
}

#[derive(Deserialize)]
struct CommentThreadsResponse {
    #[serde(default)]
    items: Vec<CommentThread>,
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

fn main() -> Result<()> {
    let options = Options::parse();

    let home = std::env::var("HOME").context("could not determine home directory")?;
    let env_file = format!("{home}/.config/fabric/.env");
    let _ = dotenvy::from_path(&env_file);
    // Only the duration and comments modes talk to the YouTube Data API; the
    // transcript comes from the InnerTube endpoint and needs no key.
    let api_key = std::env::var("YOUTUBE_API_KEY").ok();
    let require_key = || {
        api_key.clone().ok_or_else(|| {
            anyhow!(
                "YOUTUBE_API_KEY not found in ~/.config/fabric/.env. To add please run \
                 `echo YOUTUBE_API_KEY=\"[Your API Key]\" >> ~/.config/fabric/.env`."
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
        println!("{}", get_duration_minutes(&client, &require_key()?, &video_id)?);
    } else if options.transcript {
        println!("{}", transcript_or_message(&client));
    } else if options.comments {
        let comments = get_comments(&client, &require_key()?, &video_id);
        println!("{}", serde_json::to_string_pretty(&comments)?);
    } else {
        let api_key = require_key()?;
        let output = json!({
            "transcript": transcript_or_message(&client),
            "duration": get_duration_minutes(&client, &api_key, &video_id)?,
            "comments": get_comments(&client, &api_key, &video_id),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
