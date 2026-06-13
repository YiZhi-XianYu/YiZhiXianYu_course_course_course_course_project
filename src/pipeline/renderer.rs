use std::path::{Path, PathBuf};

use crate::error::Result;

use super::SubtitleStyle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPlan {
    pub filter: String,
    pub subtitle_path: PathBuf,
    pub style: SubtitleStyle,
}

#[derive(Debug, Clone)]
pub struct Renderer {
    style: SubtitleStyle,
}

impl Renderer {
    pub fn new(style: SubtitleStyle) -> Self {
        Self { style }
    }

    pub fn plan(&self, subtitle_path: &Path) -> Result<RenderPlan> {
        Ok(RenderPlan {
            filter: build_subtitles_filter(subtitle_path, &self.style),
            subtitle_path: subtitle_path.to_path_buf(),
            style: self.style.clone(),
        })
    }
}

pub fn build_subtitles_filter(subtitle_path: &Path, style: &SubtitleStyle) -> String {
    let mut filter = format!("subtitles='{}'", escape_filter_path(subtitle_path));
    let mut force_style = Vec::new();

    if let Some(font_size) = style.font_size {
        force_style.push(format!("Fontsize={font_size}"));
    }
    if style.shadow {
        force_style.push("Shadow=1".to_string());
        force_style.push("Outline=1".to_string());
    } else {
        force_style.push("Shadow=0".to_string());
        force_style.push("Outline=0".to_string());
    }

    if let Some(font) = &style.font {
        if let Some(font_name) = font.file_stem().and_then(|value| value.to_str()) {
            force_style.push(format!("FontName={}", escape_force_style_value(font_name)));
        }
    }

    if !force_style.is_empty() {
        filter.push_str(":force_style='");
        filter.push_str(&force_style.join(","));
        filter.push('\'');
    }

    filter
}

fn escape_filter_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .replace(':', "\\:")
        .replace('\'', "\\'")
        .replace(',', "\\,")
}

fn escape_force_style_value(value: &str) -> String {
    value.replace('\'', "").replace([',', ':'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_filter_with_style() {
        let style = SubtitleStyle {
            font: Some(PathBuf::from("assets/NotoSansCJK-Regular.ttf")),
            font_size: Some(32),
            shadow: false,
        };
        let filter = build_subtitles_filter(Path::new("tests/test.srt"), &style);

        assert!(filter.contains("subtitles='tests/test.srt'"));
        assert!(filter.contains("Fontsize=32"));
        assert!(filter.contains("Shadow=0"));
        assert!(filter.contains("FontName=NotoSansCJK-Regular"));
    }
}
