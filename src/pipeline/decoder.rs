use std::fs;

use crate::error::Result;

use super::{BurnOptions, VideoPacket};

pub fn decode_request(options: &BurnOptions) -> Result<VideoPacket> {
    let subtitle_text = fs::read_to_string(&options.subtitle)?;
    Ok(VideoPacket {
        input: options.input.clone(),
        subtitle: options.subtitle.clone(),
        output: options.output.clone(),
        subtitle_text,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::pipeline::SubtitleStyle;

    #[test]
    fn reads_subtitle_text_into_packet() {
        let mut subtitle = std::env::temp_dir();
        subtitle.push(format!(
            "subtitle_burner_decoder_{}.srt",
            std::process::id()
        ));
        fs::write(&subtitle, "1\n00:00:00,000 --> 00:00:01,000\nhi\n").unwrap();

        let options = BurnOptions {
            input: PathBuf::from("input.mp4"),
            subtitle: subtitle.clone(),
            output: PathBuf::from("output.mp4"),
            style: SubtitleStyle::default(),
            verbose: false,
            dry_run: true,
        };

        let packet = decode_request(&options).unwrap();
        assert!(packet.subtitle_text.contains("hi"));

        let _ = fs::remove_file(subtitle);
    }
}
