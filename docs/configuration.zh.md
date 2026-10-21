# 配置说明

[English](configuration.md) | [中文](configuration.zh.md)

## 配置目录

ccline 读写的所有文件都位于 Claude Code 的配置目录下，下文记作 `<配置目录>`：设置了 [`CLAUDE_CONFIG_DIR`](https://code.claude.com/docs/en/env-vars) 时为该目录，否则为 `~/.claude`。

| 文件 | 用途 |
| --- | --- |
| `<配置目录>/ccline/config.toml` | 主配置：样式、段落和主题名称 |
| `<配置目录>/ccline/themes/*.toml` | 内置和自定义主题文件 |
| `<配置目录>/ccline/models.toml` | 模型显示名称，见[模型名称](#模型名称modelstoml) |
| `<配置目录>/ccline/agents/*.json` | 按会话保存的可见运行子代理数量及观测时间 |
| `<配置目录>/ccline/agents-installation.json` | ccline 安装的子代理面板命令，见[子代理面板](subagent-panel.zh.md) |
| `<配置目录>/ccline/.update_state.json` | Update 段落的更新检查状态 |
| `<配置目录>/settings.json` | Claude Code 设置；ccline 只修改其中的 `subagentStatusLine` |

ccline 从自身运行环境读取 `CLAUDE_CONFIG_DIR`。以 `CLAUDE_CONFIG_DIR=/path/to/dir` 启动的 Claude Code 实例所运行的状态栏命令会继承该变量，因此读取 `/path/to/dir/ccline/`，启用 Agents 时修改 `/path/to/dir/settings.json`，不会改动默认的 `~/.claude`。为这类实例运行 `ccline --config` 时，请在 shell 中设置同样的变量。`CLAUDE_CONFIG_DIR` 必须是绝对路径，空值或相对路径会明确报错；支持包含空格的路径。二进制文件本身可以放在 `PATH` 中的任意位置。

## 初始化与错误

渲染时只读取配置，不修改配置文件。启用 Agents 后，子代理面板会写入按会话保存的数量快照，主状态栏读取该快照。内置主题文件和 `models.toml` 模板在缺失时由主菜单的 **Initialize Config**（无输入直接运行 `ccline`）以及 TUI（`ccline --config`）启动时创建。这两个操作也会清理已有的 `models.toml`，见[过时字段](#过时字段)。主菜单的 **Check Configuration** 会同时检查 `config.toml` 和 `models.toml`。TUI 中 **S** 保存 `config.toml`，**W** 和 **Ctrl+S** 保存主题文件。

缺少 `config.toml` 时使用默认主题，缺少 `models.toml` 时使用内置模型名称。文件存在但无法读取或解析时会明确报错，不回退到默认值。这同样适用于 `--theme` 选择的主题文件：主题目录中的文件优先于同名内置主题，未知主题名会报错。

## 段落

所有段落都可以开关，并配置图标、颜色、背景和文字样式；在 TUI 中用 **Shift+Up/Down** 调整顺序。支持的段落：Directory、Git、Model、Effort、Agents、Context Window、Usage、Session、Cost、Output Style、Update。各数据段落的显示内容见[段落说明](segments.zh.md)。

## 模型名称（`models.toml`）

该文件用于自定义模型 ID 的显示名称。Claude 模型（Sonnet、Opus、Haiku、Fable）会自动识别并提取版本号，只有覆盖内置名称或使用第三方模型时才需要配置。上下文窗口大小来自 Claude Code 的输入，不在此配置。未知字段会被拒绝。

```toml
# 模型条目：按模型 ID 子串匹配
# 优先于内置的 Claude 模型识别
[[models]]
pattern = "glm-4.5"
display_name = "GLM-4.5"

[[models]]
pattern = "kimi-k2"
display_name = "Kimi K2"

# 上下文修饰符：独立匹配，可与模型条目组合
# 在显示名称后追加 display_suffix
# 例如模型 "Opus 4" + 修饰符 " 1M" = "Opus 4 1M"
[[context_modifiers]]
pattern = "[1m]"
display_suffix = " 1M"
```

该文件中的条目优先于内置条目。Model 段落和子代理面板行使用同一套名称。

### 过时字段

旧版本要求每个条目都填写 `context_limit`。该字段仍存在时，渲染会报错，错误信息会指出该字段并提示下面的自动清理。

**Initialize Config** 和启动 `ccline --config` 会删除所有 `[[models]]` 和 `[[context_modifiers]]` 条目中的 `context_limit`，包括 `models = [{ ... }]` 形式的内联条目。文件中的其他内容，包括注释和顺序，都保持原样。清理前会先把原文件备份为同目录下的 `models.toml.<时间戳>.bak`，TUI 或主菜单会报告删除了多少个字段以及备份位置。没有需要删除的字段时不会改动文件。如果清理后文件仍然无效（例如还有其他未知字段），则不写入任何内容，在错误中指出剩下的问题，并且在修复该文件之前 TUI 不会启动。提到 `context_limit` 的注释不会被修改。
