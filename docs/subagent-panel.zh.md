# 子代理面板

[English](subagent-panel.md) | [中文](subagent-panel.zh.md)

## 启用

**所有内置主题默认关闭 Agents。** 运行 `ccline --config`，选择 **Agents**，按 **Enter** 启用，再按 **S** 保存。预览使用同一套渲染逻辑展示主状态栏数量和示例 agent 行正文。保存时 ccline 会在 `<配置目录>/settings.json`（见[配置目录](configuration.zh.md#配置目录)）中把 [`subagentStatusLine`](https://code.claude.com/docs/en/statusline#subagent-status-lines) 设为命令 `ccline --subagent-statusline`。重启 Claude Code 后生效。

## 主状态栏

Agents 图标和 `1 Agent`（多个时为 `2 Agents` 等）与模型名称、思考强度显示在同一行。数量只统计最近一次原生面板输入中 `type` 为 `local_agent` 且 `status` 为 `running` 的条目，不包含已完成、失败或被停止的条目。这是传入的可见条目数量，不是全部运行中的子代理总数。

每次面板刷新都会把数量和观测时间原子写入 `<配置目录>/ccline/agents/<十六进制编码的会话 ID>.json`。主状态栏只读取自身会话的快照。数量缺失、为零或过期时隐藏该段落，不留下多余分隔符或空行。读写或解析失败会明确报错。

Claude Code 在面板清空后不再调用面板命令，因此快照默认在 15 秒后过期。可以在 `config.toml` 或主题文件的 Agents 段落中设置 `options = { summary_ttl_seconds = 15 }` 调整间隔；`0` 关闭过期。数量更新或隐藏在主状态栏下一次刷新时生效。需要空闲时也刷新，可自行把 `statusLine.refreshInterval` 设为 `5` 等值；ccline 不修改该设置。

## 行内容

Claude Code 绘制原生树，包括父子缩进、连接线、状态圆点和选择控件。每次刷新面板时，宿主通过 stdin 传入可见任务；ccline 按任务 ID 返回一条 JSON 记录，只替换行正文，沿用宿主的排列和布局：

```text
Explore · find config loading · [>] 1m20s · 12.4k (6%) · Sonnet 5.5 · high
```

字段依次为：

1. 代理类型，缺失时使用代理名称。
2. 任务的进度标签。
3. 状态标记，以及运行中的已用时间或其他任务状态。
4. 子代理的 token 数及其占上下文窗口的比例。
5. 通过 [`models.toml`](configuration.zh.md#模型名称modelstoml) 解析的模型显示名称。
6. 配置的推理工作量；数值预算显示为 `8k budget`。

输入中缺失的字段会被省略。行正文直接以代理类型开头，使用 Agents 段落的文字颜色和粗体设置，其中状态字段使用所选主题的语义颜色：

| 输入状态 | 状态字段 | 颜色 |
| --- | --- | --- |
| `running` | `[>] 1m20s` | 橙色 |
| `completed` | `[+] completed` | 绿色 |
| `failed` | `[!] failed` | 红色 |
| `killed` | `[x] killed` | 灰色 |
| 其他值 | `[?]` 加原始状态文字 | 黄色 |

具体色值跟随 `config.theme`，Powerline Light 使用深色文字；自定义主题名称使用 Default 的状态配色。标记使用 ASCII，不依赖 Nerd Fonts。不会根据已用时间或 token 数推断等待审批、工具缓慢或无活动状态。

主状态栏的 Agents 图标、背景和段落分隔符不用于行正文。ccline 不添加标题、树形前缀、前导空格或空行。权限提示下方的间距由 Claude Code 控制；其 v2.1.309 原生面板设置了一行顶部留白。

行宽超过 Claude Code 提供的 `columns` 时，ccline 依次去掉工作量、模型、token 数，最后用 `…` 截短标签，保留代理类型和状态标记。宽度包含状态标记，按终端显示列计算，计入中文、组合字符和 emoji，ANSI 颜色不占列宽。仍无法容纳的正文由 Claude Code 裁剪，树形前缀占用的宽度也由宿主处理。

哪些行存在由 Claude Code 决定：成功结束的子代理行会立即消失，失败或被停止的行保留 30 秒，面板只列出有限数量的行（[官方文档](https://code.claude.com/docs/en/sub-agents#run-subagents-in-foreground-or-background)）。`agentType` 字段要求 Claude Code 2.1.293 或更新版本。Agents 段落被禁用或配置中没有该段落时，命令不输出任何内容，Claude Code 保留默认行。格式错误的面板输入会明确报错。

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
