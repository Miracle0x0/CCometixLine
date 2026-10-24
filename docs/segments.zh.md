# 段落说明

[English](segments.md) | [中文](segments.zh.md)

会话数据来自 Claude Code 的[状态栏输入](https://code.claude.com/docs/en/statusline#available-data)，Agents 数量来自原生子代理面板。宿主报告为缺失或 null 的值显示为未知，不会显示为零；格式错误的输入会明确报错。

## 上下文窗口（Context Window）

显示 Claude Code 在 [`context_window`](https://code.claude.com/docs/en/statusline#context-window-fields) 中报告的上下文占用，例如 `20% · 200k tokens`。

- 百分比取 `used_percentage`。
- token 数取 `current_usage` 的输入部分：输入加缓存写入和缓存读取 token。这与 Claude Code 计算 `used_percentage` 的仅输入口径一致，不计输出 token。
- 窗口大小由宿主提供，因此默认 1M 窗口的模型即使模型 ID 不带 `[1m]` 后缀也能正确计算。
- 首次响应之前，以及 `/compact` 之后到下一次 API 调用之前，Claude Code 将这些值报告为 null，ccline 显示 `-`，不会以零或旧数据代替。

ccline 不读取转录文件。

## 用量（Usage）

Usage 默认关闭。运行 `ccline --config`，选择 **Usage**，按 **Enter** 启用，再按 **S** 保存。它读取 `rate_limits`（Claude Code 2.1.80 起提供），没有段落专属选项，不发起 API 请求，也不读取凭据或缓存。

### 订阅窗口

文本依次显示七天百分比、五小时百分比和五小时重置倒计时，例如 `63% · 24% · 2h`；前面的圆形图标表示七天用量。不可用的值和已过期的重置时间显示为 `?`。

### 网关预算

在设置了[预算限制的 Claude apps 网关](https://code.claude.com/docs/en/statusline#spend-limit-fields)后，Claude Code 会单独或与订阅窗口一起发送 `rate_limits.spend_limit`。

- 只有预算时，段落显示预算信息，例如 `spend 63% · $314.12/$500 monthly · 12d`：已用百分比、以美元计的估算花费和上限、周期以及重置倒计时。圆形图标跟随预算百分比，不套用五小时或七天的标签。
- 金额和周期与百分比分开到达，可能缺失，也可能短暂与百分比不一致。ccline 只显示已有的字段（`$314.12 used`、`$500 limit`），或只显示百分比。
- 同时存在订阅窗口和预算时，预算信息接在五小时重置倒计时之后，例如 `63% · 24% · 2h · spend 63% · $500 limit · 12d`。

既没有订阅窗口也没有预算时，隐藏该段落。

### 空闲时刷新

重置时间到达时 Claude Code 会重新运行状态栏，但主会话空闲时其他更新事件可能停止。如需在空闲期间让倒计时保持更新，请自行在 `statusLine` 设置中配置 [`refreshInterval`](https://code.claude.com/docs/en/statusline#manually-configure-a-status-line)，例如按分钟显示的倒计时可设为 `"refreshInterval": 60`。ccline 不会添加、修改或删除 `refreshInterval`。

## 会话与仓库标识

**Session Name**、**Worktree** 和 **Pull Request** 是独立段落，所有主题默认关闭。在 `ccline --config` 中启用后按 **S** 保存。它们直接读取[原生状态栏字段](https://code.claude.com/docs/en/statusline#available-data)，不额外查询 Git、GitHub 或 GitLab。

| 段落 / 配置 ID | 数据来源 | 显示内容 |
| --- | --- | --- |
| Session Name / `session_name` | `session_name` | `/rename` 或 `--name` 设置的名称，或宿主生成的会话标题 |
| Worktree / `worktree` | `workspace.git_worktree` | 当前目录所属的 linked Git worktree 名称，也支持在 Claude Code 外创建的工作树 |
| Pull Request / `pull_request` | `pr.number`、`pr.url`、`pr.kind`、`pr.review_state` | 可点击的 `PR #123` 或 `MR !42`，后接宿主报告的审查状态（若有） |

字段缺失或为 null 时隐藏对应段落。Session Name 独立于 Session 的时长和修改行数；主工作树不显示 Worktree。PR/MR 关闭或合并、宿主不再报告该对象后，Pull Request 随之隐藏，不另行轮询。链接采用 OSC 8，需要终端支持。状态显示为 `approved`、`pending`、`changes requested` 或 `draft`；其中 GitLab 的 `approved` 表示宿主报告为可合并，并不代表 ccline 另外检查过审批。GitHub PR 字段要求 Claude Code 2.1.145+，GitLab MR 字段要求 2.1.234+，`workspace.git_worktree` 要求 2.1.98+。

## 提示词缓存（Prompt Cache）

Prompt Cache 默认关闭。在 `ccline --config` 中启用 **Prompt Cache**，按 **S** 保存。它读取原生 [`prompt_cache`](https://code.claude.com/docs/en/statusline#prompt-cache-fields) 对象（Claude Code 2.1.251+），不请求 API，也不读取转录文件。

例如 `Cache 91% · warm · 58m · 2 misses · last: tools_changed`，依次表示 token 命中比例、缓存状态、到期倒计时，以及未命中次数和宿主诊断的最近一次未命中的可能原因。原因字段需要 Claude Code 2.1.260+。统计仅覆盖主会话，不包含子代理；百分比表示输入 token 中从缓存读取的比例，不是请求命中率。

命中比例为 null 时显示 `?`。`unobserved` 表示本会话尚未报告缓存 token，可能未启用缓存，也可能供应商未报告；其余情况显示 `warm` 或 `cold`。宿主报告的到期时间经过后显示 `cold`，仅在缓存为 warm 且到期时间已知时显示倒计时。对象缺失或为 null 时隐藏段落；未命中次数非零时才显示未命中信息。空闲时的倒计时刷新沿用 Usage 的 [`statusLine.refreshInterval`](#空闲时刷新) 设置。

## 思考强度（Effort）

显示 `effort.level` 报告的当前推理工作量档位，例如 `low`、`medium`、`high`、`xhigh` 或 `max`。要求 Claude Code 2.1.119 或更新版本；输入没有 `effort` 字段时省略该段落。ccline 不会根据模型名称、thinking 设置或 token 用量推断档位。所有内置主题默认在 Model 后启用 Effort。

运行 `ccline --config`，在段落列表中选择 **Effort**，按 **Enter** 可独立于 Model 开关。按 **Tab** 编辑图标、颜色、背景和文字样式，**Shift+Up/Down** 调整顺序，**S** 保存 `config.toml`。预览会随编辑实时更新。**W** 写入当前主题文件，供 `--theme` 启动时使用。

如果配置或所选主题中没有 Effort，TUI 会把它作为禁用选项显示在 Model 之后（没有 Model 时放在末尾）。所选内置主题下 Effort 使用闪电图标和独立的紫色配色；自定义主题使用默认的 Effort 外观。新增的 Effort 选项只在保存时写入。也可以直接在 `config.toml` 或主题文件中配置：

```toml
[[segments]]
id = "effort"
enabled = true
icon = { plain = "⚡", nerd_font = "\uf0e7" }
colors = { icon = { r = 189, g = 147, b = 249 }, text = { r = 189, g = 147, b = 249 } }
styles = { text_bold = false }
options = {}
```

设置 `enabled = false` 可隐藏 Effort。

## Agents

Agents 段落在模型名称、思考强度所在行显示图标和 `1 Agent`（多个时为 `2 Agents` 等）。数量只统计最近一次原生面板输入中的运行条目，不代表所有子代理。启用后还会为原生子代理树渲染行正文，显示带颜色的状态标记和已用时间；树形缩进与状态圆点由 Claude Code 提供。数量缺失、为零或已过期时隐藏该段落；行样式、缓存及过期配置见[子代理面板](subagent-panel.zh.md)。
