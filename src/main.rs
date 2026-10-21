use ccometixline::cli::Cli;
use ccometixline::config::{Config, InputData, ModelConfig};
use ccometixline::core::{collect_all_segments, StatusLineGenerator};
use ccometixline::ui::{MainMenu, MenuResult};
use std::io::{self, IsTerminal};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse_args();

    if cli.config {
        ccometixline::ui::run_configurator()?;
        return Ok(());
    }

    // Check if stdin has data
    if io::stdin().is_terminal() {
        if let Some(result) = MainMenu::run()? {
            match result {
                MenuResult::LaunchConfigurator => {
                    ccometixline::ui::run_configurator()?;
                }
                MenuResult::InitConfig | MenuResult::CheckConfig => {}
                MenuResult::Exit => {}
            }
        }
        return Ok(());
    }

    // Rendering only reads configuration: both files are loaded once and shared.
    let config = match cli.theme {
        Some(theme) => ccometixline::ui::themes::ThemePresets::get_theme(&theme)?,
        None => Config::load()?,
    };
    let models = ModelConfig::load()?;

    if cli.subagent_statusline {
        use ccometixline::agents::{self, panel, summary};
        let input: panel::PanelInput = serde_json::from_reader(io::stdin().lock())?;
        let now = agents::now()?;
        if config.agents_enabled() {
            summary::save(&ccometixline::config::paths::claude_dir()?, &input, now)?;
        }
        for row in panel::rows(&input, &config, now, &models) {
            println!("{}", serde_json::to_string(&row)?);
        }
        return Ok(());
    }

    // Read Claude Code data from stdin
    let input: InputData = serde_json::from_reader(io::stdin().lock())?;
    let columns = terminal_columns()?;
    let segments_data = collect_all_segments(&config, &models, &input)?;
    let statusline = StatusLineGenerator::new(config).generate_with_width(segments_data, columns);
    if !statusline.is_empty() {
        println!("{statusline}");
    }

    Ok(())
}

/// Claude Code supplies the terminal width even though stdout is captured, not a TTY.
fn terminal_columns() -> Result<Option<usize>, Box<dyn std::error::Error>> {
    match std::env::var("COLUMNS") {
        Ok(value) => {
            let columns = value.parse::<std::num::NonZeroUsize>().map_err(|error| {
                format!("Invalid COLUMNS {value:?}: expected a positive integer ({error})")
            })?;
            Ok(Some(columns.get()))
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("Cannot read COLUMNS: {error}").into()),
    }
}
