# CyberLexi（NEONLEX）

个人桌面端英汉词典（Phase 1 MVP）。Rust 2024 + GPUI（经 gpui-kit）+ rusqlite（FTS5）+ tokio 后台运行时。Windows 首发平台。

界面为 NEONLEX 赛博朋克设计（设计稿：`docs/cyberpunk ui.html`）：侧边栏导航 + 顶栏搜索 + 词典/单词本/设置三视图、扫描线/网格/氛围光特效层、MW 全字段词条布局（sense 卡片 / 词源 / 词形 / 派生 / 右侧信息栏）。

## 构建要求（Windows）

- Rust **stable / MSVC** 工具链（本仓库开发快照：rustc 1.99.1 (58cf8f9c 2026-10-13)，见 `rust-toolchain.toml`）
- Visual Studio 2022 Build Tools（"Desktop development with C++" 工作负载 + Windows SDK）
- CMake 在 PATH 上（gpui-kit 底层依赖需要）
- `rustup` 默认 host triple 须为 `x86_64-pc-windows-msvc`（`rustup set default-host x86_64-pc-windows-msvc`）

```powershell
cargo run -p lexi-ui            # 开发运行（dev 构建已实测：DirectX 11.1 + GPU 渲染）
cargo test -p lexi-core         # 核心逻辑独立测试（115 个），无需 GPU/窗口
cargo run -p lexi-core --bin db_probe   # 词库/查询链路探针（调试用）
```

## 界面（对照设计稿）

- **侧边栏**：品牌区、词典/单词本/设置导航（选中态霓虹描边 + 左缘指示条）、底部统计（今日查词目标 + 连续打卡，来自 history 表 UTC 日聚合）
- **顶栏**：搜索框（焦点青光圈、`Ctrl K` 聚焦、300ms 防抖 + Enter 立即查）、MW 配额 chip、离线指示、随机单词（ECDICT 高频前 20k 随机）、历史弹层（最近 50 条点击即查）、首启导入进度细条
- **词典视图**：headword + homograph 上标 + 播放/收藏圆钮、MW/IPA 双音标、词性/考试/常用度标签、meta 信息条（SRC/SECTION/SORT/ID）、sense 卡片（强调色轮转、编号、例句、sdsense 子块 ↳）、交叉引用 chips、词源/屈折/派生/首见年代、右侧信息栏（shortdef / stems（可点击）/ 中文释义 / 词频档案 / 学习档案）
- **单词本视图**：四态筛选（全部/生词/学习中/熟悉/已掌握 + 计数）、卡片网格（左缘强调色、音标、释义、掌握度进度条）、添加单词行、卡片内状态流转/移除
- **设置视图**：外观（主题色板 cyan/magenta/violet 即改即存、扫描线开关、网格开关）、发音（自动发音、语速 0.75/1.0/1.25×——rodio `Source::speed`）、数据（MW Key、缓存 TTL/上限、词库计数与强制重导、数据目录/导出/导入/配额重置）
- 特效层：46px 等距网格（canvas 逐行绘制 + 顶部衰减）、CRT 扫描线（3px 周期）、三处氛围光斑（inset box-shadow 模糊近似径向渐变）；均可关闭

### 与设计稿的已知偏差（GPUI pre-1.0 能力限制）

- 无 letter-spacing / 文字渐变 / transform:rotate：品牌字距、headword 渐变文字与色偏投影、菱形 Logo（改为渐变圆角方块）以近似样式实现
- 无 CSS 动画：播放按钮涟波、sense 卡入场动画、hero sheen 流光省略
- `<select>` 改为「点击循环切换」选择器；`<kbd>` 键帽样式保留
- 单词本筛选取四态映射：设计稿「待复习」→「学习中」、「★收藏」→「熟悉」（SRS 收藏为 Phase 2）
- 口音偏好静态显示「美式」——MW 音频源仅提供美音；英音/TTS 待后续数据源
- 云端同步省略（Phase 1 无云端）；学习档案的「复习次数/下次复习」与「形近词」面板省略（复习算法为 Phase 2、形近词无数据源）

## 运行行为（本机实测）

