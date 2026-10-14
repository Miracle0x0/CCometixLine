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
tar -xzf "$archive" -C ~/.claude/ccline
chmod +x ~/.claude/ccline/ccline
export PATH="$HOME/.claude/ccline:$PATH"
```

#### Windows

```powershell
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.claude\ccline"
Invoke-WebRequest -Uri "https://github.com/Miracle0x0/CCometixLine/releases/latest/download/ccline-windows-x64.zip" -OutFile "ccline-windows-x64.zip"
Expand-Archive -Path "ccline-windows-x64.zip" -DestinationPath "$env:USERPROFILE\.claude\ccline" -Force
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

**Agents is disabled by default in every built-in theme.** Run `ccline --config`, select **Agents**, press **Enter** to enable it, then **S** to save. Saving checks `claude --version` from PATH and chooses where agents appear:

- Claude Code **2.1.293 or newer**: Claude Code's own subagent panel below the prompt lists the agents and ccline renders its rows, as described in [Subagent Panel](#subagent-panel). The main status line keeps its other segments without the Agents segment, and no lifecycle hooks are installed.
- Older versions, or no `claude` on PATH: the main status line shows the Agents segment described in the rest of this section, fed by lifecycle command hooks or the [Experimental Agents Mod](#experimental-agents-mod).

Detection happens on save, so save again after changing Claude Code versions. An invalid version response or a failed `claude --version` is reported as an error. Restart Claude Code after saving.

With the hooks backend, saving installs the `SessionStart`, `SubagentStart`, `SubagentStop`, and `SessionEnd` command hooks in `~/.claude/settings.json`. Restart Claude Code after enabling so `SessionStart` initializes activity tracking. If there is no status line configured, it also configures ccline as the status line. An existing status line command is preserved. When no `refreshInterval` is configured, ccline adds a two-second refresh so background activity updates while the main session is idle.

Disable **Agents** and press **S** to uninstall its hooks or panel command. Other hooks and settings are preserved. A refresh interval added by ccline is removed on uninstall if it is still unchanged; an existing or subsequently edited interval is preserved. Unsaved edits only affect the preview. **W** and **Ctrl+S** save theme files without changing the hook installation; **S** applies the activity toggle. Configurations and themes without an Agents entry show it as a disabled option in the TUI.

Below Claude Code 2.1.293, while observed subagents are active, the main status line shows a summary such as `Agents: 4 active · 1 responded · reviewer 1m20s · Explore 35s · Plan 12s · +1 more`. Active agents are listed longest-running first. `max_agents` sets the upper limit on individually displayed agents and defaults to `3`. On each refresh, ccline measures the whole line, including other segments, icons, separators, and counts, then removes complete trailing name-and-duration entries until the line fits, updating `+N more`. When no names fit, or `max_agents = 0`, only counts are shown. With no active agents, the segment and its separator are hidden. If the other segments and counts alone exceed the available width, removing names cannot prevent the host from clipping the line.

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

### Subagent Panel

On Claude Code 2.1.293 or newer, saving sets `subagentStatusLine` in `~/.claude/settings.json` to the command `ccline --subagent-statusline`. Claude Code runs it on each panel refresh with the visible rows on stdin, and ccline replaces each row with a line such as:

```text
Explore · find config loading · 1m20s · 12.4k (6%) · Sonnet 5.5 · high
```

The fields are the agent type (falling back to the agent's name), the task's progress label, the elapsed time while running or the task status otherwise (such as `failed`), the subagent's token count with its share of the context window, the model's display name resolved through `models.toml`, and the configured reasoning effort (a numeric budget shows as `8k budget`). Fields absent from the input are omitted. Rows use the Agents segment's icon, icon color, text color, and bold setting; backgrounds and separators do not apply, and no heading is added. When a row is wider than the `columns` value Claude Code provides, ccline drops the effort, then the model, then the token count, and finally shortens the label with `…`.

Claude Code decides which rows exist: a successfully finished subagent's row disappears immediately, a failed or stopped one stays for 30 seconds, and the panel lists a limited number of rows ([official docs](https://code.claude.com/docs/en/sub-agents#run-subagents-in-foreground-or-background)). This mode therefore has no `responded` count, and `max_agents` and the main status line width fitting do not apply. The `agentType` field arrived in 2.1.293, which sets the version threshold. With the Agents segment disabled or missing from the configuration, the command writes nothing and Claude Code keeps its default rows. Malformed panel input produces an explicit error.

If `subagentStatusLine` already holds a value ccline did not write, including a later edit to ccline's own value, ccline keeps it, installs the hooks backend instead, and the TUI reports this. Switching to hooks or disabling Agents removes the panel command only while it is unchanged. The panel does not need the status line timer, so ccline adds no `refreshInterval` in this mode and removes one it added earlier if it is unchanged. Claude Code runs `subagentStatusLine` under the same workspace trust, `disableAllHooks`, and `allowManagedHooksOnly` rules as the status line ([official docs](https://code.claude.com/docs/en/statusline#subagent-status-lines)). The installation manifest records the panel command for uninstalling.

### Experimental Agents Mod

**Experimental Mod is off by default.** In the Agents settings panel, select **Experimental Mod**, press **Enter** to toggle it, then **S** to save. The configuration option is `experimental_mod = true` under the Agents entry's `options`. Saving checks `claude --version` from PATH: Claude Code **2.1.273 through 2.1.292** selects the Mod; **2.1.293 or newer** uses the [Subagent Panel](#subagent-panel) instead, and the TUI notes that the Mod applies only below 2.1.293; an older or unavailable CLI selects the existing hooks backend and the TUI reports why. Detection happens on save, so save again after changing Claude Code versions. Restart Claude Code after switching backends.

Mod mode installs the bundled plugin under `~/.claude/skills/ccline-agents-mod/`, enables it, and removes **only ccline's four lifecycle command hooks**. It also sets the host's `CLAUDE_CODE_ENABLE_FUNCTION_HOOKS` experiment, which gated function hooks through the Early Access releases; Claude Code 2.1.287 loads a mod with the variable unset, so the setting only matters on versions that still require it. Other hooks remain in place and continue through the Mod's `next` calls. Turning the experiment off restores the command hooks. Turning it off or moving to the subagent panel restores the host settings ccline changed if they are still unchanged. The deployed plugin defaults to disabled when no enable setting is present. On versions that still gate function hooks, enabling the host experiment also permits function hooks in other enabled plugins.

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

### Releases

Change `[package].version` in `Cargo.toml` and run `cargo check` to update `Cargo.lock`, then commit both files and push to `main` or `master`. The release workflow compares package versions before and after the push. A version change builds all seven platform archives and creates `v<version>` and its GitHub Release at the pushed commit. Dependency-only changes do not publish a release; no manual tag push is needed.

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
