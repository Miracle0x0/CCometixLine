# CCometixLine

[English](README.md) | [中文](README.zh.md)

A high-performance Claude Code statusline tool written in Rust with Git integration, usage tracking, interactive TUI configuration, and Claude Code enhancement utilities.

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
- **Usage tracking** based on transcript analysis
- **Directory display** showing current workspace
- **Minimal design** using Nerd Font icons

### Interactive TUI Features
- **Interactive main menu** when executed without input
- **TUI configuration interface** with real-time preview
- **Theme system** with multiple built-in presets
- **Segment customization** with granular control
- **Configuration management** (init, check, edit)

### Claude Code Enhancement
- **Context warning disabler** - Remove annoying "Context low" messages
- **Verbose mode enabler** - Enhanced output detail
- **Robust patcher** - Survives Claude Code version updates
- **Automatic backups** - Safe modification with easy recovery

## Installation

### Quick Install (Recommended)

Install via npm (works on all platforms):

```bash
# Install globally
npm install -g @cometix/ccline

# Or using yarn
yarn global add @cometix/ccline

# Or using pnpm
pnpm add -g @cometix/ccline
```

Use npm mirror for faster download:
```bash
npm install -g @cometix/ccline --registry https://registry.npmmirror.com
```

After installation:
- ✅ Global command `ccline` is available everywhere
- ⚙️ Follow the configuration steps below to integrate with Claude Code
- 🎨 Run `ccline -c` to open configuration panel for theme selection

### Claude Code Configuration

Add to your Claude Code `settings.json`:

**Cross-Platform (Recommended)**
```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/ccline/ccline",
    "padding": 0
  }
}
```

> **Note for Windows users:** Starting from Claude Code v2.1.47+, Unix-style path parsing is supported on Windows. The `~` symbol is automatically expanded to your user home directory. **Do not use `%USERPROFILE%`** - it no longer works reliably in v2.1.47+.
> - Recommended: `~/.claude/ccline/ccline` (works on all platforms)
> - Alternative: `"ccline"` (requires npm global installation)

**Fallback (npm installation):**
```json
{
  "statusLine": {
    "type": "command",
    "command": "ccline",
    "padding": 0
  }
}
```
*Use this if npm global installation is available in PATH*

### Update

```bash
npm update -g @cometix/ccline
```

<details>
<summary>Manual Installation (Click to expand)</summary>

