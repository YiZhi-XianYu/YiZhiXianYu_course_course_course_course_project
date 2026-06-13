use crate::error::{BurnerError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleEntry {
    pub index: u32,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleTrack {
    entries: Vec<SubtitleEntry>,
}

impl SubtitleTrack {
    pub fn new(mut entries: Vec<SubtitleEntry>) -> Self {
        entries.sort_by_key(|entry| (entry.start_ms, entry.end_ms, entry.index));
        Self { entries }
    }

    pub fn entries(&self) -> &[SubtitleEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn at(&self, pts_ms: i64) -> Option<&SubtitleEntry> {
        let idx = self
            .entries
            .partition_point(|entry| entry.start_ms <= pts_ms);
        if idx == 0 {
            return None;
        }

        self.entries[..idx]
            .iter()
            .rev()
            .find(|entry| pts_ms >= entry.start_ms && pts_ms <= entry.end_ms)
    }
}

pub fn parse_srt(content: &str) -> Result<Vec<SubtitleEntry>> {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let mut entries = Vec::new();
    let mut cursor = 0;

    while cursor < lines.len() {
        while cursor < lines.len() && lines[cursor].trim().is_empty() {
            cursor += 1;
        }
        if cursor >= lines.len() {
            break;
        }

        let index_line_no = cursor + 1;
        let index_text = strip_bom(lines[cursor].trim());
        let index = index_text
            .parse::<u32>()
            .map_err(|_| BurnerError::SrtParseError {
                line: index_line_no,
                reason: "字幕序号必须是正整数".to_string(),
            })?;
        cursor += 1;

        if cursor >= lines.len() {
            return Err(BurnerError::SrtParseError {
                line: index_line_no,
                reason: "缺少时间轴行".to_string(),
            });
        }

        let time_line_no = cursor + 1;
        let (start_ms, end_ms) = parse_time_range(lines[cursor], time_line_no)?;
        if end_ms < start_ms {
            return Err(BurnerError::SrtParseError {
                line: time_line_no,
                reason: "结束时间早于开始时间".to_string(),
            });
        }
        cursor += 1;

        let mut text_lines = Vec::new();
        while cursor < lines.len() && !lines[cursor].trim().is_empty() {
            text_lines.push(strip_html_tags(lines[cursor].trim()));
            cursor += 1;
        }

        if text_lines.is_empty() {
            return Err(BurnerError::SrtParseError {
                line: time_line_no,
                reason: "字幕文本不能为空".to_string(),
            });
        }

        entries.push(SubtitleEntry {
            index,
            start_ms,
            end_ms,
            text: text_lines.join("\n"),
        });
    }

    if entries.is_empty() {
        return Err(BurnerError::SrtParseError {
            line: 1,
            reason: "文件中没有可解析的字幕条目".to_string(),
        });
    }

    Ok(entries)
}

fn parse_time_range(line: &str, line_no: usize) -> Result<(i64, i64)> {
    let (start, end) = line
        .split_once("-->")
        .ok_or_else(|| BurnerError::SrtParseError {
            line: line_no,
            reason: "时间轴行必须包含 -->".to_string(),
        })?;
    Ok((
        parse_timestamp(start.trim(), line_no)?,
        parse_timestamp(end.trim(), line_no)?,
    ))
}

fn parse_timestamp(value: &str, line_no: usize) -> Result<i64> {
    let mut hms = value.split(':');
    let hour = parse_component(hms.next(), line_no, "小时")?;
    let minute = parse_component(hms.next(), line_no, "分钟")?;
    let second_ms = hms.next().ok_or_else(|| BurnerError::SrtParseError {
        line: line_no,
        reason: "时间戳格式应为 HH:MM:SS,mmm".to_string(),
    })?;
    if hms.next().is_some() {
        return Err(BurnerError::SrtParseError {
            line: line_no,
            reason: "时间戳包含过多冒号".to_string(),
        });
    }

    let (second_text, millis_text) =
        second_ms
            .split_once(',')
            .ok_or_else(|| BurnerError::SrtParseError {
                line: line_no,
                reason: "时间戳毫秒部分必须使用逗号分隔".to_string(),
            })?;
    let second = second_text
        .parse::<i64>()
        .map_err(|_| BurnerError::SrtParseError {
            line: line_no,
            reason: "秒必须是数字".to_string(),
        })?;
    let millis = millis_text
        .parse::<i64>()
        .map_err(|_| BurnerError::SrtParseError {
            line: line_no,
            reason: "毫秒必须是数字".to_string(),
        })?;

    if minute >= 60 || second >= 60 || millis >= 1000 || millis_text.len() != 3 {
        return Err(BurnerError::SrtParseError {
            line: line_no,
            reason: "时间戳范围非法".to_string(),
        });
    }

    Ok(((hour * 60 + minute) * 60 + second) * 1000 + millis)
}

fn parse_component(value: Option<&str>, line_no: usize, name: &str) -> Result<i64> {
    value
        .ok_or_else(|| BurnerError::SrtParseError {
            line: line_no,
            reason: format!("缺少{name}字段"),
        })?
        .parse::<i64>()
        .map_err(|_| BurnerError::SrtParseError {
            line: line_no,
            reason: format!("{name}必须是数字"),
        })
}

fn strip_bom(value: &str) -> &str {
    value.strip_prefix('\u{feff}').unwrap_or(value)
}

pub fn strip_html_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_tag = false;

    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }

    decode_entities(output.trim())
}

fn decode_entities(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_timestamp_to_millis() {
        assert_eq!(parse_timestamp("01:02:03,456", 1).unwrap(), 3_723_456);
    }

    #[test]
    fn finds_active_subtitle_by_binary_search() {
        let track = SubtitleTrack::new(vec![
            SubtitleEntry {
                index: 1,
                start_ms: 1000,
                end_ms: 2000,
                text: "one".to_string(),
            },
            SubtitleEntry {
                index: 2,
                start_ms: 3000,
                end_ms: 4000,
                text: "two".to_string(),
            },
        ]);

        assert_eq!(track.at(1500).map(|entry| entry.text.as_str()), Some("one"));
        assert_eq!(track.at(2500), None);
        assert_eq!(track.at(4000).map(|entry| entry.text.as_str()), Some("two"));
    }

    #[test]
    fn strips_tags_and_decodes_basic_entities() {
        assert_eq!(
            strip_html_tags("<i>Hello</i> &amp; <b>Rust</b>"),
            "Hello & Rust"
        );
    }
}
