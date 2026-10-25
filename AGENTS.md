# AGENTS.md — 开发协作约定（CyberLexi）

个人桌面端英汉词典。Rust 2024 + GPUI（经 gpui-kit 0.8.0）+ rusqlite（FTS5）+ tokio 后台运行时。Windows 首发，macOS/Linux 构建由 CI 支持（未人工验收）。Phase 1（MVP）已完成；Phase 2（复习算法/学习计划）未启动。

## 架构铁律（任何改动不得违反）

1. **GPUI 主线程零阻塞**：所有 IO/网络/音频/PowerShell 调用在 worker（独立 tokio 运行时线程）执行；UI 与 worker 经 channel 通信（`UiCommand` 出、`BackendEvent` 入，UI 侧 50ms 批量泵）。主线程只做渲染与事件分发。
2. **层隔离**：`lexi-core`（domain/infra/orchestrator/worker）**零 GPUI 类型**；`lexi-ui` 不直接做 IO，只发命令、消费事件。
3. **MW API Key 绝不硬编码/编译进二进制/入库**：仅存数据目录 `config.toml`（已 gitignore）。未配置 Key = 离线模式，功能完整。
4. **缓存只刷新、永不清除**：`cache_ttl_days` 是刷新周期不是存活期；过期行不删除，仅超 `cache_max_rows` 冷淘汰。
5. 中文/词源/复习类决策以 `docs/` 下设计文档为准；不确定的实现细节在代码注释标「待实测/待确认」。

## 常用命令

```powershell
cargo run -p lexi-ui            # 开发运行（dev 构建实测可用）
cargo test -p lexi-core         # 核心逻辑测试（126 个），无需 GPU/窗口
cargo test --workspace          # 全量
cargo run -p lexi-core --bin db_probe   # 查询链路探针（调试）
bash scripts/fetch_assets.sh    # 离线词库资源再生成
```

## 代码结构地图

- `crates/lexi-core/src`
  - `domain/types.rs` — Sense/MwEntry/MergedEntry 等数据模型（MergedEntry 是 UI 渲染唯一来源）
  - `domain/merger.rs` — MW + ECDICT 字段级合并；`combine_entries` 同形词合并 + 短语词条分离（hw/meta.id 含空格 → phrases）
  - `infra/mw/` — client（配额 UTC 日计数 + 跨午夜 roll_day + 熔断）、parser（MW 文档 §2.1–2.29 全字段）、markup（tokens 转纯文本）、audio_url、**tts.rs（Windows SAPI 经 PowerShell，仅 Windows 有效，其他平台运行时报错但不崩）**
  - `infra/db/` — Db/FTS5、cache_repo（raw_json 全字段重放）、wordbook、history、meta（配额持久化）
  - `infra/audio.rs` — MW mp3 下载缓存 + rodio 播放（`Source::speed` 变速）
  - `orchestrator.rs` — 查询链编排：英文 缓存→MW→ECDICT；中文 CEDICT 精确→拼音前缀→ECDICT FTS
  - `worker.rs` — 命令循环 + 后台任务 spawn + 事件广播（boot 时 ConfigSaved/DictStats/TtsVoices 等）
  - `events.rs` — UiCommand / BackendEvent 协议（纯类型）
- `crates/lexi-ui/src`
  - `app.rs` — AppView 状态 + 事件泵 + 导航栈（nav_history ≤50，返回按钮）；toast（notice）；启动广播静默（boot_synced）
  - `dict_view.rs` — 词条渲染：hero（发音按钮 ≤3 = MW 变体 + TTS 补足）/sense 卡片（classify_sn 编号层级规则）/dros/phrases/syns/usages/quotes 区块/aside 面板
  - `topbar.rs` / `sidebar.rs` / `settings_view.rs` / `book_view.rs` / `theme.rs`（Neon 色板 + 三强调色板）/ `widgets.rs` / `effects.rs`（网格/扫描线/氛围光）
- `migrations/` — SQL 迁移（cache_entry 富字段为 ALTER TABLE 追加）
- `assets/` — 只读随包资源（dict/fonts）；运行时定位 exe 同级 `assets/`（dev 下仓库根）

## GPUI / gpui-kit 0.8 能力限制（踩坑实录，写 UI 前先读）

