# Segments

[English](segments.md) | [中文](segments.zh.md)

Session data comes from Claude Code's [status line input](https://code.claude.com/docs/en/statusline#available-data); the Agents count comes from its native subagent panel. Values the host reports as missing or null are shown as unknown, never as zero; malformed input is reported as an error.

## Context Window

Shows the context usage Claude Code reports in [`context_window`](https://code.claude.com/docs/en/statusline#context-window-fields), such as `20% · 200k tokens`.

- The percentage is `used_percentage`.
- The token count is the input side of `current_usage`: input plus cache creation and cache read tokens. This is the same input-only formula Claude Code uses for `used_percentage`; output tokens are not counted.
- The window size comes from the host, so models with a 1M window are measured correctly without a `[1m]` suffix in the model ID.
- Before the first response, and right after `/compact` until the next API call, Claude Code reports these values as null. ccline shows them as `-` instead of zero or older data.

ccline does not read transcript files.

## Usage

Usage is disabled by default. Run `ccline --config`, select **Usage**, press **Enter** to enable it, then **S** to save. It reads `rate_limits` (available since Claude Code 2.1.80), has no segment-specific options, and makes no API requests or credential/cache reads.

### Subscription Windows

The text shows the seven-day percentage, five-hour percentage, and five-hour reset countdown in that order, such as `63% · 24% · 2h`; the preceding circle icon shows seven-day utilization. Unavailable values and expired reset times appear as `?`.

### Gateway Spend Limit

Behind a [Claude apps gateway spend limit](https://code.claude.com/docs/en/statusline#spend-limit-fields), Claude Code sends `rate_limits.spend_limit` instead of, or alongside, the subscription windows.

- With only a spend limit, the segment shows a budget such as `spend 63% · $314.12/$500 monthly · 12d`: the used percentage, the estimated spend and limit in dollars, the period, and the reset countdown. The circle icon follows the budget percentage. No five-hour or seven-day label is applied.
- The dollar amounts and period arrive separately from the percentage and may be absent or briefly disagree with it. ccline shows whichever are present (`$314.12 used`, `$500 limit`), or only the percentage.
- With both subscription windows and a spend limit, the budget follows the five-hour reset, such as `63% · 24% · 2h · spend 63% · $500 limit · 12d`.

If no window and no spend limit is present, the segment is hidden.

### Refreshing While Idle

Claude Code re-runs the status line when a reset time passes, but its other update events can go quiet while the main session is idle. To keep the countdown current during idle periods, set [`refreshInterval`](https://code.claude.com/docs/en/statusline#manually-configure-a-status-line) in your `statusLine` setting yourself, for example `"refreshInterval": 60` for minute-level countdowns. ccline never adds, changes, or removes `refreshInterval`.

## Effort

Shows the current reasoning effort level reported in `effort.level`, such as `low`, `medium`, `high`, `xhigh`, or `max`. Requires Claude Code 2.1.119 or later. The segment is omitted when the input has no `effort` field. ccline does not infer the level from model names, thinking settings, or token usage. All built-in themes enable Effort after Model.

Run `ccline --config`, select **Effort** in the segment list, and press **Enter** to toggle it independently of Model. Use **Tab** to edit its icon, colors, background, and text style, **Shift+Up/Down** to reorder it, and **S** to save `config.toml`. The preview updates as you edit. **W** writes to the current theme file, which is used when launching with `--theme`.

If your configuration or selected theme does not contain Effort, the TUI presents it as a disabled option after Model (or at the end when Model is absent). Effort uses a lightning icon and its own purple palette for the selected built-in theme; custom themes use the default Effort appearance. The added Effort option is written only when you save. You can also configure the segment directly in `config.toml` or a theme file:

```toml
[[segments]]
id = "effort"
enabled = true
icon = { plain = "⚡", nerd_font = "\uf0e7" }
colors = { icon = { c16 = 13 }, text = { c16 = 13 } }
styles = { text_bold = false }
options = {}
```

Set `enabled = false` to hide Effort.

## Agents

The Agents segment shows its icon and `2 Agents running` alongside the model and effort. The count covers running entries in the latest native panel input, not all subagents. Enabling it also installs text-only rows for Claude Code's subagent panel. Missing, zero, or expired counts hide the segment; see [Subagent Panel](subagent-panel.md) for the cache and expiry setting.
