# 子代理面板

[English](subagent-panel.md) | [中文](subagent-panel.zh.md)

## 启用

**所有内置主题默认关闭 Agents。** 运行 `ccline --config`，选择 **Agents**，按 **Enter** 启用，再按 **S** 保存。保存时 ccline 会在 `<配置目录>/settings.json`（见[配置目录](configuration.zh.md#配置目录)）中把 [`subagentStatusLine`](https://code.claude.com/docs/en/statusline#subagent-status-lines) 设为命令 `ccline --subagent-statusline`。重启 Claude Code 后生效。

## 行内容

Claude Code 每次刷新面板时运行该命令，并通过 stdin 传入可见的行；ccline 将每一行替换为类似下面的内容：

```text
Explore · find config loading · 1m20s · 12.4k (6%) · Sonnet 5.5 · high
```

字段依次为：

1. 代理类型，缺失时使用代理名称。
2. 任务的进度标签。
3. 运行中显示已用时间，否则显示任务状态（例如 `failed`）。
4. 子代理的 token 数及其占上下文窗口的比例。
5. 通过 [`models.toml`](configuration.zh.md#模型名称modelstoml) 解析的模型显示名称。
6. 配置的推理工作量；数值预算显示为 `8k budget`。

输入中缺失的字段会被省略。行使用 Agents 段落的图标、图标颜色、文字颜色和粗体设置；背景和分隔符不适用，也不添加标题。

行宽超过 Claude Code 提供的 `columns` 时，ccline 依次去掉工作量、模型、token 数，最后用 `…` 截短标签。宽度按终端显示列计算，正确处理中文、组合字符和 emoji，ANSI 颜色不占列宽。

哪些行存在由 Claude Code 决定：成功结束的子代理行会立即消失，失败或被停止的行保留 30 秒，面板只列出有限数量的行（[官方文档](https://code.claude.com/docs/en/sub-agents#run-subagents-in-foreground-or-background)）。`agentType` 字段要求 Claude Code 2.1.293 或更新版本。Agents 段落被禁用或配置中没有该段落时，命令不输出任何内容，Claude Code 保留默认行。Agents 段落本身不会在主状态栏输出内容。格式错误的面板输入会明确报错。

## 所有权与冲突

ccline 在 `<配置目录>/ccline/agents-installation.json` 中记录它写入的命令。

- 如果 `subagentStatusLine` 已有不是 ccline 写入的值（包括用户后来改写过的 ccline 值），启用 Agents 并保存时会报告配置冲突，不做任何修改。请自行删除该值，或保留它并保持 Agents 关闭。
- 与 ccline 将要写入的命令完全相同的值视为 ccline 所有；ccline 为旧的可执行文件路径写入的命令会被替换。
- 关闭 **Agents** 并按 **S** 会移除面板命令；仅在该值未被改动时移除，用户改写过的值会保留。
- 保存不会修改其他设置：`statusLine`、`refreshInterval` 和 hooks 均保持原样。**W** 和 **Ctrl+S** 保存主题文件，不改变面板安装状态。

Claude Code 运行 `subagentStatusLine` 时适用与状态栏相同的工作区信任、`disableAllHooks` 和 `allowManagedHooksOnly` 规则。没有 Agents 条目的配置和主题会在 TUI 中显示为禁用选项。

## 已移除的后端

旧版本还会通过生命周期命令 hooks 或面向旧版 Claude Code 的实验性 Mod 为主状态栏的内联 Agents 段落提供数据。这两种后端、`max_agents` 和 `experimental_mod` 选项，以及 `--agents-hook` 和 `--width-offset` 参数均已移除。

- 如果旧安装在设置中留下了 `ccline --agents-hook` hooks，请删除它们。
- 如果使用过 Mod，请从 `settings.json` 中删除 `enabledPlugins["ccline-agents-mod@skills-dir"]`、由 ccline 添加的 `env` 下的 `CLAUDE_CODE_ENABLE_FUNCTION_HOOKS`，并删除 `<配置目录>/skills/ccline-agents-mod/` 目录。
- Hooks 后端曾在 `statusLine` 中加入 `"refreshInterval": 2`；可按需保留、修改或删除（见[空闲时刷新](segments.zh.md#空闲时刷新)）。
- 由 Hooks 或 Mod 后端写入的 `agents-installation.json` 会被报告为无效；删除这些 hooks 后再删除该文件。
- 如果在 `statusLine` 命令中加了 `--width-offset`，请将其删除。
