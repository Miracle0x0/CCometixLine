# CCometixLine

[English](README.md) | [中文](README.zh.md)

A high-performance Claude Code statusline tool written in Rust with Git integration, context and usage display, Claude Code subagent panel rows, and interactive TUI configuration.

![Language:Rust](https://img.shields.io/static/v1?label=Language&message=Rust&color=orange&style=flat-square)
![License:MIT](https://img.shields.io/static/v1?label=License&message=MIT&color=blue&style=flat-square)

## Screenshots

![CCometixLine](assets/img1.png)

The statusline shows: Model | Directory | Git Branch Status | Context Window Information

## Features

### Core Functionality
- **Git integration** with branch, status, and tracking info  
- **Model display** with simplified Claude model names
- **Effort display** with an independent toggle for the current reasoning effort level
- **Context and usage display** read directly from Claude Code's status line input
- **Directory display** showing current workspace
- **Minimal design** using Nerd Font icons

### Interactive TUI Features
- **Interactive main menu** when executed without input
- **TUI configuration interface** with real-time preview
- **Theme system** with multiple built-in presets
- **Segment customization** with granular control
- **Configuration management** (init, check, edit)

## Installation

### GitHub Release (Recommended)

Download the archive for your platform from [GitHub Releases](https://github.com/Miracle0x0/CCometixLine/releases/latest):

| Platform | Archive |
| --- | --- |
| Linux x64 (glibc) | `ccline-linux-x64.tar.gz` |
| Linux x64 (musl/static) | `ccline-linux-x64-static.tar.gz` |
| Linux ARM64 (glibc) | `ccline-linux-arm64.tar.gz` |
| Linux ARM64 (musl/static) | `ccline-linux-arm64-static.tar.gz` |
| macOS Intel | `ccline-macos-x64.tar.gz` |
| macOS Apple Silicon | `ccline-macos-arm64.tar.gz` |
| Windows x64 | `ccline-windows-x64.zip` |

#### Linux / macOS

Set `archive` to the filename from the table, then download and extract it:

```bash
archive=ccline-linux-x64.tar.gz
mkdir -p ~/.claude/ccline
curl -fL "https://github.com/Miracle0x0/CCometixLine/releases/latest/download/$archive" -o "$archive"
tar -xzf "$archive" --strip-components=1 -C ~/.claude/ccline
chmod +x ~/.claude/ccline/ccline
export PATH="$HOME/.claude/ccline:$PATH"
```

#### Windows

```powershell
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.claude\ccline"
Invoke-WebRequest -Uri "https://github.com/Miracle0x0/CCometixLine/releases/latest/download/ccline-windows-x64.zip" -OutFile "ccline-windows-x64.zip"
Expand-Archive -Path "ccline-windows-x64.zip" -DestinationPath . -Force
Copy-Item -Path ".\ccline-windows-x64\ccline.exe" -Destination "$env:USERPROFILE\.claude\ccline\ccline.exe" -Force
$env:PATH = "$env:USERPROFILE\.claude\ccline;$env:PATH"
```

These commands make `ccline` available in the current terminal. For future terminals, add `~/.claude/ccline` (Windows: `%USERPROFILE%\.claude\ccline`) to your user `PATH`.

### Claude Code Configuration

Add to your Claude Code `settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/ccline/ccline",
    "padding": 0
  }
}
```

Run `ccline -c` to open the configuration panel and select a theme.

### Update

Run the download and extraction commands for your platform again to replace the installed binary with the latest GitHub Release. The statusline's Update segment checks this repository's releases for newer versions.

### Build from Source

```bash
git clone https://github.com/Miracle0x0/CCometixLine.git
cd CCometixLine
cargo build --release
```

Linux / macOS:

```bash
mkdir -p ~/.claude/ccline
cp target/release/ccometixline ~/.claude/ccline/ccline
chmod +x ~/.claude/ccline/ccline
export PATH="$HOME/.claude/ccline:$PATH"
```

Windows (PowerShell):

```powershell
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.claude\ccline"
Copy-Item target\release\ccometixline.exe "$env:USERPROFILE\.claude\ccline\ccline.exe" -Force
$env:PATH = "$env:USERPROFILE\.claude\ccline;$env:PATH"
```

## Usage

### Theme Override

```bash
# Temporarily use specific theme (overrides config file)
ccline --theme cometix
ccline --theme minimal
ccline --theme gruvbox
ccline --theme nord
ccline --theme powerline-dark

# Or use custom theme files from the themes directory (see Configuration)
ccline --theme my-custom-theme
```

### Verbose Output and Removed Patcher

Earlier versions offered `ccline --patch` to modify Claude Code's `cli.js`. Claude Code now ships as a native binary with no `cli.js`, so the patcher has been removed. For verbose output, use Claude Code's own setting: run `/config verbose=true`, or toggle verbose output in the Config tab of `/config` ([official docs](https://code.claude.com/docs/en/settings#use-the-config-menu)). ccline no longer hides "Context low" warnings or changes any other Claude Code behavior; no official replacement for those patches has been confirmed.

## Default Segments

Displays: `Model | Effort | Directory | Git Branch Status | Context Window`

- **Git**: branch name; `✓` clean, `●` dirty, `⚠` conflicts; `↑n` ahead, `↓n` behind
- **Model**: simplified names such as `claude-sonnet-5-5` → `Sonnet 5.5`, customizable in `models.toml`
- **Effort**: the current reasoning effort level, such as `high`
- **Context Window**: Claude Code's reported context usage, such as `20% · 200k tokens`; `-` before the first response and right after `/compact`

Optional segments include **Usage** (subscription windows and gateway spend limits, such as `63% · 24% · 2h`), Session, Cost, Output Style, and Update. [Segments](docs/segments.md) describes each segment's data, formats, and options. To keep Usage countdowns current while the session is idle, set `refreshInterval` in your `statusLine` setting yourself; ccline does not manage it.

## Configuration

- **Interactive TUI**: `ccline --config` for real-time editing with preview
- **Files**: `<config dir>/ccline/config.toml`, `themes/*.toml`, and `models.toml`
- **Initialization**: **Initialize Config** in the main menu (run `ccline` without input)

`<config dir>` is [`CLAUDE_CONFIG_DIR`](https://code.claude.com/docs/en/env-vars) when set, otherwise `~/.claude`. Rendering only reads these files, and an unreadable or invalid file is reported as an error instead of falling back to defaults. See [Configuration](docs/configuration.md) for the directory rules, initialization, and `models.toml`.

## Subagent Panel

Enable **Agents** in `ccline --config` and press **S** to install `ccline --subagent-statusline` as Claude Code's [`subagentStatusLine`](https://code.claude.com/docs/en/statusline#subagent-status-lines). ccline then renders each row of the subagent panel below the prompt:

```text
Explore · find config loading · 1m20s · 12.4k (6%) · Sonnet 5.5 · high
```

An existing `subagentStatusLine` that ccline did not write is reported as a conflict and left unchanged. See [Subagent Panel](docs/subagent-panel.md) for the row fields, width handling, ownership rules, and migration from the removed hooks and Mod backends.

## Requirements

- **Git**: Version 1.5+ (Git 2.22+ recommended for better branch detection)
- **Terminal**: Must support Nerd Fonts for proper icon display
  - Install a [Nerd Font](https://www.nerdfonts.com/) (e.g., FiraCode Nerd Font, JetBrains Mono Nerd Font)
  - Configure your terminal to use the Nerd Font
- **Claude Code**: For statusline integration

## Development

```bash
# Build development version
cargo build

# Run tests
cargo test

# Build optimized release
cargo build --release
```

### Releases

Change `[package].version` in `Cargo.toml` and run `cargo check` to update `Cargo.lock`, then commit both files and push to `main` or `master`. The release workflow compares package versions before and after the push. A version change builds all seven platform archives and creates `v<version>` and its GitHub Release at the pushed commit. Dependency-only changes do not publish a release; no manual tag push is needed.

## Roadmap

- [x] TOML configuration file support
- [x] TUI configuration interface
- [x] Custom themes
- [x] Interactive main menu

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## Related Projects

- [tweakcc](https://github.com/Piebald-AI/tweakcc) - Command-line tool to customize your Claude Code themes, thinking verbs, and more.

## License

This project is licensed under the [MIT License](LICENSE).

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Haleclipse/CCometixLine&type=Date)](https://star-history.com/#Haleclipse/CCometixLine&Date)
