use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, BurnerError>;

#[derive(Debug)]
pub enum BurnerError {
    InputNotFound { path: PathBuf },
    SubtitleNotFound { path: PathBuf },
    OutputDirectoryNotFound { path: PathBuf },
    InvalidArguments { message: String },
    SrtParseError { line: usize, reason: String },
    FfmpegNotFound,
    FfmpegFailed { code: Option<i32>, stderr: String },
    PipelineClosed { stage: &'static str },
    ThreadPanicked { stage: &'static str },
    Io(io::Error),
}

impl Display for BurnerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputNotFound { path } => {
                write!(f, "无法打开输入视频文件: {}", path.display())
            }
            Self::SubtitleNotFound { path } => {
                write!(f, "无法打开 SRT 字幕文件: {}", path.display())
            }
            Self::OutputDirectoryNotFound { path } => {
                write!(f, "输出目录不存在: {}", path.display())
            }
            Self::InvalidArguments { message } => write!(f, "参数错误: {message}"),
            Self::SrtParseError { line, reason } => {
                write!(f, "SRT 解析失败（第 {line} 行）: {reason}")
            }
            Self::FfmpegNotFound => write!(
                f,
                "未找到 ffmpeg 可执行文件。请先安装 FFmpeg，并确认 ffmpeg 已加入 PATH"
            ),
            Self::FfmpegFailed { code, stderr } => {
                write!(f, "FFmpeg 执行失败，退出码: {code:?}\n{stderr}")
            }
            Self::PipelineClosed { stage } => write!(f, "流水线阶段提前关闭: {stage}"),
            Self::ThreadPanicked { stage } => write!(f, "流水线线程异常退出: {stage}"),
            Self::Io(err) => write!(f, "IO 错误: {err}"),
        }
    }
}

impl Error for BurnerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for BurnerError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
