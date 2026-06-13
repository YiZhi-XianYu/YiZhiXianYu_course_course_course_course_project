use std::env;
use std::path::PathBuf;

use crate::error::{BurnerError, Result};
use crate::pipeline::{BurnOptions, SubtitleStyle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Burn(BurnOptions),
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
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    if args.is_empty() {
        return Ok(Command::Help);
    }

    let mut input = None;
    let mut subtitle = None;
    let mut output = None;
    let mut font = None;
    let mut font_size = None;
    let mut no_shadow = false;
    let mut verbose = false;
    let mut dry_run = false;

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
                let parsed = value
                    .parse::<u32>()
                    .map_err(|_| BurnerError::InvalidArguments {
                        message: "--font-size 必须是正整数".to_string(),
                    })?;
                font_size = Some(parsed);
            }
            "--no-shadow" => no_shadow = true,
            "-v" | "--verbose" => verbose = true,
            "--dry-run" => dry_run = true,
            unknown if unknown.starts_with('-') => {
                return Err(BurnerError::InvalidArguments {
                    message: format!("未知选项: {unknown}"),
                });
            }
            positional => {
                return Err(BurnerError::InvalidArguments {
                    message: format!("不支持的位置参数: {positional}"),
                });
            }
        }
    }

    let input = required_path(input, "--input")?;
    let subtitle = required_path(subtitle, "--subtitle")?;
    let output = required_path(output, "--output")?;
    let style = SubtitleStyle {
        font,
        font_size,
        shadow: !no_shadow,
    };

    Ok(Command::Burn(BurnOptions {
        input,
        subtitle,
        output,
        style,
        verbose,
        dry_run,
    }))
}

fn next_value(iter: &mut impl Iterator<Item = String>, name: &str) -> Result<String> {
    iter.next().ok_or_else(|| BurnerError::InvalidArguments {
        message: format!("{name} 缺少参数值"),
    })
}

fn required_path(value: Option<String>, name: &str) -> Result<PathBuf> {
    value
        .map(PathBuf::from)
        .ok_or_else(|| BurnerError::InvalidArguments {
            message: format!("缺少必需参数 {name}"),
        })
}

pub fn help_text() -> &'static str {
    concat!(
        "subtitle-burner 0.1.0\n",
        "小型视频处理流水线：将 SRT 字幕硬编码烧录到视频画面中。\n\n",
        "用法:\n",
        "  subtitle-burner --input <INPUT> --subtitle <SUBTITLE> --output <OUTPUT> [OPTIONS]\n\n",
        "选项:\n",
        "  -i, --input <INPUT>        输入视频文件路径\n",
        "  -s, --subtitle <SUBTITLE>  SRT 字幕文件路径\n",
        "  -o, --output <OUTPUT>      输出视频文件路径\n",
        "  -f, --font <FONT>          字体文件路径，用于 FFmpeg subtitles 滤镜\n",
        "      --font-size <SIZE>     指定字幕字号\n",
        "      --no-shadow            禁用字幕阴影\n",
        "      --dry-run              只打印 FFmpeg 命令，不真正执行\n",
        "  -v, --verbose              显示流水线阶段日志\n",
        "  -h, --help                 显示帮助\n",
        "  -V, --version              显示版本\n"
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
                assert_eq!(options.subtitle, PathBuf::from("a.srt"));
                assert_eq!(options.output, PathBuf::from("out.mp4"));
                assert_eq!(options.style.font_size, Some(28));
                assert!(!options.style.shadow);
            }
            _ => panic!("expected burn command"),
        }
    }
}