Alternatively, download from [Releases](https://github.com/Haleclipse/CCometixLine/releases):

#### Linux

#### Option 1: Dynamic Binary (Recommended)
```bash
mkdir -p ~/.claude/ccline
wget https://github.com/Haleclipse/CCometixLine/releases/latest/download/ccline-linux-x64.tar.gz
tar -xzf ccline-linux-x64.tar.gz
cp ccline ~/.claude/ccline/
chmod +x ~/.claude/ccline/ccline
```
*Requires: Ubuntu 22.04+, CentOS 9+, Debian 11+, RHEL 9+ (glibc 2.35+)*

#### Option 2: Static Binary (Universal Compatibility)
```bash
mkdir -p ~/.claude/ccline
wget https://github.com/Haleclipse/CCometixLine/releases/latest/download/ccline-linux-x64-static.tar.gz
tar -xzf ccline-linux-x64-static.tar.gz
cp ccline ~/.claude/ccline/
chmod +x ~/.claude/ccline/ccline
```
*Works on any Linux distribution (static, no dependencies)*

#### macOS (Intel)

```bash  
mkdir -p ~/.claude/ccline
wget https://github.com/Haleclipse/CCometixLine/releases/latest/download/ccline-macos-x64.tar.gz
tar -xzf ccline-macos-x64.tar.gz
cp ccline ~/.claude/ccline/
chmod +x ~/.claude/ccline/ccline
```

#### macOS (Apple Silicon)

```bash
mkdir -p ~/.claude/ccline  
wget https://github.com/Haleclipse/CCometixLine/releases/latest/download/ccline-macos-arm64.tar.gz
tar -xzf ccline-macos-arm64.tar.gz
cp ccline ~/.claude/ccline/
chmod +x ~/.claude/ccline/ccline
```

#### Windows

```powershell
# Create directory and download
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.claude\ccline"
Invoke-WebRequest -Uri "https://github.com/Haleclipse/CCometixLine/releases/latest/download/ccline-windows-x64.zip" -OutFile "ccline-windows-x64.zip"
Expand-Archive -Path "ccline-windows-x64.zip" -DestinationPath "."
Move-Item "ccline.exe" "$env:USERPROFILE\.claude\ccline\"
```

</details>

### Build from Source

```bash
git clone https://github.com/Haleclipse/CCometixLine.git
cd CCometixLine
cargo build --release

# Linux/macOS
mkdir -p ~/.claude/ccline
cp target/release/ccometixline ~/.claude/ccline/ccline
chmod +x ~/.claude/ccline/ccline

# Windows (PowerShell)
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.claude\ccline"
copy target\release\ccometixline.exe "$env:USERPROFILE\.claude\ccline\ccline.exe"
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

# Or use custom theme files from ~/.claude/ccline/themes/
ccline --theme my-custom-theme
```

### Claude Code Enhancement

```bash
# Disable context warnings and enable verbose mode
ccline --patch /path/to/claude-code/cli.js

# Example for common installation
ccline --patch ~/.local/share/fnm/node-versions/v24.4.1/installation/lib/node_modules/@anthropic-ai/claude-code/cli.js
```

## Default Segments

Displays: `Model | Effort | Directory | Git Branch Status | Context Window`

### Git Status Indicators

- Branch name with Nerd Font icon
- Status: `✓` Clean, `●` Dirty, `⚠` Conflicts  
- Remote tracking: `↑n` Ahead, `↓n` Behind

### Model Display

Shows simplified Claude model names:
- `claude-3-5-sonnet` → `Sonnet 3.5`
- `claude-4-sonnet` → `Sonnet 4`

### Context Window Display

Token usage percentage based on transcript analysis with context limit tracking.

### Effort Display

Shows the current reasoning effort level reported by Claude Code, such as `low`, `medium`, `high`, `xhigh`, or `max`. Requires Claude Code 2.1.119 or later. The segment is omitted when the input has no `effort` field. All built-in themes enable Effort after Model.

## Configuration

CCometixLine supports full configuration via TOML files and interactive TUI:

- **Configuration file**: `~/.claude/ccline/config.toml`
- **Interactive TUI**: `ccline --config` for real-time editing with preview
- **Theme files**: `~/.claude/ccline/themes/*.toml` for custom themes
- **Automatic initialization**: `ccline --init` creates default configuration

### Available Segments

All segments are configurable with:
- Enable/disable toggle
- Custom separators and icons
- Color customization
- Format options

Supported segments: Directory, Git, Model, Effort, Agents, Context Window, Usage, Session, Cost, Output Style, Update

### Usage Configuration

Usage is disabled by default. Run `ccline --config`, select **Usage**, press **Enter** to enable it, then **S** to save. It reads `rate_limits` directly from [Claude Code's statusline input](https://code.claude.com/docs/en/statusline#available-data), available since Claude Code 2.1.80. The text shows the seven-day percentage, five-hour percentage, and five-hour reset countdown in that order, such as `63% · 24% · 2h`; the preceding circle icon shows seven-day utilization. Unavailable values and expired reset times appear as `?`. If neither window is present, the segment is hidden. Usage has no segment-specific options and makes no API requests or credential/cache reads.

### Effort Configuration

Run `ccline --config`, select **Effort** in the segment list, and press **Enter** to toggle it independently of Model. Use **Tab** to edit its icon, colors, background, and text style, **Shift+Up/Down** to reorder it, and **S** to save `config.toml`. The preview updates as you edit. **W** writes to the current theme file, which is used when launching with `--theme`.

If your configuration or selected theme does not contain Effort, the TUI presents it as a disabled option after Model (or at the end when Model is absent). Effort uses a lightning icon and its own purple palette for the selected built-in theme; custom themes use the default Effort appearance. Press **Enter** to enable it, then **S** to save the configuration or **W** to write the current theme. The added Effort option is written only when you save. You can also configure the segment directly in `config.toml` or a theme file:

```toml
[[segments]]
id = "effort"
enabled = true
icon = { plain = "⚡", nerd_font = "\uf0e7" }
colors = { icon = { c16 = 13 }, text = { c16 = 13 } }
styles = { text_bold = false }
options = {}
```

Set `enabled = false` to hide Effort. The value comes directly from the statusline input's `effort.level`; ccline does not infer it from model names, thinking settings, or token usage.

### Subagent Activity

**Agents is disabled by default in every built-in theme.** Run `ccline --config`, select **Agents**, press **Enter** to enable it, then **S** to save. With the default backend, saving installs the `SessionStart`, `SubagentStart`, `SubagentStop`, and `SessionEnd` command hooks in `~/.claude/settings.json`. Restart Claude Code after enabling so `SessionStart` initializes activity tracking. If there is no status line configured, it also configures ccline as the status line. An existing status line command is preserved. When no `refreshInterval` is configured, ccline adds a two-second refresh so background activity updates while the main session is idle.

Disable **Agents** and press **S** to uninstall its hooks. Other hooks and settings are preserved. A refresh interval added by ccline is removed on uninstall if it is still unchanged; an existing or subsequently edited interval is preserved. Unsaved edits only affect the preview. **W** and **Ctrl+S** save theme files without changing the hook installation; **S** applies the activity toggle. Configurations and themes without an Agents entry show it as a disabled option in the TUI.

While observed subagents are active, the main status line shows a summary such as `Agents: 4 active · 1 responded · reviewer 1m20s · Explore 35s · Plan 12s · +1 more`. Active agents are listed longest-running first. `max_agents` sets the upper limit on individually displayed agents and defaults to `3`. On each refresh, ccline measures the whole line, including other segments, icons, separators, and counts, then removes complete trailing name-and-duration entries until the line fits, updating `+N more`. When no names fit, or `max_agents = 0`, only counts are shown. With no active agents, the segment and its separator are hidden. If the other segments and counts alone exceed the available width, removing names cannot prevent the host from clipping the line.

In the TUI, select **Agents**, press **Tab** to enter Settings, select **Max agents**, and press **Enter** to enter a non-negative integer. **Ctrl+U** clears the input, **Enter** applies it, **Esc** cancels, and **S** saves `config.toml`. Invalid input shows an error and keeps the editor open. Colors, icons, backgrounds, text style, and ordering use the existing controls. The preview uses example activity and fits names to the actual preview width. You can also edit the Agents entry in a configuration or theme file:

```toml
[[segments]]
id = "agents"
enabled = true
icon = { plain = "A", nerd_font = "\uf0c0" }
colors = { icon = { c16 = 6 }, text = { c16 = 6 } }
styles = { text_bold = false }
options = { max_agents = 3, experimental_mod = false }
```

Automatic layout reads [`COLUMNS`](https://code.claude.com/docs/en/statusline#how-status-lines-work), which Claude Code sets for each invocation. Width is measured in terminal cells, accounting for CJK text, combining characters, and emoji; ANSI colors take no space. By default ccline reserves Claude Code's two-column margin on each side. If you set `statusLine.padding`, use the status line command `ccline --width-offset N`, where `N = 4 + 2 × padding`; for example, `padding = 2` needs `--width-offset 8`. Missing or invalid `COLUMNS` produces an explicit error when active names require layout; supply it when invoking ccline manually. Window resizing takes effect on the next invocation.

Activity is grouped by the host's `CLAUDE_PID`, then keyed by session ID and agent ID, with file locking for concurrent hook events. Repeated starts do not increase the count, a resumed agent starts a new elapsed-time measurement, compaction preserves activity, and starting or resuming a session resets that session's observations. The hook command is `ccline --agents-hook`, with event data read from stdin. The default hooks backend keeps each process's activity in `~/.claude/ccline/agents/<CLAUDE_PID>.json` and uses a matching `.lock` file. `/clear` removes the current session's records; normal Claude Code exit removes both files, including records from sessions switched with `/resume`. Ordinary replies and subagent stops retain records, and cleanup leaves other Claude Code processes untouched. Hooks and status line invocations require the host-provided `CLAUDE_PID`. The installation manifest at `~/.claude/ccline/agents-installation.json` tracks the managed hook command and refresh interval for uninstalling.

The summary reflects [official lifecycle hook events](https://code.claude.com/docs/en/hooks#subagentstart). `responded` means a `SubagentStop` event was observed, not that a task succeeded. These hooks do not reliably distinguish waiting for permission, failure, cancellation, or continuation requested by another stop hook, so the segment uses the configured active color instead of inferring those states. An interruption without an end event cannot be confirmed from this data; forced termination or a crash without `SessionEnd` cannot trigger file cleanup. Agents that were already running before installation are not reconstructed from transcript files. Malformed state or settings files produce explicit errors.

### Experimental Agents Mod

**Experimental Mod is off by default.** In the Agents settings panel, select **Experimental Mod**, press **Enter** to toggle it, then **S** to save. The configuration option is `experimental_mod = true` under the Agents entry's `options`. Saving checks `claude --version` from PATH: Claude Code **2.1.273 or newer** selects the Mod; an older or unavailable CLI selects the existing hooks backend and the TUI reports why. Detection happens on save, so save again after changing Claude Code versions. Restart Claude Code after switching backends.

Mod mode installs the bundled plugin under `~/.claude/skills/ccline-agents-mod/`, enables it and the host's `CLAUDE_CODE_ENABLE_FUNCTION_HOOKS` experiment, and removes **only ccline's four lifecycle command hooks**. Other hooks remain in place and continue through the Mod's `next` calls. Turning the experiment off restores the command hooks and restores the host settings ccline changed if they are still unchanged. The deployed plugin defaults to disabled when no enable setting is present. Enabling the host experiment also permits function hooks in other enabled plugins.

The Mod consumes the same lifecycle events in process and publishes activity through `CCLINE_AGENTS_SNAPSHOT`, inherited by status line processes. This removes ccline's per-event command processes and activity-file reads, writes, and locks. The existing Rust renderer, elapsed times, `max_agents`, width fitting, and refresh interval remain in use. Mod activity is held in memory; it does not reconstruct events missed before plugin loading or after a reload. If `CCLINE_AGENTS_SNAPSHOT` is missing, invalid, or lacks the current session, only the Agents segment is hidden; the other status line segments still render and ccline exits normally. Mod mode does not read old hook records. The API is Early Access; the [component documentation](mods/agents/README.md) records the source baseline and validation scope.

### Model Configuration (`models.toml`)

Location: `~/.claude/ccline/models.toml` (auto-created on first run)

This file configures how model IDs are displayed and their context window limits. Claude models (Sonnet, Opus, Haiku) are automatically recognized with version extraction — you only need this file for overrides or third-party models.

```toml
# Model entries: simple substring matching on the model ID
# These take priority over built-in Claude model recognition
[[models]]
pattern = "glm-4.5"
display_name = "GLM-4.5"
context_limit = 128000

[[models]]
pattern = "kimi-k2"
display_name = "Kimi K2"
context_limit = 128000

# Context modifiers: matched independently and composable with model entries
# Overrides context_limit and appends display_suffix to the display name
# e.g., model "Opus 4" + modifier " 1M" = "Opus 4 1M"
[[context_modifiers]]
pattern = "[1m]"
display_suffix = " 1M"
context_limit = 1000000
```


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

## Roadmap

- [x] TOML configuration file support
- [x] TUI configuration interface
- [x] Custom themes
- [x] Interactive main menu
- [x] Claude Code enhancement tools

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## Related Projects

- [tweakcc](https://github.com/Piebald-AI/tweakcc) - Command-line tool to customize your Claude Code themes, thinking verbs, and more.

## License

This project is licensed under the [MIT License](LICENSE).

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Haleclipse/CCometixLine&type=Date)](https://star-history.com/#Haleclipse/CCometixLine&Date)
