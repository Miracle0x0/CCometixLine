# CCometixLine

[English](README.md) | [中文](README.zh.md)

基于 Rust 的高性能 Claude Code 状态栏工具，集成 Git 信息、使用量跟踪、交互式 TUI 配置和 Claude Code 补丁工具。

![Language:Rust](https://img.shields.io/static/v1?label=Language&message=Rust&color=orange&style=flat-square)
![License:MIT](https://img.shields.io/static/v1?label=License&message=MIT&color=blue&style=flat-square)

## 截图

![CCometixLine](assets/img1.png)

状态栏显示：模型 | 目录 | Git 分支状态 | 上下文窗口信息

## 特性

### 核心功能
- **Git 集成** 显示分支、状态和跟踪信息
- **模型显示** 简化的 Claude 模型名称
- **思考强度显示** 显示当前推理工作量档位，支持独立开关
- **使用量跟踪** 基于转录文件分析  
- **目录显示** 显示当前工作空间
- **简洁设计** 使用 Nerd Font 图标

### 交互式 TUI 功能
- **交互式主菜单** 无输入时直接执行显示菜单
- **TUI 配置界面** 实时预览配置效果
- **主题系统** 多种内置预设主题
- **段落自定义** 精细化控制各段落
- **配置管理** 初始化、检查、编辑配置

### Claude Code 增强
- **禁用上下文警告** 移除烦人的"Context low"消息
- **启用详细模式** 增强输出详细信息
- **稳定补丁器** 适应 Claude Code 版本更新
- **自动备份** 安全修改，支持轻松恢复

## 安装

### 快速安装（推荐）

通过 npm 安装（适用于所有平台）：

```bash
# 全局安装
npm install -g @cometix/ccline

# 或使用 yarn
yarn global add @cometix/ccline

# 或使用 pnpm
pnpm add -g @cometix/ccline
```

使用镜像源加速下载：
```bash
npm install -g @cometix/ccline --registry https://registry.npmmirror.com
```

安装后：
- ✅ 全局命令 `ccline` 可在任何地方使用
- ⚙️ 按照下方提示进行配置以集成到 Claude Code
- 🎨 运行 `ccline -c` 打开配置面板进行主题选择

### Claude Code 配置

添加到 Claude Code `settings.json`：

**跨平台通用（推荐）**
```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/ccline/ccline",
    "padding": 0
  }
}
```

> **Windows 用户注意：** 从 Claude Code v2.1.47+ 开始，Windows 上支持 Unix 风格路径解析。`~` 符号会自动展开为您的用户主目录。**请勿使用 `%USERPROFILE%`** — 它在 v2.1.47+ 版本中不再可靠。
> - 推荐：`~/.claude/ccline/ccline`（跨平台通用）
> - 备选：`"ccline"`（需要 npm 全局安装）

**后备方案 (npm 安装):**
```json
{
  "statusLine": {
    "type": "command",
    "command": "ccline",
    "padding": 0
  }
}
```
*如果 npm 全局安装已在 PATH 中可用，则使用此配置*

### 更新

```bash
npm update -g @cometix/ccline
```

<details>
<summary>手动安装（点击展开）</summary>

或者从 [Releases](https://github.com/Haleclipse/CCometixLine/releases) 手动下载：

#### Linux

#### 选项 1: 动态链接版本（推荐）
```bash
mkdir -p ~/.claude/ccline
wget https://github.com/Haleclipse/CCometixLine/releases/latest/download/ccline-linux-x64.tar.gz
tar -xzf ccline-linux-x64.tar.gz
cp ccline ~/.claude/ccline/
chmod +x ~/.claude/ccline/ccline
```
*系统要求: Ubuntu 22.04+, CentOS 9+, Debian 11+, RHEL 9+ (glibc 2.35+)*

#### 选项 2: 静态链接版本（通用兼容）
```bash
mkdir -p ~/.claude/ccline
wget https://github.com/Haleclipse/CCometixLine/releases/latest/download/ccline-linux-x64-static.tar.gz
tar -xzf ccline-linux-x64-static.tar.gz
cp ccline ~/.claude/ccline/
chmod +x ~/.claude/ccline/ccline
```
*适用于任何 Linux 发行版（静态链接，无依赖）*

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
# 创建目录并下载
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.claude\ccline"
Invoke-WebRequest -Uri "https://github.com/Haleclipse/CCometixLine/releases/latest/download/ccline-windows-x64.zip" -OutFile "ccline-windows-x64.zip"
Expand-Archive -Path "ccline-windows-x64.zip" -DestinationPath "."
Move-Item "ccline.exe" "$env:USERPROFILE\.claude\ccline\"
```

</details>

### 从源码构建

```bash
git clone https://github.com/Haleclipse/CCometixLine.git
cd CCometixLine
cargo build --release
cp target/release/ccometixline ~/.claude/ccline/ccline
```

## 使用

### 主题覆盖

```bash
# 临时使用指定主题（覆盖配置文件设置）
ccline --theme cometix
ccline --theme minimal
ccline --theme gruvbox
ccline --theme nord
ccline --theme powerline-dark

# 或使用 ~/.claude/ccline/themes/ 目录下的自定义主题
ccline --theme my-custom-theme
```

### Claude Code 增强

```bash
# 禁用上下文警告并启用详细模式
ccline --patch /path/to/claude-code/cli.js

# 常见安装路径示例
ccline --patch ~/.local/share/fnm/node-versions/v24.4.1/installation/lib/node_modules/@anthropic-ai/claude-code/cli.js
```

## 默认段落

显示：`模型 | Effort | 目录 | Git 分支状态 | 上下文窗口`

### Git 状态指示器

- 带 Nerd Font 图标的分支名
- 状态：`✓` 清洁，`●` 有更改，`⚠` 冲突
- 远程跟踪：`↑n` 领先，`↓n` 落后

### 模型显示

显示简化的 Claude 模型名称：
- `claude-3-5-sonnet` → `Sonnet 3.5`
- `claude-4-sonnet` → `Sonnet 4`

### 上下文窗口显示

基于转录文件分析的令牌使用百分比，包含上下文限制跟踪。

### 思考强度显示

显示 Claude Code 报告的当前推理工作量档位，例如 `low`、`medium`、`high`、`xhigh` 或 `max`。要求 Claude Code 2.1.119 或更新版本；输入没有 `effort` 字段时省略该段落。所有内置主题默认在 Model 后启用 Effort。

## 配置

CCometixLine 支持通过 TOML 文件和交互式 TUI 进行完整配置：

- **配置文件**: `~/.claude/ccline/config.toml`
- **交互式 TUI**: `ccline --config` 实时编辑配置并预览效果
- **主题文件**: `~/.claude/ccline/themes/*.toml` 自定义主题文件
- **自动初始化**: `ccline --init` 创建默认配置

### 可用段落

所有段落都支持配置：
- 启用/禁用切换
- 自定义分隔符和图标
- 颜色自定义
- 格式选项

支持的段落：目录、Git、模型、Effort、Agents、上下文窗口、使用量、会话、成本、输出样式、更新

### Effort 配置

运行 `ccline --config`，在段落列表中选中 **Effort**，按 **Enter** 独立切换显示状态，不影响 Model。按 **Tab** 编辑图标、颜色、背景和文字样式，按 **Shift+上/下方向键** 调整顺序，按 **S** 保存到 `config.toml`。预览随编辑实时更新。**W** 将配置写入当前主题文件，用于通过 `--theme` 指定主题的情况。

如果当前配置或选中的主题没有 Effort，TUI 会在 Model 后显示一个未启用的 Effort 选项；没有 Model 时放在列表末尾。Effort 使用闪电图标和对应内置主题的独立紫色系配色；自定义主题使用默认 Effort 样式。按 **Enter** 启用，再按 **S** 保存配置或 **W** 写入当前主题即可。新增的 Effort 选项只在保存时写入文件。也可以直接在 `config.toml` 或主题文件中配置：

```toml
[[segments]]
id = "effort"
enabled = true
icon = { plain = "⚡", nerd_font = "\uf0e7" }
colors = { icon = { c16 = 13 }, text = { c16 = 13 } }
styles = { text_bold = false }
options = {}
```

设置 `enabled = false` 即可隐藏 Effort。显示值直接来自状态栏输入的 `effort.level`，不会根据模型名、思考开关或 token 用量推断。

### 子 agent 运行状态

**所有内置主题默认关闭 Agents。** 运行 `ccline --config`，选中 **Agents**，按 **Enter** 开启，再按 **S** 保存。保存时会在 `~/.claude/settings.json` 安装 `SessionStart`、`SubagentStart`、`SubagentStop` 和 `SessionEnd` 命令 hooks。没有配置状态栏时会同时配置 ccline；已有状态栏命令保持不变。未设置 `refreshInterval` 时会添加两秒刷新，使主会话空闲时也能更新后台活动。

关闭 **Agents** 并按 **S** 保存即可卸载对应 hooks，保留其他 hooks 和设置。本功能添加的刷新间隔若未被修改，会在卸载时移除；原有或后来手动修改的间隔保持不变。未保存的切换只影响预览。**W** 和 **Ctrl+S** 只保存主题文件，不改变 hooks 安装状态；**S** 才会应用活动开关。配置或主题没有 Agents 条目时，TUI 会将其作为默认关闭的选项显示。

观察到活跃子 agent 时，主状态栏显示例如 `Agents: 4 active · 1 responded · reviewer 1m20s · Explore 35s · Plan 12s · +1 more`。活跃 agent 按运行时长从长到短排列，`max_agents` 是显示名称数量的上限，默认 `3`。每次刷新会根据终端列宽，在保留其他段落、图标、分隔符和数量摘要的前提下，从末尾收起放不下的完整“名称 + 耗时”，并更新 `+N more`。没有名称能放下或 `max_agents = 0` 时只显示数量；没有活跃子 agent 时整个段落及其分隔符隐藏。若其余段落和数量摘要本身已经超宽，收起名称无法避免宿主裁剪。

在 TUI 中选中 **Agents**，按 **Tab** 进入设置，选中 **Max agents** 并按 **Enter** 输入非负整数；**Ctrl+U** 清空输入，**Enter** 应用，**Esc** 取消，**S** 保存到 `config.toml`。无效输入会显示错误并保留编辑框。颜色、图标、背景、文字样式和排序仍使用原有控件；预览使用示例活动，按预览区域的实际宽度收起名称。也可以直接编辑配置或主题中的 Agents 条目：

```toml
[[segments]]
id = "agents"
enabled = true
icon = { plain = "A", nerd_font = "\uf0c0" }
colors = { icon = { c16 = 6 }, text = { c16 = 6 } }
styles = { text_bold = false }
options = { max_agents = 3 }
```

自动布局读取 Claude Code 每次调用时设置的 [`COLUMNS`](https://code.claude.com/docs/en/statusline#how-status-lines-work)，按终端显示列计算中文、组合字符和 emoji，ANSI 颜色不占列宽。默认扣除 Claude Code 左右各两列留白；设置了 `statusLine.padding` 时，将状态栏命令设为 `ccline --width-offset N`，其中 `N = 4 + 2 × padding`，例如 `padding = 2` 对应 `--width-offset 8`。有活跃名称需要布局时，缺失或无效的 `COLUMNS` 会明确报错；手动运行时需显式提供列数。窗口尺寸变化在下次调用时生效。

记录按会话 ID 和 agent ID 隔离，并通过进程间文件锁处理并发事件。重复启动事件不会增加计数；恢复子 agent 时重新计时；压缩上下文保留活动记录，启动或恢复会话则重置该会话的观察记录。hook 命令为 `ccline --agents-hook`，事件数据通过 stdin 读取。所有活动记录直接保存在 `~/.claude/ccline/agents/` 下，关闭后重新开启会保留已有会话记录。`~/.claude/ccline/agents-installation.json` 保存本功能管理的 hook 命令和刷新间隔信息，用于卸载。

汇总依据[官方生命周期 hooks](https://code.claude.com/docs/en/hooks#subagentstart)。`responded` 表示收到 `SubagentStop`，不代表任务成功。这些事件无法可靠区分等待授权、失败、取消或其他 stop hook 要求继续运行的状态，因此使用可配置的活跃颜色，不推断这些细分状态。没有结束事件的异常中断无法从这份数据确认；安装前已在运行的 agent 不会通过扫描转录文件重建。状态或设置文件损坏时会明确报错。

### 模型配置 (`models.toml`)

文件位置：`~/.claude/ccline/models.toml`（首次运行时自动创建）

此文件配置模型 ID 的显示名称及其上下文窗口限制。Claude 模型（Sonnet、Opus、Haiku）会自动识别并提取版本号，此文件仅用于覆盖默认行为或添加第三方模型支持。

```toml
# 模型条目：基于模型 ID 的子字符串匹配
# 优先级高于内置 Claude 模型识别
[[models]]
pattern = "glm-4.5"
display_name = "GLM-4.5"
context_limit = 128000

[[models]]
pattern = "kimi-k2"
display_name = "Kimi K2"
context_limit = 128000

# 上下文修饰符：独立匹配，可与模型条目组合使用
# 覆盖 context_limit 并将 display_suffix 追加到显示名称
# 例如：模型 "Opus 4" + 修饰符 " 1M" = "Opus 4 1M"
[[context_modifiers]]
pattern = "[1m]"
display_suffix = " 1M"
context_limit = 1000000
```


## 系统要求

- **Git**: 版本 1.5+ (推荐 Git 2.22+ 以获得更好的分支检测)
- **终端**: 必须支持 Nerd Font 图标正常显示
  - 安装 [Nerd Font](https://www.nerdfonts.com/) 字体
  - 中文用户推荐: [Maple Font](https://github.com/subframe7536/maple-font) (支持中文的 Nerd Font)
  - 在终端中配置使用该字体
- **Claude Code**: 用于状态栏集成

## 开发

```bash
# 构建开发版本
cargo build

# 运行测试
cargo test

# 构建优化版本
cargo build --release
```

## 路线图

- [x] TOML 配置文件支持
- [x] TUI 配置界面
- [x] 自定义主题
- [x] 交互式主菜单
- [x] Claude Code 增强工具

## 贡献

欢迎贡献！请随时提交 issue 或 pull request。

## 许可证

本项目采用 [MIT 许可证](LICENSE)。

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Haleclipse/CCometixLine&type=Date)](https://star-history.com/#Haleclipse/CCometixLine&Date)
