use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{BurnerError, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct CompanionOptions {
    pub input: PathBuf,
    pub sticker: PathBuf,
    pub output: PathBuf,
    pub python: PathBuf,
    pub ffmpeg: PathBuf,
    pub script: PathBuf,
    pub scale: f32,
    pub y_offset: f32,
    pub smooth: f32,
    pub min_size: u32,
    pub lost_frames: u32,
    pub verbose: bool,
    pub dry_run: bool,
}

impl CompanionOptions {
    pub fn with_defaults(input: PathBuf, sticker: PathBuf, output: PathBuf) -> Self {
        Self {
            input,
            sticker,
            output,
            python: PathBuf::from("C:/software/Anaconda/envs/cv_env/python.exe"),
            ffmpeg: default_ffmpeg_path(),
            script: PathBuf::from("scripts/companion_sticker.py"),
            scale: 1.6,
            y_offset: 0.08,
            smooth: 0.72,
            min_size: 70,
            lost_frames: 12,
            verbose: false,
            dry_run: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionReport {
    pub output: PathBuf,
    pub dry_run: bool,
}

pub fn run_companion(options: CompanionOptions) -> Result<CompanionReport> {
    validate_options(&options)?;

    let silent_video = std::env::temp_dir().join(unique_temp_name("companion-video", "mp4"));
    let script_args = build_script_args(&options, &silent_video);

    if options.dry_run {
        println!(
            "{} {}",
            options.python.display(),
            display_args(&script_args)
        );
        println!(
            "{} {}",
            options.ffmpeg.display(),
            display_args(&merge_audio_args(&options, &silent_video))
        );
        return Ok(CompanionReport {
            output: options.output,
            dry_run: true,
        });
    }

    if options.verbose {
        eprintln!(
            "[companion] processing frames with {}",
            options.script.display()
        );
    }
    run_python_script(&options.python, script_args)?;

    if options.verbose {
        eprintln!("[companion] merging original audio");
    }
    run_ffmpeg(&options.ffmpeg, merge_audio_args(&options, &silent_video))?;

    Ok(CompanionReport {
        output: options.output,
        dry_run: false,
    })
}

fn validate_options(options: &CompanionOptions) -> Result<()> {
    if !options.input.is_file() {
        return Err(BurnerError::InputNotFound {
            path: options.input.clone(),
        });
    }
    if !options.sticker.is_file() {
        return Err(BurnerError::InvalidArguments {
            message: format!("sticker image not found: {}", options.sticker.display()),
        });
    }
    if !options.script.is_file() {
        return Err(BurnerError::InvalidArguments {
            message: format!("companion script not found: {}", options.script.display()),
        });
    }
    if options.scale <= 0.0 {
        return Err(BurnerError::InvalidArguments {
            message: "--scale must be greater than 0".to_string(),
        });
    }
    if !(0.0..=0.98).contains(&options.smooth) {
        return Err(BurnerError::InvalidArguments {
            message: "--smooth must be between 0 and 0.98".to_string(),
        });
    }
    if options.min_size == 0 {
        return Err(BurnerError::InvalidArguments {
            message: "--min-face must be greater than 0".to_string(),
        });
    }
    if let Some(parent) = options.output.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(BurnerError::OutputDirectoryNotFound {
                path: parent.to_path_buf(),
            });
        }
    }
    Ok(())
}

fn build_script_args(options: &CompanionOptions, output: &Path) -> Vec<OsString> {
    let mut args = vec![
        options.script.as_os_str().to_os_string(),
        "--input".into(),
        options.input.as_os_str().to_os_string(),
        "--sticker".into(),
        options.sticker.as_os_str().to_os_string(),
        "--output".into(),
        output.as_os_str().to_os_string(),
        "--scale".into(),
        options.scale.to_string().into(),
        "--y-offset".into(),
        options.y_offset.to_string().into(),
        "--smooth".into(),
        options.smooth.to_string().into(),
        "--min-size".into(),
        options.min_size.to_string().into(),
        "--lost-frames".into(),
        options.lost_frames.to_string().into(),
    ];
    if options.verbose {
        args.push("--verbose".into());
    }
    args
}

fn merge_audio_args(options: &CompanionOptions, silent_video: &Path) -> Vec<OsString> {
    vec![
        "-y".into(),
        "-i".into(),
        silent_video.as_os_str().to_os_string(),
        "-i".into(),
        options.input.as_os_str().to_os_string(),
        "-map".into(),
        "0:v:0".into(),
        "-map".into(),
        "1:a?".into(),
        "-c:v".into(),
        "copy".into(),
        "-c:a".into(),
        "copy".into(),
        "-shortest".into(),
        options.output.as_os_str().to_os_string(),
    ]
}

fn run_python_script(python: &Path, args: Vec<OsString>) -> Result<()> {
    let output = Command::new(python).args(&args).output();
    let output = match output {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(BurnerError::PythonNotFound {
                path: python.to_path_buf(),
            });
        }
        Err(err) => return Err(BurnerError::Io(err)),
    };

    if !output.status.success() {
        return Err(BurnerError::PythonFailed {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

fn run_ffmpeg(ffmpeg: &Path, args: Vec<OsString>) -> Result<()> {
    let output = Command::new(ffmpeg).args(&args).output();
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

fn default_ffmpeg_path() -> PathBuf {
    let winget = PathBuf::from(
        "C:/Users/XRZ/AppData/Local/Microsoft/WinGet/Packages/Gyan.FFmpeg_Microsoft.Winget.Source_8wekyb3d8bbwe/ffmpeg-8.1.1-full_build/bin/ffmpeg.exe",
    );
    if winget.is_file() {
        winget
    } else {
        PathBuf::from("ffmpeg")
    }
}

fn unique_temp_name(prefix: &str, ext: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("{prefix}-{}-{millis}.{ext}", std::process::id())
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
    fn defaults_are_reasonable() {
        let options = CompanionOptions::with_defaults(
            "in.mp4".into(),
            "image/image.png".into(),
            "out.mp4".into(),
        );

        assert_eq!(options.scale, 1.6);
        assert_eq!(options.y_offset, 0.08);
        assert_eq!(options.smooth, 0.72);
        assert_eq!(
            options.script,
            PathBuf::from("scripts/companion_sticker.py")
        );
    }
}
