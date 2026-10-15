use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ccline")]
#[command(version, about = "High-performance Claude Code StatusLine")]
pub struct Cli {
    /// Render Claude Code's subagent panel rows from stdin
    #[arg(long, conflicts_with_all = ["config", "theme"])]
    pub subagent_statusline: bool,

    /// Enter TUI configuration mode
    #[arg(short = 'c', long = "config")]
    pub config: bool,

    /// Set theme
    #[arg(short = 't', long = "theme")]
    pub theme: Option<String>,
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
    fn subagent_statusline_is_a_standalone_flag() {
        let cli = Cli::try_parse_from(["ccline", "--subagent-statusline"]).unwrap();
        assert!(cli.subagent_statusline);
        for args in [
            vec!["ccline", "--subagent-statusline", "--config"],
            vec!["ccline", "--subagent-statusline", "--theme", "default"],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
    }

    #[test]
    fn removed_legacy_flags_are_rejected() {
        for args in [
            vec!["ccline", "--patch", "cli.js"],
            vec!["ccline", "--agents-hook"],
            vec!["ccline", "--width-offset", "8"],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
    }
}
