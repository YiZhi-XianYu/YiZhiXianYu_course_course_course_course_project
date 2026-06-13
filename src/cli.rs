use std::env;
use std::path::PathBuf;

use crate::asr::{AsrLanguage, AsrOptions};
use crate::companion::CompanionOptions;
use crate::error::{BurnerError, Result};
use crate::face::MosaicOptions;
use crate::pipeline::{BurnOptions, SubtitleStyle};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Burn(Box<BurnOptions>),
    Mosaic(Box<MosaicOptions>),
    Companion(Box<CompanionOptions>),
    Help,
    Version,
}

pub fn parse_env() -> Result<Command> {
    parse_args(env::args().skip(1))
}

pub fn parse_args<I, S>(args: I) -> Result<Command>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args: Vec<String> = args.into_iter().map(Into::into).collect();
    if args.is_empty() {
        return Ok(Command::Help);
    }

    match args.first().map(String::as_str) {
        Some("mosaic") => {
            args.remove(0);
            parse_mosaic_args(args)
        }
        Some("companion") => {
            args.remove(0);
            parse_companion_args(args)
        }
        Some("-h" | "--help") => Ok(Command::Help),
        Some("-V" | "--version") => Ok(Command::Version),
        _ => parse_burn_args(args),
    }
}

fn parse_burn_args(args: Vec<String>) -> Result<Command> {
    let mut input = None;
    let mut subtitle = None;
    let mut output = None;
    let mut font = None;
    let mut font_size = None;
    let mut no_shadow = false;
    let mut verbose = false;
    let mut dry_run = false;
    let mut auto_subtitle = false;
    let mut language = AsrLanguage::Auto;
    let mut model = None;
    let mut whisper = None;
    let mut keep_srt = false;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "-i" | "--input" => input = Some(next_value(&mut iter, "--input")?),
            "-s" | "--subtitle" => subtitle = Some(next_value(&mut iter, "--subtitle")?),
            "-o" | "--output" => output = Some(next_value(&mut iter, "--output")?),
            "-f" | "--font" => font = Some(PathBuf::from(next_value(&mut iter, "--font")?)),
            "--font-size" => {
                let value = next_value(&mut iter, "--font-size")?;
                font_size =
                    Some(
                        value
                            .parse::<u32>()
                            .map_err(|_| BurnerError::InvalidArguments {
                                message: "--font-size must be a positive integer".to_string(),
                            })?,
                    );
            }
            "--no-shadow" => no_shadow = true,
            "-v" | "--verbose" => verbose = true,
            "--dry-run" => dry_run = true,
            "--auto-subtitle" => auto_subtitle = true,
            "--language" => {
                let value = next_value(&mut iter, "--language")?;
                language = AsrLanguage::parse(&value)?;
            }
            "--model" => model = Some(PathBuf::from(next_value(&mut iter, "--model")?)),
            "--whisper" => whisper = Some(PathBuf::from(next_value(&mut iter, "--whisper")?)),
            "--keep-srt" => keep_srt = true,
            unknown if unknown.starts_with('-') => {
                return Err(BurnerError::InvalidArguments {
                    message: format!("unknown option: {unknown}"),
                });
            }
            positional => {
                return Err(BurnerError::InvalidArguments {
                    message: format!("unsupported positional argument: {positional}"),
                });
            }
        }
    }

    let input = required_path(input, "--input")?;
    let output = required_path(output, "--output")?;
    let subtitle = subtitle.map(PathBuf::from);

    if subtitle.is_none() && !auto_subtitle {
        return Err(BurnerError::InvalidArguments {
            message: "missing --subtitle; use --auto-subtitle for speech recognition".to_string(),
        });
    }
    if subtitle.is_some() && auto_subtitle {
        return Err(BurnerError::InvalidArguments {
            message: "--subtitle and --auto-subtitle cannot be used together".to_string(),
        });
    }

    let auto_subtitle = if auto_subtitle {
        let mut asr = AsrOptions {
            language,
            keep_srt,
            ..AsrOptions::default()
        };
        if let Some(model) = model {
            asr.model_path = model;
        }
        if let Some(whisper) = whisper {
            asr.whisper_path = whisper;
        }
        Some(asr)
    } else {
        None
    };

    Ok(Command::Burn(Box::new(BurnOptions {
        input,
        subtitle,
        output,
        style: SubtitleStyle {
            font,
            font_size,
            shadow: !no_shadow,
        },
        auto_subtitle,
        verbose,
        dry_run,
    })))
}

