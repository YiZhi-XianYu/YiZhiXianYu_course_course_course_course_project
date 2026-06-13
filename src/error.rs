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
    WhisperNotFound { path: PathBuf },
    WhisperFailed { code: Option<i32>, stderr: String },
    AsrOutputMissing { path: PathBuf },
    PythonNotFound { path: PathBuf },
    PythonFailed { code: Option<i32>, stderr: String },
    PipelineClosed { stage: &'static str },
    ThreadPanicked { stage: &'static str },
    Io(io::Error),
}

impl Display for BurnerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputNotFound { path } => {
                write!(f, "input video file not found: {}", path.display())
            }
            Self::SubtitleNotFound { path } => {
                write!(f, "SRT subtitle file not found: {}", path.display())
            }
            Self::OutputDirectoryNotFound { path } => {
                write!(f, "output directory does not exist: {}", path.display())
            }
            Self::InvalidArguments { message } => write!(f, "invalid arguments: {message}"),
            Self::SrtParseError { line, reason } => {
                write!(f, "SRT parse failed at line {line}: {reason}")
            }
            Self::FfmpegNotFound => write!(
                f,
                "ffmpeg executable was not found. Install FFmpeg and make sure it is in PATH"
            ),
            Self::FfmpegFailed { code, stderr } => {
                write!(f, "FFmpeg failed with exit code {code:?}\n{stderr}")
            }
            Self::WhisperNotFound { path } => {
                write!(f, "whisper.cpp executable not found: {}", path.display())
            }
            Self::WhisperFailed { code, stderr } => {
                write!(f, "whisper.cpp failed with exit code {code:?}\n{stderr}")
            }
            Self::AsrOutputMissing { path } => {
                write!(f, "ASR did not generate an SRT file: {}", path.display())
            }
            Self::PythonNotFound { path } => {
                write!(f, "Python executable not found: {}", path.display())
            }
            Self::PythonFailed { code, stderr } => {
                write!(
                    f,
                    "Python image processing script failed with exit code {code:?}\n{stderr}"
                )
            }
            Self::PipelineClosed { stage } => write!(f, "pipeline stage closed early: {stage}"),
            Self::ThreadPanicked { stage } => write!(f, "pipeline thread panicked: {stage}"),
            Self::Io(err) => write!(f, "IO error: {err}"),
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
