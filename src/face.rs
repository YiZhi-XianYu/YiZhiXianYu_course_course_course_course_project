use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{BurnerError, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct MosaicOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub python: PathBuf,
    pub ffmpeg: PathBuf,
    pub script: PathBuf,
    pub scale: f32,
    pub block_size: u32,
    pub min_size: u32,
    pub verbose: bool,
    pub dry_run: bool,
}

impl MosaicOptions {
    pub fn with_defaults(input: PathBuf, output: PathBuf) -> Self {
        Self {
            input,
            output,
            python: PathBuf::from("C:/software/Anaconda/envs/cv_env/python.exe"),
            ffmpeg: default_ffmpeg_path(),
            script: PathBuf::from("scripts/face_mosaic.py"),
            scale: 1.25,
            block_size: 18,
            min_size: 40,
            verbose: false,
            dry_run: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MosaicReport {
    pub output: PathBuf,
    pub dry_run: bool,
}

pub fn run_mosaic(options: MosaicOptions) -> Result<MosaicReport> {
    validate_mosaic_options(&options)?;

    let silent_video = std::env::temp_dir().join(unique_temp_name("mosaic-video", "mp4"));
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
        return Ok(MosaicReport {
            output: options.output,
            dry_run: true,
        });
    }

    if options.verbose {
        eprintln!(
            "[mosaic] processing frames with {}",
            options.script.display()
        );
    }
    run_python_script(&options.python, script_args)?;

    if options.verbose {
        eprintln!("[mosaic] merging original audio");
    }
    run_ffmpeg(&options.ffmpeg, merge_audio_args(&options, &silent_video))?;

    Ok(MosaicReport {
        output: options.output,
        dry_run: false,
    })
}

fn validate_mosaic_options(options: &MosaicOptions) -> Result<()> {
    if !options.input.is_file() {
        return Err(BurnerError::InputNotFound {
            path: options.input.clone(),
        });
    }
    if !options.script.is_file() {
        return Err(BurnerError::InvalidArguments {
            message: format!("马赛克处理脚本不存在: {}", options.script.display()),
        });
    }
    if options.scale <= 0.0 {
        return Err(BurnerError::InvalidArguments {
            message: "--face-scale 必须大于 0".to_string(),
        });
    }
    if options.block_size == 0 {
        return Err(BurnerError::InvalidArguments {
            message: "--mosaic-block 必须大于 0".to_string(),
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

fn build_script_args(options: &MosaicOptions, output: &Path) -> Vec<OsString> {
    let mut args = vec![
        options.script.as_os_str().to_os_string(),
        "--input".into(),
        options.input.as_os_str().to_os_string(),
        "--output".into(),
        output.as_os_str().to_os_string(),
        "--scale".into(),
        options.scale.to_string().into(),
        "--block-size".into(),
        options.block_size.to_string().into(),
        "--min-size".into(),
        options.min_size.to_string().into(),
    ];
    if options.verbose {
        args.push("--verbose".into());
    }
    args
}

fn merge_audio_args(options: &MosaicOptions, silent_video: &Path) -> Vec<OsString> {
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
        let options = MosaicOptions::with_defaults("in.mp4".into(), "out.mp4".into());
        assert_eq!(options.scale, 1.25);
        assert_eq!(options.block_size, 18);
        assert_eq!(options.script, PathBuf::from("scripts/face_mosaic.py"));
        assert_eq!(
            options.python,
            PathBuf::from("C:/software/Anaconda/envs/cv_env/python.exe")
        );
    }
}