fn parse_mosaic_args(args: Vec<String>) -> Result<Command> {
    let mut input = None;
    let mut output = None;
    let mut python = None;
    let mut ffmpeg = None;
    let mut script = None;
    let mut scale = 1.25_f32;
    let mut block_size = 18_u32;
    let mut min_size = 40_u32;
    let mut verbose = false;
    let mut dry_run = false;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-i" | "--input" => input = Some(next_value(&mut iter, "--input")?),
            "-o" | "--output" => output = Some(next_value(&mut iter, "--output")?),
            "--python" => python = Some(PathBuf::from(next_value(&mut iter, "--python")?)),
            "--ffmpeg" => ffmpeg = Some(PathBuf::from(next_value(&mut iter, "--ffmpeg")?)),
            "--script" => script = Some(PathBuf::from(next_value(&mut iter, "--script")?)),
            "--face-scale" => {
                let value = next_value(&mut iter, "--face-scale")?;
                scale = value
                    .parse::<f32>()
                    .map_err(|_| BurnerError::InvalidArguments {
                        message: "--face-scale must be a number".to_string(),
                    })?;
            }
            "--mosaic-block" => {
                let value = next_value(&mut iter, "--mosaic-block")?;
                block_size = value
                    .parse::<u32>()
                    .map_err(|_| BurnerError::InvalidArguments {
                        message: "--mosaic-block must be a positive integer".to_string(),
                    })?;
            }
            "--min-face" => {
                let value = next_value(&mut iter, "--min-face")?;
                min_size = value
                    .parse::<u32>()
                    .map_err(|_| BurnerError::InvalidArguments {
                        message: "--min-face must be a positive integer".to_string(),
                    })?;
            }
            "-v" | "--verbose" => verbose = true,
            "--dry-run" => dry_run = true,
            unknown if unknown.starts_with('-') => {
                return Err(BurnerError::InvalidArguments {
                    message: format!("unknown option: {unknown}"),
                });
            }
            positional => {
                return Err(BurnerError::InvalidArguments {
                    message: format!("unsupported positional argument: {positional}"),
                });
            }
        }
    }

    let input = required_path(input, "--input")?;
    let output = required_path(output, "--output")?;
    let mut options = MosaicOptions::with_defaults(input, output);
    if let Some(python) = python {
        options.python = python;
    }
    if let Some(ffmpeg) = ffmpeg {
        options.ffmpeg = ffmpeg;
    }
    if let Some(script) = script {
        options.script = script;
    }
    options.scale = scale;
    options.block_size = block_size;
    options.min_size = min_size;
    options.verbose = verbose;
    options.dry_run = dry_run;

    Ok(Command::Mosaic(Box::new(options)))
}