- 首次启动：自动导入 `assets/dict` → ECDICT **59,137 条** + CC-CEDICT **125,314 条**（约 1 分钟，顶部进度条；数据指纹记入 meta，二启跳过并打印 `offline dicts up to date`）
- 可写数据目录：便携优先（exe 旁 `data/`，dev 构建即 `target/debug/data/`），无权限回退 `%LOCALAPPDATA%\com.cyberlexi\`
- 查询链路（均已在单测覆盖 + db_probe 实测）：英文（缓存→MW→ECDICT 兜底）；中文（CEDICT 精确→拼音前缀→ECDICT 中文 FTS）；候选点击补全详情
- 词库再生成：`bash scripts/fetch_assets.sh`（完整数据源下载 + make_mini 筛选）

## 目录与数据

- `assets/` 只读资源（随包分发，永不写入）：词典数据、字体、主题
  - `assets/dict/ecdict.mini.csv` — ECDICT mini 子集（MIT，由完整版筛选生成，见 `crates/lexi-core/src/bin/make_mini.rs`）
  - `assets/dict/cedict.txt` — CC-CEDICT（CC BY-SA 4.0，MDBG 官方导出）
  - `assets/fonts/` — JetBrains Mono + Noto Sans SC（均 OFL）；界面字体为系统 "Segoe UI"，图标为 gpui-kit-assets 内嵌 Lucide（编译期打入二进制，无需本地文件）
- 可写数据目录：优先 exe 旁 `data/`（便携模式），无权限时回退 `%LOCALAPPDATA%/com.cyberlexi/`
  - `db/lexi.sqlite`（缓存 + 单词本 + 离线词库 + FTS5）、`audio/`、`config.toml`（**gitignored**，MW API Key）、`logs/`、`export/`
- 迁移：设置页「导出/导入数据包」zip 整个数据目录（含音频缓存与配置）

## Merriam-Webster API

- Endpoint：`https://www.dictionaryapi.com/api/v3/references/collegiate/json/{word}?key=KEY`
- Key 仅存于数据目录 `config.toml`，**绝不硬编码/入库**；未配置时整条在线链跳过（离线模式功能完整）
- 非商用额度 ≤1000 次/天/Key：本地 meta 表计数，超限熔断到离线直至次日
- 音频 URL 规则实现于 `lexi-core/src/infra/mw/audio_url.rs`（含文档三例单测）

## 风险与应对

| 风险 | 应对 |
|---|---|
| GPUI pre-1.0 breaking change | 不直接依赖 gpui git rev；经 crates.io 的 `gpui-kit`（其内部以 `gpui-pre` 快照锁定 GPUI）。升级 = 升 gpui-kit 版本，README 记录所用 gpui-kit / gpui-pre 版本 |
| gpui-kit Windows 组件不足 | Input / VirtualList / Popover / Dialog / Switch 等均已在 gpui-kit 0.8 提供；若具体组件在 Windows 出问题，回退方案是自写简化组件（不改 core 层） |
| MW 1000 次/天限额 | 强缓存（TTL 30 天 + 命中不落网）+ 本地计数熔断 + 设置页提示勿外传 Key |
| ECDICT mini 覆盖率不足 | 数据源可替换：用户可放入更大规模 csv（同列名格式）重新导入；完整版生成见 make_mini 工具 |
| MW 限流响应形态未知 | 文档未给出限流 HTTP 状态码/报文；实现按 403/429 + body 探测并熔断，**待实测确认**（标记于 `mw/mod.rs`） |
| Windows HiDPI 渲染 | gpui-kit 声称支持；发布前须在 100%/150%/200% 缩放下人工验收 |

## Phase 1 明确不做

登录、云同步、多窗口、插件、Phase 2 学习功能（复习算法/学习计划只有占位表与空 trait；侧边栏统计是 history 表的轻量聚合，不涉及复习调度）。

## 许可

- 代码：MIT
- ECDICT 数据：MIT（skywind3000/ECDICT）
- CC-CEDICT 数据：CC BY-SA 4.0
- 字体：JetBrains Mono OFL / Noto Sans SC OFL
