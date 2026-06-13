use std::ffi::OsString;
use std::process::Command;

use crate::error::{BurnerError, Result};

use super::RenderedJob;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncoderReport {
    pub subtitle_count: usize,
    pub args: Vec<OsString>,
}

pub fn encode_with_ffmpeg(job: &RenderedJob) -> Result<EncoderReport> {
    let args = build_ffmpeg_args(job);

    if job.dry_run {
        println!("ffmpeg {}", display_args(&args));
        return Ok(EncoderReport {
            subtitle_count: job.track.len(),
            args,
        });
    }

    let output = Command::new("ffmpeg").args(&args).output();
    let output = match output {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(BurnerError::FfmpegNotFound);
        }
        Err(err) => return Err(BurnerError::Io(err)),
    };

    if !output.status.success() {
        return Err(BurnerError::FfmpegFailed {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    Ok(EncoderReport {
        subtitle_count: job.track.len(),
        args,
    })
}

pub fn build_ffmpeg_args(job: &RenderedJob) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("-y"),
        OsString::from("-i"),
        job.packet.input.as_os_str().to_os_string(),
        OsString::from("-vf"),
        OsString::from(job.plan.filter.clone()),
        OsString::from("-c:v"),
        OsString::from("libx264"),
        OsString::from("-preset"),
        OsString::from("medium"),
        OsString::from("-crf"),
        OsString::from("23"),
        OsString::from("-c:a"),
        OsString::from("copy"),
        OsString::from("-movflags"),
        OsString::from("+faststart"),
    ];

    if let Some(font) = &job.style.font {
        if let Some(font_dir) = font.parent() {
            args.push(OsString::from("-fontsdir"));
            args.push(font_dir.as_os_str().to_os_string());
        }
    }

    args.push(job.packet.output.as_os_str().to_os_string());
    args
}

fn display_args(args: &[OsString]) -> String {
    args.iter()
        .map(|arg| {
            let text = arg.to_string_lossy();
            if text.contains(' ') {
                format!("\"{text}\"")
            } else {
                text.into_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::pipeline::{RenderPlan, SubtitleStyle, VideoPacket};
    use crate::subtitle::{SubtitleEntry, SubtitleTrack};

    use super::*;

    #[test]
    fn args_include_video_and_audio_settings() {
        let style = SubtitleStyle::default();
        let job = RenderedJob {
            packet: VideoPacket {
                input: PathBuf::from("in.mp4"),
                subtitle: PathBuf::from("a.srt"),
                output: PathBuf::from("out.mp4"),
                subtitle_text: String::new(),
                generated_srt: None,
            },
            track: SubtitleTrack::new(vec![SubtitleEntry {
                index: 1,
                start_ms: 0,
                end_ms: 1000,
                text: "x".to_string(),
            }]),
            plan: RenderPlan {
                filter: "subtitles=a.srt".to_string(),
                subtitle_path: PathBuf::from("a.srt"),
                style: style.clone(),
            },
            style,
            dry_run: true,
        };

        let args = build_ffmpeg_args(&job);
        assert!(args.contains(&OsString::from("-vf")));
        assert!(args.contains(&OsString::from("libx264")));
        assert!(args.contains(&OsString::from("copy")));
    }
}
