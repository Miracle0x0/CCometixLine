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

## 思考强度（Effort）

显示 `effort.level` 报告的当前推理工作量档位，例如 `low`、`medium`、`high`、`xhigh` 或 `max`。要求 Claude Code 2.1.119 或更新版本；输入没有 `effort` 字段时省略该段落。ccline 不会根据模型名称、thinking 设置或 token 用量推断档位。所有内置主题默认在 Model 后启用 Effort。

运行 `ccline --config`，在段落列表中选择 **Effort**，按 **Enter** 可独立于 Model 开关。按 **Tab** 编辑图标、颜色、背景和文字样式，**Shift+Up/Down** 调整顺序，**S** 保存 `config.toml`。预览会随编辑实时更新。**W** 写入当前主题文件，供 `--theme` 启动时使用。

如果配置或所选主题中没有 Effort，TUI 会把它作为禁用选项显示在 Model 之后（没有 Model 时放在末尾）。所选内置主题下 Effort 使用闪电图标和独立的紫色配色；自定义主题使用默认的 Effort 外观。新增的 Effort 选项只在保存时写入。也可以直接在 `config.toml` 或主题文件中配置：

```toml
[[segments]]
id = "effort"
enabled = true
icon = { plain = "⚡", nerd_font = "\uf0e7" }
colors = { icon = { c16 = 13 }, text = { c16 = 13 } }
styles = { text_bold = false }
options = {}
```

设置 `enabled = false` 可隐藏 Effort。

## Agents

Agents 段落在模型名称、思考强度所在行显示图标和 `1 Agent running`（多个时为 `2 Agents running` 等）。数量只统计最近一次原生面板输入中的运行条目，不代表所有子代理。启用后还会为 Claude Code 的子代理面板渲染无图标的文字行。数量缺失、为零或已过期时隐藏该段落；缓存及过期配置见[子代理面板](subagent-panel.zh.md)。
