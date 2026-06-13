use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{BurnerError, Result};
use crate::tool_paths::ffmpeg_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsrOptions {
    pub whisper_path: PathBuf,
    pub model_path: PathBuf,
    pub language: AsrLanguage,
    pub keep_srt: bool,
}

impl Default for AsrOptions {
    fn default() -> Self {
        Self {
            whisper_path: PathBuf::from("tools/whisper/Release/whisper-cli.exe"),
            model_path: PathBuf::from("models/ggml-small.bin"),
            language: AsrLanguage::Auto,
            keep_srt: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsrLanguage {
    Auto,
    Chinese,
    English,
}

impl AsrLanguage {
    pub fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "zh" | "cn" | "chinese" => Ok(Self::Chinese),
            "en" | "english" => Ok(Self::English),
            _ => Err(BurnerError::InvalidArguments {
                message: "language 只支持 auto、zh、en".to_string(),
            }),
        }
    }

    pub fn whisper_code(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Chinese => "zh",
            Self::English => "en",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsrResult {
    pub srt_path: PathBuf,
    pub srt_text: String,
    pub kept_srt: Option<PathBuf>,
}

pub fn transcribe_video_to_srt(
    input: &Path,
    output: &Path,
    options: &AsrOptions,
    dry_run: bool,
    verbose: bool,
) -> Result<AsrResult> {
    validate_asr_tools(options)?;

    let work_dir = std::env::temp_dir().join(unique_work_dir_name());
    fs::create_dir_all(&work_dir)?;
    let audio_path = work_dir.join("audio.wav");
    let srt_stem = work_dir.join("auto-subtitle");
    let srt_path = srt_stem.with_extension("srt");

    if dry_run {
        println!(
            "{} {}",
            ffmpeg_path().display(),
            display_args(&extract_audio_args(input, &audio_path))
        );
        println!(
            "{} {}",
            options.whisper_path.display(),
            display_args(&whisper_args(options, &audio_path, &srt_stem))
        );
        let srt_text = "1\n00:00:00,000 --> 00:00:01,000\nAUTO SUBTITLE DRY RUN\n";
        return Ok(AsrResult {
            srt_path,
            srt_text: srt_text.to_string(),
            kept_srt: None,
        });
    }

    if verbose {
        eprintln!("[asr] extracting audio: {}", audio_path.display());
    }
    run_ffmpeg(extract_audio_args(input, &audio_path))?;

    if verbose {
        eprintln!(
            "[asr] transcribing with: {}",
            options.whisper_path.display()
        );
    }
    run_whisper(options, &audio_path, &srt_stem)?;

    if !srt_path.is_file() {
        return Err(BurnerError::AsrOutputMissing { path: srt_path });
    }

    let srt_text = fs::read_to_string(&srt_path)?;
    let stable_srt = output.with_extension("auto.srt");
    fs::copy(&srt_path, &stable_srt)?;
    let kept_srt = options.keep_srt.then_some(stable_srt.clone());

    Ok(AsrResult {
        srt_path: stable_srt,
        srt_text,
        kept_srt,
    })
}

pub fn validate_asr_tools(options: &AsrOptions) -> Result<()> {
    if !options.whisper_path.is_file() {
        return Err(BurnerError::WhisperNotFound {
            path: options.whisper_path.clone(),
        });
    }
    if !options.model_path.is_file() {
        return Err(BurnerError::InvalidArguments {
            message: format!("ASR 模型文件不存在: {}", options.model_path.display()),
        });
    }
    Ok(())
}

fn run_ffmpeg(args: Vec<OsString>) -> Result<()> {
    let output = Command::new(ffmpeg_path()).args(&args).output();
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
    Ok(())
}

fn run_whisper(options: &AsrOptions, audio_path: &Path, output_stem: &Path) -> Result<()> {
    let output = Command::new(&options.whisper_path)
        .args(whisper_args(options, audio_path, output_stem))
        .output();
    let output = match output {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(BurnerError::WhisperNotFound {
                path: options.whisper_path.clone(),
            });
        }
        Err(err) => return Err(BurnerError::Io(err)),
    };

    if !output.status.success() {
        return Err(BurnerError::WhisperFailed {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

fn extract_audio_args(input: &Path, output: &Path) -> Vec<OsString> {
    vec![
        "-y".into(),
        "-i".into(),
        input.as_os_str().to_os_string(),
        "-vn".into(),
        "-ac".into(),
        "1".into(),
        "-ar".into(),
        "16000".into(),
        output.as_os_str().to_os_string(),
    ]
}

fn whisper_args(options: &AsrOptions, audio_path: &Path, output_stem: &Path) -> Vec<OsString> {
    vec![
        "-m".into(),
        options.model_path.as_os_str().to_os_string(),
        "-f".into(),
        audio_path.as_os_str().to_os_string(),
        "-l".into(),
        options.language.whisper_code().into(),
        "-osrt".into(),
        "-of".into(),
        output_stem.as_os_str().to_os_string(),
    ]
}

fn unique_work_dir_name() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("subtitle-burner-asr-{}-{millis}", std::process::id())
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
    use super::*;

    #[test]
    fn parses_supported_languages() {
        assert_eq!(AsrLanguage::parse("auto").unwrap(), AsrLanguage::Auto);
        assert_eq!(AsrLanguage::parse("zh").unwrap(), AsrLanguage::Chinese);
        assert_eq!(AsrLanguage::parse("en").unwrap(), AsrLanguage::English);
        assert!(AsrLanguage::parse("ja").is_err());
    }

    #[test]
    fn default_paths_match_downloaded_layout() {
        let options = AsrOptions::default();
        assert_eq!(
            options.whisper_path,
            PathBuf::from("tools/whisper/Release/whisper-cli.exe")
        );
        assert_eq!(options.model_path, PathBuf::from("models/ggml-small.bin"));
    }
}