fn parse_companion_args(args: Vec<String>) -> Result<Command> {
    let mut input = None;
    let mut sticker = None;
    let mut output = None;
    let mut python = None;
    let mut ffmpeg = None;
    let mut script = None;
    let mut scale = 1.6_f32;
    let mut y_offset = 0.08_f32;
    let mut smooth = 0.72_f32;
    let mut min_size = 70_u32;
    let mut lost_frames = 12_u32;
    let mut verbose = false;
    let mut dry_run = false;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-i" | "--input" => input = Some(next_value(&mut iter, "--input")?),
            "-o" | "--output" => output = Some(next_value(&mut iter, "--output")?),
            "--sticker" => sticker = Some(next_value(&mut iter, "--sticker")?),
            "--python" => python = Some(PathBuf::from(next_value(&mut iter, "--python")?)),
            "--ffmpeg" => ffmpeg = Some(PathBuf::from(next_value(&mut iter, "--ffmpeg")?)),
            "--script" => script = Some(PathBuf::from(next_value(&mut iter, "--script")?)),
            "--scale" => {
                let value = next_value(&mut iter, "--scale")?;
                scale = value
                    .parse::<f32>()
                    .map_err(|_| BurnerError::InvalidArguments {
                        message: "--scale must be a number".to_string(),
                    })?;
            }
            "--y-offset" => {
                let value = next_value(&mut iter, "--y-offset")?;
                y_offset = value
                    .parse::<f32>()
                    .map_err(|_| BurnerError::InvalidArguments {
                        message: "--y-offset must be a number".to_string(),
                    })?;
            }
            "--smooth" => {
                let value = next_value(&mut iter, "--smooth")?;
                smooth = value
                    .parse::<f32>()
                    .map_err(|_| BurnerError::InvalidArguments {
                        message: "--smooth must be a number".to_string(),
                    })?;
            }
            "--min-face" => {
                let value = next_value(&mut iter, "--min-face")?;
                min_size = value
                    .parse::<u32>()
                    .map_err(|_| BurnerError::InvalidArguments {
                        message: "--min-face must be a positive integer".to_string(),
                    })?;
            }
            "--lost-frames" => {
                let value = next_value(&mut iter, "--lost-frames")?;
                lost_frames = value
                    .parse::<u32>()
                    .map_err(|_| BurnerError::InvalidArguments {
                        message: "--lost-frames must be a positive integer".to_string(),
                    })?;
            }
            "-v" | "--verbose" => verbose = true,
            "--dry-run" => dry_run = true,
            unknown if unknown.starts_with('-') => {
                return Err(BurnerError::InvalidArguments {
                    message: format!("unknown option: {unknown}"),
                });
            }
            positional => {
                return Err(BurnerError::InvalidArguments {
                    message: format!("unsupported positional argument: {positional}"),
                });
            }
        }
    }

    let input = required_path(input, "--input")?;
    let sticker = required_path(sticker, "--sticker")?;
    let output = required_path(output, "--output")?;
    let mut options = CompanionOptions::with_defaults(input, sticker, output);
    if let Some(python) = python {
        options.python = python;
    }
    if let Some(ffmpeg) = ffmpeg {
        options.ffmpeg = ffmpeg;
    }
    if let Some(script) = script {
        options.script = script;
    }
    options.scale = scale;
    options.y_offset = y_offset;
    options.smooth = smooth;
    options.min_size = min_size;
    options.lost_frames = lost_frames;
    options.verbose = verbose;
    options.dry_run = dry_run;

    Ok(Command::Companion(Box::new(options)))
}

fn next_value(iter: &mut impl Iterator<Item = String>, name: &str) -> Result<String> {
    iter.next().ok_or_else(|| BurnerError::InvalidArguments {
        message: format!("{name} is missing a value"),
    })
}

fn required_path(value: Option<String>, name: &str) -> Result<PathBuf> {
    value
        .map(PathBuf::from)
        .ok_or_else(|| BurnerError::InvalidArguments {
            message: format!("missing required argument {name}"),
        })
}

