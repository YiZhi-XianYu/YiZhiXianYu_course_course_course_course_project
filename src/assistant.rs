use std::path::PathBuf;

use crate::agent::{run_agent, AgentGoal, AgentOptions, AgentReport, PlannedStep};
use crate::asr::AsrLanguage;
use crate::error::{BurnerError, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct AssistantOptions {
    pub request: String,
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub sticker: Option<PathBuf>,
    pub subtitle: Option<PathBuf>,
    pub language: AsrLanguage,
    pub keep_srt: bool,
    pub verbose: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssistantPlan {
    pub goal: AgentGoal,
    pub steps: Vec<PlannedStep>,
    pub reason: String,
    pub output: PathBuf,
}

pub fn run_assistant(options: AssistantOptions) -> Result<AgentReport> {
    let plan = understand_request(&options)?;
    if options.verbose {
        eprintln!("[assistant] request: {}", options.request);
        eprintln!(
            "[assistant] intent: {:?}, steps: {:?} ({})",
            plan.goal, plan.steps, plan.reason
        );
    }

    run_agent(AgentOptions {
        input: options.input,
        output: plan.output,
        goal: plan.goal,
        custom_steps: Some(plan.steps),
        sticker: options.sticker,
        subtitle: options.subtitle,
        language: options.language,
        keep_srt: options.keep_srt,
        verbose: options.verbose,
        dry_run: options.dry_run,
    })
}

pub fn understand_request(options: &AssistantOptions) -> Result<AssistantPlan> {
    let text = normalize(&options.request);
    if text.trim().is_empty() {
        return misunderstood();
    }

    let mut matches = Vec::new();
    collect_intent(
        &mut matches,
        &text,
        PlannedStep::Mosaic,
        &["马赛克", "打码", "隐私", "匿名", "遮脸", "模糊人脸"],
    );
    collect_intent(
        &mut matches,
        &text,
        PlannedStep::Companion,
        &[
            "异世界",
            "二次元",
            "蜡笔小新",
            "贴纸",
            "头顶",
            "趴在",
            "伙伴",
            "挂件",
        ],
    );
    collect_intent(
        &mut matches,
        &text,
        subtitle_step(options),
        &["字幕", "识别语音", "语音识别", "自动识别", "烧录"],
    );

    if matches.is_empty() {
        return misunderstood();
    }

    matches.sort_by_key(|item| item.0);
    let mut steps = Vec::new();
    for (_, step) in matches {
        if !steps.contains(&step) {
            steps.push(step);
        }
    }

    if steps.contains(&PlannedStep::Companion) && options.sticker.is_none() {
        return Err(BurnerError::InvalidArguments {
            message: "异世界/贴纸任务需要提供 --sticker <PNG>".to_string(),
        });
    }

    let goal = infer_goal(&steps);
    let reason = format!("识别到 {} 个任务，按自然语言出现顺序执行", steps.len());
    Ok(AssistantPlan {
        goal,
        steps,
        reason,
        output: options
            .output
            .clone()
            .unwrap_or_else(|| default_output_for_goal(&options.input, goal)),
    })
}

fn collect_intent(
    matches: &mut Vec<(usize, PlannedStep)>,
    text: &str,
    step: PlannedStep,
    keywords: &[&str],
) {
    if let Some(index) = keywords
        .iter()
        .filter_map(|keyword| text.find(keyword))
        .min()
    {
        matches.push((index, step));
    }
}

fn subtitle_step(options: &AssistantOptions) -> PlannedStep {
    if options.subtitle.is_some() {
        PlannedStep::SubtitleBurn
    } else {
        PlannedStep::AutoSubtitleBurn
    }
}

fn infer_goal(steps: &[PlannedStep]) -> AgentGoal {
    if steps.contains(&PlannedStep::Mosaic) && steps.len() == 1 {
        AgentGoal::Privacy
    } else if steps.contains(&PlannedStep::Companion) {
        AgentGoal::Isekai
    } else {
        AgentGoal::Subtitle
    }
}

fn misunderstood<T>() -> Result<T> {
    Err(BurnerError::InvalidArguments {
        message: "抱歉没有听懂你想做什么".to_string(),
    })
}

fn normalize(text: &str) -> String {
    text.to_ascii_lowercase()
        .replace('，', ",")
        .replace('。', ".")
        .replace('！', "!")
        .replace('？', "?")
}

fn default_output_for_goal(input: &std::path::Path, goal: AgentGoal) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let suffix = match goal {
        AgentGoal::Isekai => "isekai",
        AgentGoal::Privacy => "privacy",
        AgentGoal::Subtitle => "subtitle",
    };
    input.with_file_name(format!("{stem}_{suffix}.mp4"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(request: &str) -> AssistantOptions {
        AssistantOptions {
            request: request.to_string(),
            input: "input.mp4".into(),
            output: None,
            sticker: Some("image/image.png".into()),
            subtitle: None,
            language: AsrLanguage::Auto,
            keep_srt: false,
            verbose: false,
            dry_run: true,
        }
    }

    #[test]
    fn understands_privacy_request() {
        let mut opts = options("请给所有人脸打码");
        opts.sticker = None;
        let plan = understand_request(&opts).unwrap();
        assert_eq!(plan.goal, AgentGoal::Privacy);
        assert_eq!(plan.steps, vec![PlannedStep::Mosaic]);
    }

    #[test]
    fn understands_isekai_request() {
        let plan = understand_request(&options("让蜡笔小新趴在人物头顶")).unwrap();
        assert_eq!(plan.goal, AgentGoal::Isekai);
        assert_eq!(plan.steps, vec![PlannedStep::Companion]);
    }

    #[test]
    fn understands_subtitle_request() {
        let mut opts = options("自动识别语音并烧录字幕");
        opts.sticker = None;
        let plan = understand_request(&opts).unwrap();
        assert_eq!(plan.goal, AgentGoal::Subtitle);
        assert_eq!(plan.steps, vec![PlannedStep::AutoSubtitleBurn]);
    }

    #[test]
    fn understands_multiple_tasks_in_order() {
        let plan = understand_request(&options("先给人脸打码，然后让蜡笔小新趴在头顶，再烧录字幕"))
            .unwrap();
        assert_eq!(
            plan.steps,
            vec![
                PlannedStep::Mosaic,
                PlannedStep::Companion,
                PlannedStep::AutoSubtitleBurn
            ]
        );
    }

    #[test]
    fn apologizes_when_no_task_is_understood() {
        let err = understand_request(&options("今天晚上吃什么")).unwrap_err();
        assert!(err.to_string().contains("抱歉没有听懂你想做什么"));
    }
}
