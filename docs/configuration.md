# Configuration

[English](configuration.md) | [中文](configuration.zh.md)

## Configuration Directory

Every file ccline reads or writes lives under Claude Code's configuration directory, written `<config dir>` below: [`CLAUDE_CONFIG_DIR`](https://code.claude.com/docs/en/env-vars) when that variable is set, otherwise `~/.claude`.

| File | Purpose |
| --- | --- |
| `<config dir>/ccline/config.toml` | Main configuration: style, segments, and theme name |
| `<config dir>/ccline/themes/*.toml` | Built-in and custom theme files |
| `<config dir>/ccline/models.toml` | Model display names, see [Model Names](#model-names-modelstoml) |
| `<config dir>/ccline/agents/*.json` | Per-session visible running-agent count and observation time |
| `<config dir>/ccline/agents-installation.json` | The subagent panel command ccline installed, see [Subagent Panel](subagent-panel.md) |
| `<config dir>/ccline/.update_state.json` | Update check state for the Update segment |
| `<config dir>/settings.json` | Claude Code settings; ccline edits only `subagentStatusLine` |

ccline reads `CLAUDE_CONFIG_DIR` from the environment it runs in. A status line command started by a Claude Code instance with `CLAUDE_CONFIG_DIR=/path/to/dir` inherits that variable, so it reads `/path/to/dir/ccline/`, and enabling Agents edits `/path/to/dir/settings.json` without touching the default `~/.claude`. When you run `ccline --config` for such an instance, set the same variable in your shell. `CLAUDE_CONFIG_DIR` must be an absolute path; an empty or relative value is reported as an error. Paths containing spaces are supported. The binary itself can live anywhere on `PATH`.

## Initialization and Errors

Rendering reads configuration without changing it. With Agents enabled, the subagent panel writes its per-session count snapshot and the main status line reads it. Built-in theme files and the `models.toml` template are created where missing by **Initialize Config** in the main menu (run `ccline` without input) and when the TUI (`ccline --config`) starts. The same two actions clean up an existing `models.toml`, see [Obsolete Fields](#obsolete-fields). **Check Configuration** in the main menu validates both `config.toml` and `models.toml`. **S** in the TUI saves `config.toml`; **W** and **Ctrl+S** save theme files.

A missing `config.toml` means the default theme, and a missing `models.toml` means the built-in model names. A file that exists but cannot be read or parsed is reported as an error instead of falling back to defaults. This includes a theme file selected with `--theme`: a file in the themes directory takes priority over the built-in theme of the same name, and an unknown theme name is an error.

## Segments

All segments are configurable with an enable/disable toggle, icons, colors, background, and text style, and can be reordered with **Shift+Up/Down** in the TUI. Supported segments: Directory, Git, Model, Effort, Agents, Context Window, Prompt Cache, Usage, Session Name, Worktree, Pull Request, Session, Cost, Output Style, Update. [Segments](segments.md) describes what each data segment shows.

## Model Names (`models.toml`)

This file customizes how model IDs are displayed. Claude models (Sonnet, Opus, Haiku, Fable) are automatically recognized with version extraction, so you only need this file for overrides or third-party models. Context window sizes come from Claude Code's input and are not configured here. Unknown fields are rejected.

```toml
# Model entries: simple substring matching on the model ID
# These take priority over built-in Claude model recognition
[[models]]
pattern = "glm-4.5"
display_name = "GLM-4.5"

[[models]]
pattern = "kimi-k2"
display_name = "Kimi K2"

# Context modifiers: matched independently and composable with model entries
# Appends display_suffix to the display name
# e.g., model "Opus 4" + modifier " 1M" = "Opus 4 1M"
[[context_modifiers]]
pattern = "[1m]"
display_suffix = " 1M"
```

Entries in this file take priority over the built-in ones. The same names are used by the Model segment and by the subagent panel rows.

### Obsolete Fields

Earlier versions required `context_limit` in every entry. While it remains, rendering reports an error that names the field and suggests the cleanup below.

**Initialize Config** and starting `ccline --config` remove `context_limit` from every `[[models]]` and `[[context_modifiers]]` entry, including inline `models = [{ ... }]` entries. Everything else in the file, comments and ordering included, is left as written. The original file is saved first as `models.toml.<timestamp>.bak` in the same directory, and the TUI or menu reports how many fields were removed and where the backup is. A file with nothing to remove is not touched. If the file would still be invalid after the cleanup, for example because of another unknown field, nothing is written, the error names the remaining problem, and the TUI does not start until the file is fixed. Comments that mention `context_limit` are not changed.
