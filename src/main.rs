use ccometixline::cli::Cli;
use ccometixline::config::{Config, InputData};
use ccometixline::core::{collect_all_segments, StatusLineGenerator};
use ccometixline::ui::{MainMenu, MenuResult};
use std::io::{self, IsTerminal};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse_args();

    if cli.agents_hook {
        use ccometixline::agents::{self, store::ActivityStore, HookInput};
        let input: HookInput = serde_json::from_reader(io::stdin().lock())?;
        ActivityStore::new(
            agents::data_dir()?
                .join("agents")
                .join(cli.installation_id.as_ref().unwrap().to_string()),
        )
        .record(input, agents::store::now()?)?;
        return Ok(());
    }

    if cli.config {
        ccometixline::ui::run_configurator()?;
        return Ok(());
    }

    // Handle Claude Code patcher
    if let Some(claude_path) = cli.patch {
        use ccometixline::utils::ClaudeCodePatcher;

        println!("🔧 Claude Code Context Warning Disabler");
        println!("Target file: {}", claude_path);

        // Create backup in same directory
        let backup_path = format!("{}.backup", claude_path);
        std::fs::copy(&claude_path, &backup_path)?;
        println!("📦 Created backup: {}", backup_path);

        // Load and patch
        let mut patcher = ClaudeCodePatcher::new(&claude_path)?;

        println!("\n🔄 Applying patches...");
        let results = patcher.apply_all_patches();
        patcher.save()?;

        ClaudeCodePatcher::print_summary(&results);
        println!("💡 To restore warnings, replace your cli.js with the backup file:");
        println!("   cp {} {}", backup_path, claude_path);

        return Ok(());
    }

    // Load configuration
    let mut config = Config::load()?;

    // Apply theme override if provided
    if let Some(theme) = cli.theme {
        config = ccometixline::ui::themes::ThemePresets::get_theme(&theme);
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

    // Read Claude Code data from stdin
    let stdin = io::stdin();
    let input: InputData = serde_json::from_reader(stdin.lock())?;

    // Collect segment data
    let segments_data = collect_all_segments(&config, &input)?;

    // Render statusline
    let generator = StatusLineGenerator::new(config);
    let width = if segments_data.iter().any(|(_, data)| {
        matches!(data, ccometixline::core::segments::SegmentContent::Agents(agents) if agents.max_agents > 0)
    }) {
        Some(ccometixline::core::statusline::available_width(
            &std::env::var("COLUMNS")
                .map_err(|e| format!("Cannot read status line width from COLUMNS: {e}"))?,
            cli.width_offset,
        )?)
    } else {
        None
    };
    let statusline = generator.generate_with_width(segments_data, width);

    println!("{}", statusline);

    Ok(())
}
