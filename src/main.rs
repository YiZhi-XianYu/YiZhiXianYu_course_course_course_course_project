use std::process::ExitCode;

use subtitle_burner::agent::run_agent;
use subtitle_burner::assistant::run_assistant;
use subtitle_burner::cli::{help_text, parse_env, Command};
use subtitle_burner::companion::run_companion;
use subtitle_burner::face::run_mosaic;
use subtitle_burner::pipeline::run_burn_pipeline;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> subtitle_burner::Result<()> {
    match parse_env()? {
        Command::Help => {
            print!("{}", help_text());
            Ok(())
        }
        Command::Version => {
            println!("subtitle-burner 0.1.0");
            Ok(())
        }
        Command::Burn(options) => {
            let report = run_burn_pipeline(*options)?;
            println!(
                "subtitle burn complete: {} subtitles, output {}",
                report.subtitle_count,
                report.output.display()
            );
            if let Some(srt) = report.generated_srt {
                println!("generated SRT saved: {}", srt.display());
            }
            if report.dry_run {
                println!("dry-run mode did not generate a video.");
            }
            Ok(())
        }
        Command::Agent(options) => {
            let report = run_agent(*options)?;
            println!(
                "agent workflow complete: {} steps, output {}",
                report.steps.len(),
                report.output.display()
            );
            for (index, step) in report.steps.iter().enumerate() {
                println!("  step {}: {:?}", index + 1, step);
            }
            if report.dry_run {
                println!("dry-run mode did not generate a video.");
            }
            Ok(())
        }
        Command::Assistant(options) => {
            let report = run_assistant(*options)?;
            println!(
                "assistant workflow complete: {} steps, output {}",
                report.steps.len(),
                report.output.display()
            );
            for (index, step) in report.steps.iter().enumerate() {
                println!("  step {}: {:?}", index + 1, step);
            }
            if report.dry_run {
                println!("dry-run mode did not generate a video.");
            }
            Ok(())
        }
        Command::Mosaic(options) => {
            let report = run_mosaic(*options)?;
            println!("mosaic complete: output {}", report.output.display());
            if report.dry_run {
                println!("dry-run mode did not generate a video.");
            }
            Ok(())
        }
        Command::Companion(options) => {
            let report = run_companion(*options)?;
            println!("companion complete: output {}", report.output.display());
            if report.dry_run {
                println!("dry-run mode did not generate a video.");
            }
            Ok(())
        }
    }
}
