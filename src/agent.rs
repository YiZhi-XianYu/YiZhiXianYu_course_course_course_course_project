use std::path::{Path, PathBuf};

use crate::asr::{AsrLanguage, AsrOptions};
use crate::companion::{run_companion, CompanionOptions};
use crate::error::{BurnerError, Result};
use crate::face::{run_mosaic, MosaicOptions};
use crate::pipeline::{run_burn_pipeline, BurnOptions, SubtitleStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentGoal {
    Isekai,
    Privacy,
    Subtitle,
}

impl AgentGoal {
    pub fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "isekai" => Ok(Self::Isekai),
            "privacy" => Ok(Self::Privacy),
            "subtitle" | "subtitles" => Ok(Self::Subtitle),
            _ => Err(BurnerError::InvalidArguments {
                message: "agent goal must be one of: isekai, privacy, subtitle".to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub goal: AgentGoal,
    pub custom_steps: Option<Vec<PlannedStep>>,
    pub sticker: Option<PathBuf>,
    pub subtitle: Option<PathBuf>,
    pub language: AsrLanguage,
    pub keep_srt: bool,
    pub verbose: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentReport {
    pub output: PathBuf,
    pub steps: Vec<AgentStep>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStep {
    Companion { output: PathBuf },
    Mosaic { output: PathBuf },
    AutoSubtitleBurn { output: PathBuf },
    SubtitleBurn { output: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Perception {
    input_exists: bool,
    sticker_available: bool,
    subtitle_available: bool,
    wants_privacy: bool,
    wants_isekai: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannedStep {
    Companion,
    Mosaic,
    AutoSubtitleBurn,
    SubtitleBurn,
}

pub fn run_agent(options: AgentOptions) -> Result<AgentReport> {
    validate_agent_options(&options)?;
    let perception = perceive(&options);
    let plan = options
        .custom_steps
        .clone()
        .map(Ok)
        .unwrap_or_else(|| plan(&options, &perception))?;

    if options.verbose {
        eprintln!("[agent] perception: {:?}", perception);
        eprintln!("[agent] plan: {:?}", plan);
    }

    let mut current_input = options.input.clone();
    let mut completed = Vec::new();
    for (index, step) in plan.iter().enumerate() {
        let is_last = index + 1 == plan.len();
        let step_output = if is_last {
            options.output.clone()
        } else {
            temp_step_output(index, step)
        };

        match step {
            PlannedStep::Companion => {
                if options.dry_run && !current_input.is_file() {
                    eprintln!(
                        "[agent] dry-run: skip companion command because intermediate input is not generated: {}",
                        current_input.display()
                    );
                    completed.push(AgentStep::Companion {
                        output: step_output.clone(),
                    });
                    current_input = step_output.clone();
                    continue;
                }
                let sticker =
                    options
                        .sticker
                        .clone()
                        .ok_or_else(|| BurnerError::InvalidArguments {
                            message: "isekai agent requires --sticker for companion effect"
                                .to_string(),
                        })?;
                let mut companion = CompanionOptions::with_defaults(
                    current_input.clone(),
                    sticker,
                    step_output.clone(),
                );
                companion.verbose = options.verbose;
                companion.dry_run = options.dry_run;
                run_companion(companion)?;
                current_input = step_output.clone();
                completed.push(AgentStep::Companion {
                    output: step_output.clone(),
                });
            }
            PlannedStep::Mosaic => {
                if options.dry_run && !current_input.is_file() {
                    eprintln!(
                        "[agent] dry-run: skip mosaic command because intermediate input is not generated: {}",
                        current_input.display()
                    );
                    completed.push(AgentStep::Mosaic {
                        output: step_output.clone(),
                    });
                    current_input = step_output.clone();
                    continue;
                }
                let mut mosaic =
                    MosaicOptions::with_defaults(current_input.clone(), step_output.clone());
                mosaic.verbose = options.verbose;
                mosaic.dry_run = options.dry_run;
                run_mosaic(mosaic)?;
                current_input = step_output.clone();
                completed.push(AgentStep::Mosaic {
                    output: step_output.clone(),
                });
            }
            PlannedStep::AutoSubtitleBurn => {
                if options.dry_run && !current_input.is_file() {
                    eprintln!(
                        "[agent] dry-run: skip auto subtitle command because intermediate input is not generated: {}",
                        current_input.display()
                    );
                    completed.push(AgentStep::AutoSubtitleBurn {
                        output: step_output.clone(),
                    });
                    current_input = step_output.clone();
                    continue;
                }
                let burn = BurnOptions {
                    input: current_input.clone(),
                    subtitle: None,
                    output: step_output.clone(),
                    style: SubtitleStyle::default(),
                    auto_subtitle: Some(AsrOptions {
                        language: options.language,
                        keep_srt: options.keep_srt,
                        ..AsrOptions::default()
                    }),
                    verbose: options.verbose,
                    dry_run: options.dry_run,
                };
                run_burn_pipeline(burn)?;
                current_input = step_output.clone();
                completed.push(AgentStep::AutoSubtitleBurn {
                    output: step_output.clone(),
                });
            }
            PlannedStep::SubtitleBurn => {
                if options.dry_run && !current_input.is_file() {
                    eprintln!(
                        "[agent] dry-run: skip subtitle command because intermediate input is not generated: {}",
                        current_input.display()
                    );
                    completed.push(AgentStep::SubtitleBurn {
                        output: step_output.clone(),
                    });
                    current_input = step_output.clone();
                    continue;
                }
                let burn = BurnOptions {
                    input: current_input.clone(),
                    subtitle: options.subtitle.clone(),
                    output: step_output.clone(),
                    style: SubtitleStyle::default(),
                    auto_subtitle: None,
                    verbose: options.verbose,
                    dry_run: options.dry_run,
                };
                run_burn_pipeline(burn)?;
                current_input = step_output.clone();
                completed.push(AgentStep::SubtitleBurn {
                    output: step_output.clone(),
                });
            }
        }
    }

    Ok(AgentReport {
        output: options.output,
        steps: completed,
        dry_run: options.dry_run,
    })
}

fn validate_agent_options(options: &AgentOptions) -> Result<()> {
    if !options.input.is_file() {
        return Err(BurnerError::InputNotFound {
            path: options.input.clone(),
        });
    }
    if let Some(sticker) = &options.sticker {
        if !sticker.is_file() {
            return Err(BurnerError::InvalidArguments {
                message: format!("sticker image not found: {}", sticker.display()),
            });
        }
    }
    if let Some(subtitle) = &options.subtitle {
        if !subtitle.is_file() {
            return Err(BurnerError::SubtitleNotFound {
                path: subtitle.clone(),
            });
        }
    }
    if let Some(parent) = options.output.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(BurnerError::OutputDirectoryNotFound {
                path: parent.to_path_buf(),
            });
        }
    }
    Ok(())
}

fn perceive(options: &AgentOptions) -> Perception {
    Perception {
        input_exists: options.input.is_file(),
        sticker_available: options
            .sticker
            .as_ref()
            .is_some_and(|sticker| sticker.is_file()),
        subtitle_available: options
            .subtitle
            .as_ref()
            .is_some_and(|subtitle| subtitle.is_file()),
        wants_privacy: options.goal == AgentGoal::Privacy,
        wants_isekai: options.goal == AgentGoal::Isekai,
    }
}

fn plan(options: &AgentOptions, perception: &Perception) -> Result<Vec<PlannedStep>> {
    match options.goal {
        AgentGoal::Privacy => Ok(vec![PlannedStep::Mosaic]),
        AgentGoal::Subtitle => {
            if perception.subtitle_available {
                Ok(vec![PlannedStep::SubtitleBurn])
            } else {
                Ok(vec![PlannedStep::AutoSubtitleBurn])
            }
        }
        AgentGoal::Isekai => {
            let mut steps = Vec::new();
            if perception.sticker_available {
                steps.push(PlannedStep::Companion);
            }
            if perception.subtitle_available {
                steps.push(PlannedStep::SubtitleBurn);
            } else {
                steps.push(PlannedStep::AutoSubtitleBurn);
            }
            if steps.is_empty() {
                return Err(BurnerError::InvalidArguments {
                    message: "agent has no executable steps".to_string(),
                });
            }
            Ok(steps)
        }
    }
}

fn temp_step_output(index: usize, step: &PlannedStep) -> PathBuf {
    let name = match step {
        PlannedStep::Companion => "agent-companion",
        PlannedStep::Mosaic => "agent-mosaic",
        PlannedStep::AutoSubtitleBurn | PlannedStep::SubtitleBurn => "agent-subtitle",
    };
    std::env::temp_dir().join(format!("{name}-{index}-{}.mp4", std::process::id()))
}

#[allow(dead_code)]
fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_agent_goals() {
        assert_eq!(AgentGoal::parse("isekai").unwrap(), AgentGoal::Isekai);
        assert_eq!(AgentGoal::parse("privacy").unwrap(), AgentGoal::Privacy);
        assert_eq!(AgentGoal::parse("subtitle").unwrap(), AgentGoal::Subtitle);
        assert!(AgentGoal::parse("unknown").is_err());
    }

    #[test]
    fn plans_isekai_with_sticker_and_auto_subtitle() {
        let options = AgentOptions {
            input: "in.mp4".into(),
            output: "out.mp4".into(),
            goal: AgentGoal::Isekai,
            custom_steps: None,
            sticker: Some("sticker.png".into()),
            subtitle: None,
            language: AsrLanguage::Auto,
            keep_srt: false,
            verbose: false,
            dry_run: true,
        };
        let perception = Perception {
            input_exists: true,
            sticker_available: true,
            subtitle_available: false,
            wants_privacy: false,
            wants_isekai: true,
        };

        assert_eq!(
            plan(&options, &perception).unwrap(),
            vec![PlannedStep::Companion, PlannedStep::AutoSubtitleBurn]
        );
    }
}
