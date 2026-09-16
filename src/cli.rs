use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ccline")]
#[command(version, about = "High-performance Claude Code StatusLine")]
pub struct Cli {
    /// Record a Claude Code subagent lifecycle hook from stdin
    #[arg(long, conflicts_with_all = ["config", "theme", "patch"])]
    pub agents_hook: bool,

    /// Columns reserved by Claude Code: 4 outer columns plus twice statusLine.padding
    #[arg(long, default_value_t = 4)]
    pub width_offset: usize,

    /// Enter TUI configuration mode
    #[arg(short = 'c', long = "config")]
    pub config: bool,

    /// Set theme
    #[arg(short = 't', long = "theme")]
    pub theme: Option<String>,

    /// Patch Claude Code cli.js to disable context warnings
    #[arg(long = "patch")]
    pub patch: Option<String>,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_is_a_standalone_flag_and_conflicts_with_other_modes() {
        let cli = Cli::try_parse_from(["ccline", "--agents-hook"]).unwrap();
        assert!(cli.agents_hook);
        for args in [
            vec!["ccline", "--agents-hook", "1780000000000000000"],
            vec![
                "ccline",
                "--agents-hook",
                "--installation-id",
                "20260916T063000.123456789Z",
            ],
            vec!["ccline", "--agents-hook", "--config"],
            vec!["ccline", "--agents-hook", "--theme", "default"],
            vec!["ccline", "--agents-hook", "--patch", "cli.js"],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
    }
}
