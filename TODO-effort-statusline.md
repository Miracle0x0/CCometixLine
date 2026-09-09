# 思考强度状态栏显示：实现说明与 TODO

## 1. 结论与核查范围

**已在 `feat/effort-statusline` 分支实现独立的 `effort` 段落。** 显示值直接来自 Claude Code 状态栏输入中的 `effort.level`，支持现有配置中的启用开关、排序、图标、颜色和文字样式，并已接入九个内置主题及 TUI 预览。实现没有新增依赖、网络查询或转录文件扫描。

Claude Code 官方变更日志明确记录，`2.1.119` 为状态栏 stdin JSON 加入了 `effort.level` 和 `thinking.enabled`。本功能基于该正式接口，支持基线为 Claude Code `2.1.119`。不设计旧版本数据获取方案，也不增加运行时版本阻断。[官方变更日志：2.1.119](https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md#21119)

本次核查日期为 2026-09-09，项目版本为 `1.1.2`，源码基线为 `master` 的 `580931a81bc8463386af1eaa3be41ca959fedff9`。本机 `claude --version` 返回 `2.1.263 (Claude Code)`，满足上述版本条件。已通过 12 项自动化测试、10 项 release CLI 输入输出检查、格式检查、严格 Clippy 检查和 Linux release 构建。TUI 除 ratatui TestBackend 测试外，还使用当前用户的实际配置完成了 PTY 交互验证：打开界面、启用 Effort、切换磁盘主题、重置主题以及不保存退出，并在 256 色终端中核对了闪电图标、Effort 紫色与 Model 橙色的独立显示；配置和九份主题文件的字节内容均未改变。CLI 检查使用 README 中的 TOML 片段与实际 stdin/stdout。尚未采集真实 Claude Code 会话的状态栏输入；本文中的 JSON 是说明用样例，不代表已完成上游交互联调。

## 2. 显示内容与数据来源

这里的“思考强度”指 Claude Code 报告的当前推理工作量档位，即 reasoning effort。官方状态栏文档列出的值为 `low`、`medium`、`high`、`xhigh`、`max`，字段会反映会话中的 `/effort` 修改；当前模型不支持 effort 时，整个 `effort` 对象可以缺失。`thinking.enabled` 则表示会话是否开启扩展思考，是另一个独立字段。[官方状态栏字段说明](https://code.claude.com/docs/en/statusline#available-data)

与本项目输入结构对应的示意 JSON 如下：

```json
{
  "model": {
    "id": "claude-opus-4-6",
    "display_name": "Opus 4.6"
  },
  "workspace": {
    "current_dir": "/work/project"
  },
  "transcript_path": "/work/session.jsonl",
  "effort": {
    "level": "high"
  },
  "thinking": {
    "enabled": true
  }
}
```

Effort 是模型行为的调节参数，并不是实际思考 token 数、耗时或严格的 token 预算，而且在未开启 thinking 时仍可影响输出。因此不能把 `thinking.enabled = false` 转换为 `effort = low` 或 `effort = off`，也不能通过 token 用量反推出档位。[官方 Effort 说明](https://platform.claude.com/docs/en/build-with-claude/effort#how-effort-works)

| 候选来源 | 能提供的信息 | 本功能的选择 |
| --- | --- | --- |
| stdin 的 `effort.level` | Claude Code 报告的当前会话档位 | 唯一数据源 |
| stdin 的 `thinking.enabled` | 扩展思考开关 | 可作为后续独立显示项，不用于计算 effort |
| `CLAUDE_CODE_EFFORT_LEVEL`、`settings.json` 中的 `effortLevel` 等配置 | 努力程度的配置输入 | 不在 ccline 中重新解析优先级或据此替代会话值 |
| transcript、用量统计 | 历史消息及其用量，部分版本还可能记录消息档位 | 不用于冒充当前会话设置 |
| 模型 ID、`models.toml`、名称中的 `-thinking` | 模型身份、显示名称和上下文配置 | 不用于推断档位 |

Claude Code 的 effort 可由会话命令、启动参数、环境变量、配置文件以及 skill/subagent 设置等多处影响。由 ccline 自行合并这些来源，会重复实现上游状态管理；读取 stdin 可以直接消费上游给出的结果。[官方模型配置说明](https://code.claude.com/docs/en/model-config#effort-level)

显示语义应限于“本次状态栏输入报告的 effort”。它不是每个内部请求或子代理的独立监控值，也不能证明某个第三方网关最终采用了相同参数；如需核查这些情况，应在真实联调中分别确认，不能从状态栏字段扩大推论。

## 3. 实现结构

目前的数据流为：

```text
Claude Code stdin JSON
          |
          v
serde_json::from_reader -> InputData
          |
          v
collect_all_segments(config, input)
          |
          v
Vec<(SegmentConfig, SegmentData)>
          |
          v
StatusLineGenerator::generate -> stdout
```

| 位置 | 实现行为 | Effort 接入方式 |
| --- | --- | --- |
| [src/main.rs](src/main.rs)，`main` | 从 stdin 反序列化 `InputData`，随后采集并渲染；输入解析错误通过 `?` 返回 | 保留入口流程即可 |
| [src/config/types.rs](src/config/types.rs)，`InputData` | 声明 `model`、`effort`、`workspace`、`transcript_path`、`cost`、`output_style` | `effort` 为可选对象，内部 `level` 为必填字符串；thinking 不参与采集 |
| [src/config/types.rs](src/config/types.rs)，`SegmentId` | 枚举包含十种段落，使用 `snake_case` 序列化 | TOML 中使用 `id = "effort"` |
| [src/core/segments/mod.rs](src/core/segments/mod.rs) | `Segment::collect` 返回 `Option<SegmentData>`；数据由 `primary`、`secondary`、`metadata` 组成 | 导出新的 `EffortSegment` |
| [src/core/statusline.rs](src/core/statusline.rs)，`collect_all_segments` | 遍历配置中的已启用段落，以 `SegmentId` 分派采集，只保留 `Some(data)` | 已注册 `SegmentId::Effort` 分支 |
| [src/core/statusline.rs](src/core/statusline.rs)，`render_segment` | 通用渲染 primary/secondary、图标、颜色与背景 | 无需增加 effort 专用渲染分支 |
| [src/core/segments/effort.rs](src/core/segments/effort.rs)，`EffortSegment::collect` | 将 `effort.level` 写入 primary，字段缺失时返回 `None` | 独立于 Model 的显示与启用状态 |
| [src/config/models.rs](src/config/models.rs) | 根据模型 ID 匹配显示名称和上下文；`-thinking` 仅作为名称解析边界 | 当前没有思考档位识别机制 |
| [src/ui/components/preview.rs](src/ui/components/preview.rs) | 预览自行构造各段落的示例数据，不经过真实输入采集 | 使用 high 作为 Effort 设计预览值；真实渲染另有 CLI 测试 |

### 主题与配置的实际约束

[src/ui/themes/presets.rs](src/ui/themes/presets.rs) 中的 `get_theme()` 优先加载 `~/.claude/ccline/themes/<name>.toml`，九个内置主题的段落列表则分别定义在各个 `get_*()` 中。[src/config/loader.rs](src/config/loader.rs) 仅在文件不存在时创建主题文件，已有的 `config.toml` 也不会自动补入新段落。

**TUI 将 Effort 作为独立的可配置选项提供，不要求配置文件预先包含该条目。** `App::with_effort_option()` 在打开配置、切换主题和重置主题时准备编辑器中的 Effort：已有条目保持原来的位置、开关和样式；缺失时在 Model 后显示为未启用，使用对应内置主题的独立 Effort 样式；自定义主题使用默认 Effort 样式，没有 Model 时放在列表末尾。准备过程只修改编辑器内存，按 S 或 W 保存后才写入相应文件；状态栏运行路径仍只按已保存的配置采集和显示。

TUI 的名称映射已补齐到 [segment_list.rs](src/ui/components/segment_list.rs)、[settings.rs](src/ui/components/settings.rs) 和 [app.rs](src/ui/app.rs) 的两个启用提示分支。Enter 独立切换开关，Tab 进入样式设置，Shift+方向键排序，S 保存配置，W 写入当前主题。段落列表使用有选中状态的渲染，在窄窗口中也能滚动到选中的 Effort。首版没有新增私有 options。

TUI 启动时直接读取 `config.toml`，不再用关联主题文件覆盖它，避免保存过的 Effort 开关在重新打开界面后被主题默认值替换。配置解析错误直接返回。

## 4. 显示与输入契约

Effort 使用独立段落，九个内置主题均默认启用并放在 Model 后。普通模式使用闪电 `⚡`，Nerd Font 与 Powerline 模式使用 `\u{f0e7}`，各内置主题为 Effort 设置独立的紫色系配色；带背景的主题同时设置独立背景色。TUI 新增 Effort 时通过 `ThemePresets::effort_segment()` 选择相应默认样式，不复制 Model 的颜色或文字样式。用户可以独立修改 Effort 的全部样式属性。首版只显示 effort。

输入模型增加了 `effort: Option<Effort>`，其中 `Effort` 使用 `#[derive(Deserialize)]`，包含必填的 `level: String`。保留上游档位文本，不在 ccline 中推导模型默认值、合并档位或重新计算档位。外层 `Option` 表达协议允许对象缺失；内层必填字段让缺失或类型错误显式暴露。

`EffortSegment::collect()` 在有值时将 `level` 写入 `SegmentData.primary`，secondary 留空；协议没有提供 effort 时返回 `None`，不生成该段落。不能仅凭字段缺失就断言具体原因，也不能将其补成 `high`、`auto` 或上次值。该行为直接使用现有的可选段落语义，不建立第二数据源或缓存。

无数据时应在采集阶段返回 `None`，而不是返回 primary 为空的 `Some(SegmentData)`：当前渲染器仍会为后者输出图标、空格，甚至背景块。采集阶段省略段落，也能让普通分隔符和 Powerline 颜色过渡只处理实际存在的段落。

每次调用仅使用本次 stdin。ccline 是读取输入、输出一行后退出的程序，新增段落不需要后台任务或轮询；界面何时刷新仍由 Claude Code 的调用时机决定，不能仅凭静态分析承诺修改 `/effort` 后的即时刷新延迟。

## 5. 实施 TODO

### P0：确认真实输入与显示契约

- [x] 核对官方字段名、字段语义和引入版本 `2.1.119`。
- [x] 核对本机 Claude Code 版本以及项目输入、采集、渲染、主题和 TUI 接入点。
- [ ] 在测试会话中采集实际状态栏输入，仅保留用于验证的版本、模型、effort 和 thinking 字段；确认启动时以及 `/effort`、`/model` 修改后的载荷。
- [x] 明确首版采用独立段落，并固定各主题的默认启用状态、位置和图标。

### P1：完成运行路径

- [x] 在 `src/config/types.rs` 增加 `Effort`、`InputData.effort` 和 `SegmentId::Effort`。
- [x] 新增 `src/core/segments/effort.rs`，实现无额外 I/O 的 `EffortSegment`。
- [x] 在 `src/core/segments/mod.rs` 声明模块并导出类型，在 `collect_all_segments()` 增加对应分支。
- [x] 沿用 `main()` 的输入解析错误路径，确保错误类型的 effort 对象或 level 字段不会被默认值掩盖。

### P2：补齐主题、TUI 与使用文档

- [x] 为 `src/ui/themes/theme_*.rs` 的九个内置主题定义 Effort 段落，并在 `presets.rs` 的全部主题列表中加入它。
- [x] 补齐 `segment_list.rs`、`settings.rs` 和 `app.rs` 两处提示文本的 `SegmentId::Effort` 分支。
- [x] 在 `preview.rs` 增加 Effort 的设计预览数据，确认排序、禁用、样式编辑和窄窗口滚动的表现。
- [x] 在 README 的可用段落与配置说明中加入 Effort、上游版本要求及完整 TOML 片段；片段包含现有 `SegmentConfig` 所要求的图标、颜色、样式和 options 结构。
- [x] 验证配置或磁盘主题不含 Effort 时，打开、切换和重置后的 TUI 均提供未启用的 Effort；启用和保存仍使用原有操作，已有条目的开关与自定义样式保持不变。

### P3：验证与验收

- [x] 在相关 `src` 文件内补充输入解析、采集、渲染及 TUI 测试，共 12 项；覆盖缺失 Effort、没有 Model、自定义样式、保存后重开和磁盘主题切换与重置。
- [ ] 在真实 Claude Code 会话中验证档位切换、模型切换和两个同时运行的会话；记录输入变化与屏幕刷新之间的实际关系。
- [x] 通过 `cargo test --locked --all-targets`、`cargo fmt --all -- --check`、`cargo clippy --locked --all-targets -- -D warnings` 和 `cargo build --release --locked`。为通过当前工具链的 Clippy，等价简化了已有 Cost、Session、Usage 的可选值读取及补丁排序写法。
- [x] 使用 release 二进制验证五种档位、字段缺失、独立禁用和三种错误输入，共 10 个 CLI 场景。
- [x] 使用当前用户配置进行真实 PTY 交互验证，确认缺失 Effort 时可直接在 TUI 启用，切换和重置后仍可配置；退出而不保存时磁盘文件不变。
- [ ] 在 macOS、Windows 环境完成构建与终端联调；本次只完成 Linux 本地构建。

## 6. 验收矩阵

| 场景 | 预期结果 |
| --- | --- |
| 输入分别包含 `low`、`medium`、`high`、`xhigh`、`max` | 正文准确显示收到的档位，不合并 `xhigh` 与 `max` |
| `effort` 对象缺失 | 采集结果没有 Effort 段落，没有残留图标、空背景块或多余分隔符 |
| `effort: {}`、`effort.level` 为数字或 effort 为错误结构 | 输入解析失败；沿用现有错误返回，不显示默认档位 |
| 有合法 effort，同时 `thinking.enabled = false` | 仍显示收到的 effort，不将其改为 off 或 low |
| 环境变量或磁盘配置与 stdin 中的 effort 不同 | 只显示 stdin 中的值 |
| 连续输入从 `high` 改为 `low` | 第二次生成结果显示 low，没有缓存或前次值残留 |
| 不同会话使用不同 effort | 每次输出只由各自输入决定，不读取其他会话状态 |
| TUI 禁用、移动或修改 Effort 样式后保存 | 当前配置持久化正确；预览与实际渲染采用相同顺序和样式 |
| Effort 位于首部、中间、末尾，或因无数据省略 | 普通分隔符、Powerline 箭头和背景颜色过渡正确 |
| 当前配置或磁盘主题不含 Effort | TUI 提供未启用的 Effort 选项，Enter 启用后更新预览，S 或 W 保存；不保存退出时文件内容不变 |
| 真实会话执行 `/effort` 或切换模型 | 下一次收到新状态栏载荷时显示与之对应的值；同时记录上游是否立即触发刷新 |