pub fn help_text() -> &'static str {
    concat!(
        "subtitle-burner 0.1.0\n",
        "Video pipeline CLI: subtitle burn-in, auto subtitles, and face mosaic.\n\n",
        "Usage:\n",
        "  subtitle-burner --input <INPUT> --subtitle <SUBTITLE> --output <OUTPUT> [OPTIONS]\n",
        "  subtitle-burner --input <INPUT> --output <OUTPUT> --auto-subtitle [OPTIONS]\n",
        "  subtitle-burner mosaic --input <INPUT> --output <OUTPUT> [OPTIONS]\n\n",
        "  subtitle-burner companion --input <INPUT> --sticker <PNG> --output <OUTPUT> [OPTIONS]\n\n",
        "Subtitle options:\n",
        "  -i, --input <INPUT>        Input video path\n",
        "  -s, --subtitle <SUBTITLE>  SRT subtitle path\n",
        "  -o, --output <OUTPUT>      Output video path\n",
        "      --auto-subtitle        Generate subtitles from speech\n",
        "      --language <LANG>      ASR language: auto, zh, en [default: auto]\n",
        "      --model <MODEL>        Whisper model path [default: models/ggml-small.bin]\n",
        "      --whisper <EXE>        whisper-cli path [default: tools/whisper/Release/whisper-cli.exe]\n",
        "      --keep-srt             Keep generated SRT file\n",
        "  -f, --font <FONT>          Font path for FFmpeg subtitles filter\n",
        "      --font-size <SIZE>     Subtitle font size\n",
        "      --no-shadow            Disable subtitle shadow\n",
        "      --dry-run              Print commands without executing\n\n",
        "Mosaic options:\n",
        "      --python <PYTHON>      Python executable [default: C:/software/Anaconda/envs/cv_env/python.exe]\n",
        "      --ffmpeg <FFMPEG>      FFmpeg executable path\n",
        "      --script <SCRIPT>      Processing script [default: scripts/face_mosaic.py]\n",
        "      --face-scale <N>       Face box expansion ratio [default: 1.25]\n",
        "      --mosaic-block <N>     Mosaic block size [default: 18]\n",
        "      --min-face <N>         Minimum face size [default: 40]\n",
        "\nCompanion options:\n",
        "      --sticker <PNG>        Transparent PNG sticker\n",
        "      --python <PYTHON>      Python executable [default: C:/software/Anaconda/envs/cv_env/python.exe]\n",
        "      --ffmpeg <FFMPEG>      FFmpeg executable path\n",
        "      --script <SCRIPT>      Processing script [default: scripts/companion_sticker.py]\n",
        "      --scale <N>            Sticker width = face width * N [default: 1.6]\n",
        "      --y-offset <N>         Bottom gap above face top, in face heights [default: 0.08]\n",
        "      --smooth <N>           Position smoothing 0..0.98 [default: 0.72]\n",
        "      --min-face <N>         Minimum face size [default: 70]\n",
        "      --lost-frames <N>      Reuse last position after detection loss [default: 12]\n",
        "  -v, --verbose              Show progress logs\n",
        "  -h, --help                 Show help\n",
        "  -V, --version              Show version\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_required_options() {
        let cmd = parse_args([
            "--input",
            "in.mp4",
            "--subtitle",
            "a.srt",
            "--output",
            "out.mp4",
            "--font-size",
            "28",
            "--no-shadow",
        ])
        .unwrap();

        match cmd {
            Command::Burn(options) => {
                assert_eq!(options.input, PathBuf::from("in.mp4"));
                assert_eq!(options.subtitle, Some(PathBuf::from("a.srt")));
                assert_eq!(options.output, PathBuf::from("out.mp4"));
                assert_eq!(options.style.font_size, Some(28));
                assert!(!options.style.shadow);
            }
            _ => panic!("expected burn command"),
        }
    }

    #[test]
    fn parse_auto_subtitle_options() {
        let cmd = parse_args([
            "--input",
            "in.mp4",
            "--output",
            "out.mp4",
            "--auto-subtitle",
            "--language",
            "zh",
            "--model",
            "models/custom.bin",
            "--keep-srt",
        ])
        .unwrap();

        match cmd {
            Command::Burn(options) => {
                let asr = options.auto_subtitle.unwrap();
                assert_eq!(options.subtitle, None);
                assert_eq!(asr.language, AsrLanguage::Chinese);
                assert_eq!(asr.model_path, PathBuf::from("models/custom.bin"));
                assert!(asr.keep_srt);
            }
            _ => panic!("expected burn command"),
        }
    }

    #[test]
    fn parse_mosaic_options() {
        let cmd = parse_args([
            "mosaic",
            "--input",
            "in.mp4",
            "--output",
            "out.mp4",
            "--face-scale",
            "1.4",
            "--mosaic-block",
            "24",
        ])
        .unwrap();

        match cmd {
            Command::Mosaic(options) => {
                assert_eq!(options.input, PathBuf::from("in.mp4"));
                assert_eq!(options.output, PathBuf::from("out.mp4"));
                assert_eq!(options.scale, 1.4);
                assert_eq!(options.block_size, 24);
            }
            _ => panic!("expected mosaic command"),
        }
    }

    #[test]
    fn parse_companion_options() {
        let cmd = parse_args([
            "companion",
            "--input",
            "in.mp4",
            "--sticker",
            "image/image.png",
            "--output",
            "out.mp4",
            "--scale",
            "1.8",
            "--y-offset",
            "0.12",
        ])
        .unwrap();

        match cmd {
            Command::Companion(options) => {
                assert_eq!(options.input, PathBuf::from("in.mp4"));
                assert_eq!(options.sticker, PathBuf::from("image/image.png"));
                assert_eq!(options.output, PathBuf::from("out.mp4"));
                assert_eq!(options.scale, 1.8);
                assert_eq!(options.y_offset, 0.12);
            }
            _ => panic!("expected companion command"),
        }
    }
}
