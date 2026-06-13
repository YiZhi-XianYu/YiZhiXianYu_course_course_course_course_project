use std::fs;

use crate::error::Result;

use super::{BurnOptions, VideoPacket};

pub fn decode_request(options: &BurnOptions) -> Result<VideoPacket> {
    let (subtitle, subtitle_text, generated_srt) = if let Some(asr) = &options.auto_subtitle {
        let result = crate::asr::transcribe_video_to_srt(
            &options.input,
            &options.output,
            asr,
            options.dry_run,
            options.verbose,
        )?;
        let subtitle_path = result.kept_srt.clone().unwrap_or(result.srt_path);
        (subtitle_path, result.srt_text, result.kept_srt)
    } else {
        let subtitle = options
            .subtitle
            .clone()
            .expect("subtitle path is validated before decoder starts");
        let subtitle_text = fs::read_to_string(&subtitle)?;
        (subtitle, subtitle_text, None)
    };

    Ok(VideoPacket {
        input: options.input.clone(),
        subtitle,
        output: options.output.clone(),
        subtitle_text,
        generated_srt,
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
            subtitle: Some(subtitle.clone()),
            output: PathBuf::from("output.mp4"),
            style: SubtitleStyle::default(),
            auto_subtitle: None,
            verbose: false,
            dry_run: true,
        };

        let packet = decode_request(&options).unwrap();
        assert!(packet.subtitle_text.contains("hi"));

        let _ = fs::remove_file(subtitle);
    }
}