- **没有**：letter-spacing、文字渐变、Div rotate、CSS 动画、FluentBuilder `.when()`、`.uppercase()`、`.tracking_wide()`、`.text_4xl()`、自定义图标尺寸（用 `Sizable::with_size`）
- `rgba` 名称冲突：gpui 导出 `rgba(hex_u32)`（1 参）；主题助手用 `white(a)/steel(a)/with_alpha`
- `linear_gradient` 收 `LinearColorStop`（`linear_color_stop(color, pct)`），不收 Hsla —— 用 `theme::grad(angle, from, to)`
- `ElementId`：`From<(&str, usize)>` 有、`From<(&str, String)>` 无 —— 循环 child 用枚举下标
- canvas 绘制闭包 4 参（bounds, T, window, app）；`Pixels` 内部字段私有，用 `.as_f32()` 算术
- `overflow_y_scroll()` 前必须先 `.id()`（Stateful）
- `window.focus(&handle, cx)` 两参；div 默认 flex **row**（竖排必须显式 `.flex().flex_col()`，panel() 曾因此标题与内容并排）
- Ctrl+K 等键位：`actions!` + `KeyBinding::new`；`SendKeys` 的 `^k` 在应用未聚焦时不可达，自动化测试用鼠标点击 + 直接送键
- 无公开 `open_url` —— 外链（art/table）以文本形式展示
- **gpui-pre Windows `zoom()` 只最大化不还原**（无条件 SW_MAXIMIZE，上游 bug）→ 自定义标题栏暂无最大化按钮；需要最大化用拖动区双击（HTCAPTION 系统行为，再双击/下拖还原）。窗口控制：`appears_transparent: true` 隐藏原生标题栏 + 顶栏空白区 `.window_control_area(Drag)` + 自绘 min/close（topbar.rs `window_controls`）

## 事件与状态规则

- **日志策略**：文件日志仅 debug 构建启用（`worker::init_tracing` 与 `AppPaths::resolve` 的 logs 目录创建均按 `debug_assertions` 分流）；release 零日志零控制台（`windows_subsystem = "windows"`），tracing 调用退化为空操作
- `BackendEvent::ConfigSaved`：首个是 worker 启动广播（静默应用，不弹 toast）；之后每次都来自设置页真实保存（弹「配置已保存」）
- 配额：`MwClient` 内 UTC 日计数，`roll_day` 在每次 lookup 前跨午夜滚动；403=Key 无效、429=限流熔断（响应形态待实测）
- 导航栈：`QueryStarted` 且非返回触发时把当前展示词压栈（去重、≤50）；`go_back` 弹栈 + `InputState::set_value` 同步搜索框
- 缓存回放：`cache_entry.raw_json` 存 combine 前的全词条 JSON 数组，读取时 `parse_cached_entries` + `combine_entries` 还原——**新增 MW 字段无需列迁移**

## 验证方法（实测流程）

1. 逻辑：`cargo test --workspace`（126 项；新增解析字段必须带单测，参考 `parses_all_documented_fields`）
2. GUI 实测：构建 → 后台启动 → PowerShell（SetForegroundWindow + SetCursorPos/mouse_event 点击 + SendKeys 输入，Ctrl+K 聚焦搜索框）→ GetWindowRect 截图人工核对
3. 数据核对：`sqlite3 target/debug/data/db/lexi.sqlite` 查 cache_entry.raw_json（json_extract 数组元素）验证解析结果
4. 提交：中文 conventional 风格（`fix:`/`feat:`/`ci:`），提交信息写「问题/根因/修复/实测」四段；作者身份 `CyberLexi <dev@cyberlexi.local>`；确认 `.git-commit-msg.txt` 之类的临时文件不入库

## CI / 发布

- `.github/workflows/release.yml`：推 `v*` tag → Windows x64 / macOS arm64 / Linux x64 矩阵构建 → 二进制 + assets/ 打包 → 附加到 GitHub Release
- Linux 需 apt 系统依赖（fontconfig/freetype/xkbcommon/ALSA/cmake），清单在 workflow 内
- 仓库：`git@github.com:Lin-H/CyberLexi.git`，主分支 **master**

## 已知待实测项

- MW 服务端配额重置时刻与 UTC 日的偏差（当前按 UTC 日；若需对齐只需改 `utc_day`）
- MW 限流响应的真实 HTTP 状态码/报文（当前按 403/429 探测）
- Windows HiDPI 100%/150%/200% 渲染验收
- macOS/Linux 平台构建与运行（CI 清单按 Zed 常规依赖配置，未跑通验证）
