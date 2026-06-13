use std::process::ExitCode;

use subtitle_burner::cli::{help_text, parse_env, Command};
use subtitle_burner::pipeline::run_burn_pipeline;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("错误: {err}");
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
                "处理完成: {} 条字幕，输出 {}",
                report.subtitle_count,
                report.output.display()
            );
            if let Some(srt) = report.generated_srt {
                println!("自动生成字幕已保存: {}", srt.display());
            }
            if report.dry_run {
                println!("dry-run 模式未生成实际视频。");
            }
            Ok(())
        }
    }
}
