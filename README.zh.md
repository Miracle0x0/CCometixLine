# CCometixLine

[English](README.md) | [中文](README.zh.md)

基于 Rust 的高性能 Claude Code 状态栏工具，集成 Git 信息、上下文与用量显示、Claude Code 子代理面板行渲染和交互式 TUI 配置。

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
- **上下文与用量显示** 直接读取 Claude Code 状态栏输入
- **目录显示** 显示当前工作空间
- **简洁设计** 使用 Nerd Font 图标

### 交互式 TUI 功能
- **交互式主菜单** 无输入时直接执行显示菜单
- **TUI 配置界面** 实时预览配置效果
- **主题系统** 多种内置预设主题
- **段落自定义** 精细化控制各段落
- **配置管理** 初始化、检查、编辑配置

## 安装

### GitHub Release（推荐）

从 [GitHub Releases](https://github.com/Miracle0x0/CCometixLine/releases/latest) 下载对应平台的压缩包：

| 平台 | 压缩包 |
| --- | --- |
| Linux x64 (glibc) | `ccline-linux-x64.tar.gz` |
| Linux x64 (musl/static) | `ccline-linux-x64-static.tar.gz` |
| Linux ARM64 (glibc) | `ccline-linux-arm64.tar.gz` |
| Linux ARM64 (musl/static) | `ccline-linux-arm64-static.tar.gz` |
| macOS Intel | `ccline-macos-x64.tar.gz` |
| macOS Apple Silicon | `ccline-macos-arm64.tar.gz` |
| Windows x64 | `ccline-windows-x64.zip` |

#### Linux / macOS

将 `archive` 设为表格中的对应文件名，然后下载并解压：

```bash
archive=ccline-linux-x64.tar.gz
mkdir -p ~/.claude/ccline
curl -fL "https://github.com/Miracle0x0/CCometixLine/releases/latest/download/$archive" -o "$archive"
tar -xzf "$archive" --strip-components=1 -C ~/.claude/ccline
chmod +x ~/.claude/ccline/ccline
export PATH="$HOME/.claude/ccline:$PATH"
```

#### Windows

```powershell
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.claude\ccline"
Invoke-WebRequest -Uri "https://github.com/Miracle0x0/CCometixLine/releases/latest/download/ccline-windows-x64.zip" -OutFile "ccline-windows-x64.zip"
Expand-Archive -Path "ccline-windows-x64.zip" -DestinationPath . -Force
Copy-Item -Path ".\ccline-windows-x64\ccline.exe" -Destination "$env:USERPROFILE\.claude\ccline\ccline.exe" -Force
$env:PATH = "$env:USERPROFILE\.claude\ccline;$env:PATH"
```

以上命令会让 `ccline` 在当前终端中可用。若要在后续终端中使用，请将 `~/.claude/ccline`（Windows：`%USERPROFILE%\.claude\ccline`）加入用户 `PATH`。

### Claude Code 配置

添加到 Claude Code `settings.json`：

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/ccline/ccline",
    "padding": 0
  }
}
```

运行 `ccline -c` 打开配置面板并选择主题。

### 更新

再次执行对应平台的下载和解压命令，即可用最新 GitHub Release 替换已安装的程序。状态栏的 Update 段落会查询本仓库的 Release，提示可用的新版本。

### 从源码构建

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

## 使用

### 主题覆盖

```bash
# 临时使用指定主题（覆盖配置文件设置）
ccline --theme cometix
ccline --theme minimal
ccline --theme gruvbox
ccline --theme nord
ccline --theme powerline-dark

# 或使用主题目录下的自定义主题（见“配置”）
ccline --theme my-custom-theme
```

### 详细输出与已移除的补丁器

旧版本提供 `ccline --patch` 修改 Claude Code 的 `cli.js`。Claude Code 现在以原生二进制分发，不再包含 `cli.js`，因此补丁器已移除。需要详细输出时，使用 Claude Code 自带的设置：执行 `/config verbose=true`，或在 `/config` 的 Config 标签页中切换 verbose 输出（[官方文档](https://code.claude.com/docs/en/settings#use-the-config-menu)）。ccline 不再隐藏 "Context low" 警告，也不再修改 Claude Code 的其他行为；目前未确认这些补丁有官方替代方式。

## 默认段落

显示：`模型 | Effort | 目录 | Git 分支状态 | 上下文窗口`

- **Git**：分支名；`✓` 清洁，`●` 有更改，`⚠` 冲突；`↑n` 领先，`↓n` 落后
- **模型**：简化名称，例如 `claude-sonnet-5-5` → `Sonnet 5.5`，可在 `models.toml` 中自定义
- **Effort**：当前推理工作量档位，例如 `high`
- **上下文窗口**：Claude Code 报告的上下文占用，例如 `20% · 200k tokens`；首次响应前及 `/compact` 刚结束时显示 `-`

可选段落包括 **Usage**（订阅窗口和网关预算，例如 `63% · 24% · 2h`）、Prompt Cache、Session Name、Worktree、Pull Request、Session、Cost、Output Style 和 Update。各段落的数据来源、格式和选项见[段落说明](docs/segments.zh.md)。如需在会话空闲时让 Usage 倒计时保持更新，请自行在 `statusLine` 中设置 `refreshInterval`；ccline 不管理该设置。

## 配置

- **交互式 TUI**：`ccline --config` 实时编辑并预览
- **文件**：`<配置目录>/ccline/config.toml`、`themes/*.toml` 和 `models.toml`
- **初始化**：主菜单中的 **Initialize Config**（无输入直接运行 `ccline`）

`<配置目录>` 在设置了 [`CLAUDE_CONFIG_DIR`](https://code.claude.com/docs/en/env-vars) 时为该目录，否则为 `~/.claude`。渲染只读取这些文件，文件无法读取或解析时明确报错，不回退到默认值。目录规则、初始化和 `models.toml` 见[配置说明](docs/configuration.zh.md)。

## 子代理面板

在 `ccline --config` 中启用 **Agents** 并按 **S**，ccline 会把 `ccline --subagent-statusline` 安装为 Claude Code 的 [`subagentStatusLine`](https://code.claude.com/docs/en/statusline#subagent-status-lines)，并渲染提示符下方子代理面板的每一行：

```text
Explore · find config loading · 1m20s · 12.4k (6%) · Sonnet 5.5 · high
```

已有且不是 ccline 写入的 `subagentStatusLine` 会被报告为配置冲突并保持不变。行字段、宽度处理、所有权规则以及从已移除的 Hooks 和 Mod 后端迁移，见[子代理面板](docs/subagent-panel.zh.md)。

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

### 发布

修改 `Cargo.toml` 中的 `[package].version`，运行 `cargo check` 同步 `Cargo.lock`，然后提交这两个文件并推送到 `main` 或 `master`。发布工作流比较推送前后的包版本号；版本变化时构建七个平台的压缩包，并在本次推送的提交上创建 `v<版本号>` 标签及 GitHub Release。仅修改依赖不会发布版本，无需手动推送标签。

## 路线图

- [x] TOML 配置文件支持
- [x] TUI 配置界面
- [x] 自定义主题
- [x] 交互式主菜单

## 贡献

欢迎贡献！请随时提交 issue 或 pull request。

## 许可证

本项目采用 [MIT 许可证](LICENSE)。

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Haleclipse/CCometixLine&type=Date)](https://star-history.com/#Haleclipse/CCometixLine&Date)
