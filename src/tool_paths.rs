use std::path::PathBuf;

pub fn ffmpeg_path() -> PathBuf {
    let winget = PathBuf::from(
        "C:/Users/XRZ/AppData/Local/Microsoft/WinGet/Packages/Gyan.FFmpeg_Microsoft.Winget.Source_8wekyb3d8bbwe/ffmpeg-8.1.1-full_build/bin/ffmpeg.exe",
    );
    if winget.is_file() {
        winget
    } else {
        PathBuf::from("ffmpeg")
    }
}
