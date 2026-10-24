# Subagent Panel

[English](subagent-panel.md) | [中文](subagent-panel.zh.md)

## Enabling

**Agents is disabled by default in every built-in theme.** Run `ccline --config`, select **Agents**, press **Enter** to enable it, then **S** to save. The preview shows both the main count and sample agent row bodies using the same renderer. Saving sets [`subagentStatusLine`](https://code.claude.com/docs/en/statusline#subagent-status-lines) in `<config dir>/settings.json` (see [Configuration Directory](configuration.md#configuration-directory)) to the command `ccline --subagent-statusline`. Restart Claude Code to apply.

## Main Status Line

The Agents icon and `1 Agent` (or `2 Agents`, etc. for multiple agents) appear in the main status line alongside the model and effort. Only `local_agent` entries with status `running` in the latest native panel input are counted; completed, failed, and killed entries are excluded. This is a count of the supplied visible entries, not all running subagents.

Each panel refresh atomically saves the count and observation time in `<config dir>/ccline/agents/<hex-encoded session ID>.json`. The main status line reads only its own session's snapshot. A missing, zero, or expired count hides the segment without an extra separator or blank line. Read, write, and parse failures are reported as errors.

Claude Code stops calling the panel command when its rows disappear, so snapshots expire after 15 seconds by default. Set `options = { summary_ttl_seconds = 15 }` on the Agents segment in `config.toml` or a theme file to change this interval; `0` disables expiry. The count updates or disappears on the next main status line refresh. For updates while idle, set `statusLine.refreshInterval` yourself, for example to `5`; ccline does not change this setting.

## Rows

Claude Code renders the native tree, including parent-child indentation, connectors, status bullets, and selection controls. On each panel refresh it passes the visible tasks on stdin; ccline returns one JSON record per task ID to replace only the row body, preserving the host's order and layout:

```text
Explore · find config loading · [>] 1m20s · 12.4k (6%) · Sonnet 5.5 · high
```

The fields are:

1. The agent type, falling back to the agent's name.
2. The task's progress label.
3. A status marker and the elapsed time while running, or the task status otherwise.
4. The subagent's token count with its share of the context window.
5. The model's display name, resolved through [`models.toml`](configuration.md#model-names-modelstoml).
6. The configured reasoning effort; a numeric budget shows as `8k budget`.

Fields absent from the input are omitted. Row bodies start directly with the agent type and use the Agents segment's text color and bold setting, except that the status field uses semantic colors from the selected theme:

| Input status | Status field | Color |
| --- | --- | --- |
| `running` | `[>] 1m20s` | Orange |
| `completed` | `[+] completed` | Green |
| `failed` | `[!] failed` | Red |
| `killed` | `[x] killed` | Gray |
| Any other value | `[?]` followed by the original status | Yellow |

The shades follow `config.theme`, including dark text for Powerline Light; custom theme names use the Default status palette. The markers are ASCII and do not require Nerd Fonts. No approval wait, slow tool, or inactivity state is inferred from elapsed time or token counts.

The main-line Agents icon, background, and segment separator do not apply to row bodies. ccline adds no heading, tree prefix, leading padding, or blank line. Claude Code owns the spacing after the permission footer; its native panel has a one-row top margin in v2.1.309.

When a row is wider than the `columns` value Claude Code provides, ccline drops the effort, then the model, then the token count, and finally shortens the label with `…`. The agent type and status marker stay present. Width includes the marker and is measured in terminal cells, accounting for CJK text, combining characters, and emoji; ANSI colors take no space. Claude Code clips a body that still cannot fit and owns the width occupied by its tree prefixes.

Claude Code decides which rows exist: a successfully finished subagent's row disappears immediately, a failed or stopped one stays for 30 seconds, and the panel lists a limited number of rows ([official docs](https://code.claude.com/docs/en/sub-agents#run-subagents-in-foreground-or-background)). The `agentType` field requires Claude Code 2.1.293 or later. With the Agents segment disabled or missing from the configuration, the command writes nothing and Claude Code keeps its default rows. Malformed panel input produces an explicit error.

## Ownership and Conflicts

ccline records the command it wrote in `<config dir>/ccline/agents-installation.json`.

- If `subagentStatusLine` already holds a value ccline did not write, including a later edit to ccline's own value, saving with Agents enabled reports the conflict and changes nothing. Remove that value yourself, or keep it and leave Agents disabled.
- A value identical to the command ccline would write is treated as ccline's, and a command ccline wrote for a previous executable path is replaced.
- Disable **Agents** and press **S** to remove the panel command. It is removed only while unchanged, so a value you edited is kept.
- Saving touches no other setting: `statusLine`, `refreshInterval`, and hooks are left as they are. **W** and **Ctrl+S** save theme files without changing the panel installation.

Claude Code runs `subagentStatusLine` under the same workspace trust, `disableAllHooks`, and `allowManagedHooksOnly` rules as the status line. Configurations and themes without an Agents entry show it as a disabled option in the TUI.

## Removed Backends

Earlier versions also fed an inline Agents segment from lifecycle command hooks or an experimental Mod for older Claude Code releases. Both backends, the `max_agents` and `experimental_mod` options, and the `--agents-hook` and `--width-offset` flags have been removed.

- If an old installation left `ccline --agents-hook` hooks in your settings, delete them.
- If you used the Mod, remove `enabledPlugins["ccline-agents-mod@skills-dir"]` from `settings.json`, the `CLAUDE_CODE_ENABLE_FUNCTION_HOOKS` entry under `env` if ccline added it, and the `<config dir>/skills/ccline-agents-mod/` directory.
- The hooks backend added `"refreshInterval": 2` to `statusLine`; keep, change, or remove it as you prefer (see [Refreshing While Idle](segments.md#refreshing-while-idle)).
- An `agents-installation.json` written by the hooks or Mod backend is reported as invalid; delete it after removing those hooks.
- Remove `--width-offset` from your `statusLine` command if you added it.
