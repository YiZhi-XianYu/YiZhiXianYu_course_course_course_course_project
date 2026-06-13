pub mod decoder;
pub mod encoder;
pub mod renderer;

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use crate::asr::AsrOptions;
use crate::error::{BurnerError, Result};
use crate::subtitle::{parse_srt, SubtitleTrack};

pub use decoder::decode_request;
pub use encoder::{build_ffmpeg_args, encode_with_ffmpeg};
pub use renderer::{RenderPlan, Renderer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleStyle {
    pub font: Option<PathBuf>,
    pub font_size: Option<u32>,
    pub shadow: bool,
}

impl Default for SubtitleStyle {
    fn default() -> Self {
        Self {
            font: None,
            font_size: None,
            shadow: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BurnOptions {
    pub input: PathBuf,
    pub subtitle: Option<PathBuf>,
    pub output: PathBuf,
    pub style: SubtitleStyle,
    pub auto_subtitle: Option<AsrOptions>,
    pub verbose: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineReport {
    pub subtitle_count: usize,
    pub output: PathBuf,
    pub dry_run: bool,
    pub generated_srt: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoPacket {
    pub input: PathBuf,
    pub subtitle: PathBuf,
    pub output: PathBuf,
    pub subtitle_text: String,
    pub generated_srt: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedJob {
    pub packet: VideoPacket,
    pub track: SubtitleTrack,
    pub plan: RenderPlan,
    pub style: SubtitleStyle,
    pub dry_run: bool,
}

pub fn run_burn_pipeline(options: BurnOptions) -> Result<PipelineReport> {
    validate_paths(&options)?;

    let (decode_tx, decode_rx) = mpsc::sync_channel::<Result<VideoPacket>>(32);
    let (render_tx, render_rx) = mpsc::sync_channel::<Result<RenderedJob>>(32);

    let decoder_options = options.clone();
    let decoder = thread::spawn(move || {
        let result = decode_request(&decoder_options);
        decode_tx
            .send(result)
            .map_err(|_| BurnerError::PipelineClosed { stage: "decoder" })
    });

    let renderer_options = options.clone();
    let renderer = thread::spawn(move || -> Result<()> {
        for packet in decode_rx {
            let rendered = packet.and_then(|packet| render_request(packet, &renderer_options));
            render_tx
                .send(rendered)
                .map_err(|_| BurnerError::PipelineClosed { stage: "renderer" })?;
        }
        Ok(())
    });

    let mut report = None;
    for rendered in render_rx {
        let job = rendered?;
        if options.verbose {
            eprintln!(
                "[pipeline] subtitles: {}, filter: {}",
                job.track.len(),
                job.plan.filter
            );
        }
        let ffmpeg_result = encode_with_ffmpeg(&job)?;
        report = Some(PipelineReport {
            subtitle_count: ffmpeg_result.subtitle_count,
            output: options.output.clone(),
            dry_run: options.dry_run,
            generated_srt: job.packet.generated_srt.clone(),
        });
    }

    join_stage(decoder, "decoder")??;
    join_stage(renderer, "renderer")??;

    report.ok_or(BurnerError::PipelineClosed { stage: "encoder" })
}

fn render_request(packet: VideoPacket, options: &BurnOptions) -> Result<RenderedJob> {
    let entries = parse_srt(&packet.subtitle_text)?;
    let track = SubtitleTrack::new(entries);
    let plan = Renderer::new(options.style.clone()).plan(&packet.subtitle)?;
    Ok(RenderedJob {
        packet,
        track,
        plan,
        style: options.style.clone(),
        dry_run: options.dry_run,
    })
}

fn validate_paths(options: &BurnOptions) -> Result<()> {
    if !options.input.is_file() {
        return Err(BurnerError::InputNotFound {
            path: options.input.clone(),
        });
    }

    match (&options.subtitle, &options.auto_subtitle) {
        (Some(subtitle), None) => {
            if !subtitle.is_file() {
                return Err(BurnerError::SubtitleNotFound {
                    path: subtitle.clone(),
                });
            }
        }
        (None, Some(asr)) => crate::asr::validate_asr_tools(asr)?,
        (Some(_), Some(_)) => {
            return Err(BurnerError::InvalidArguments {
                message: "--subtitle 和 --auto-subtitle 不能同时使用".to_string(),
            });
        }
        (None, None) => {
            return Err(BurnerError::InvalidArguments {
                message: "必须提供 --subtitle，或使用 --auto-subtitle 自动识别".to_string(),
            });
        }
    }
    if let Some(parent) = options.output.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(BurnerError::OutputDirectoryNotFound {
                path: parent.to_path_buf(),
            });
        }
    }
    if let Some(font) = &options.style.font {
        if !font.is_file() {
            return Err(BurnerError::InvalidArguments {
                message: format!("字体文件不存在: {}", font.display()),
            });
        }
    }
    Ok(())
}

fn join_stage<T>(handle: thread::JoinHandle<T>, stage: &'static str) -> Result<T> {
    handle
        .join()
        .map_err(|_| BurnerError::ThreadPanicked { stage })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_default_keeps_shadow() {
        assert!(SubtitleStyle::default().shadow);
    }
}
