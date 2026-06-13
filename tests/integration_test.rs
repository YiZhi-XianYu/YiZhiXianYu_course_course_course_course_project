use std::path::Path;

use subtitle_burner::pipeline::renderer::build_subtitles_filter;
use subtitle_burner::pipeline::SubtitleStyle;
use subtitle_burner::subtitle::{parse_srt, SubtitleTrack};

fn sample_srt() -> &'static str {
    include_str!("test.srt")
}

#[test]
fn test_srt_parser_basic() {
    let entries = parse_srt(sample_srt()).unwrap();

    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].index, 1);
    assert_eq!(entries[0].text, "Hello, this is a test subtitle.");
}

#[test]
fn test_srt_parser_timestamps() {
    let entries = parse_srt(sample_srt()).unwrap();

    assert_eq!(entries[0].start_ms, 1_000);
    assert_eq!(entries[0].end_ms, 3_500);
    assert_eq!(entries[2].start_ms, 8_500);
    assert_eq!(entries[2].end_ms, 11_000);
}

#[test]
fn test_srt_strip_html_tags() {
    let entries = parse_srt(sample_srt()).unwrap();

    assert_eq!(entries[3].text, "带有 HTML 标签的字幕");
}

#[test]
fn test_srt_multiline() {
    let entries = parse_srt(sample_srt()).unwrap();

    assert_eq!(entries[2].text, "This is the third line.\nIt has two rows.");
}

#[test]
fn test_subtitle_lookup_by_pts() {
    let track = SubtitleTrack::new(parse_srt(sample_srt()).unwrap());

    assert_eq!(
        track.at(4_200).map(|entry| entry.text.as_str()),
        Some("第二条字幕，测试中文支持。")
    );
    assert!(track.at(7_500).is_none());
}

#[test]
fn test_ffmpeg_filter_generation() {
    let style = SubtitleStyle {
        font: None,
        font_size: Some(24),
        shadow: true,
    };

    let filter = build_subtitles_filter(Path::new("tests/test.srt"), &style);

    assert!(filter.starts_with("subtitles=tests/test.srt"));
    assert!(filter.contains("Fontsize=24"));
    assert!(filter.contains("Shadow=1"));
}
