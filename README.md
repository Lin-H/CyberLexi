# CyberLexi

个人桌面端英汉词典（Phase 1 MVP）。Rust 2024 + GPUI（经 gpui-kit）+ rusqlite（FTS5）+ tokio 后台运行时。Windows 首发平台。

## 构建要求（Windows）

- Rust **stable / MSVC** 工具链（本仓库开发快照：rustc 1.99.1 (58cf8f9c 2026-10-13)，见 `rust-toolchain.toml`）
- Visual Studio 2022 Build Tools（"Desktop development with C++" 工作负载 + Windows SDK）
- CMake 在 PATH 上（gpui-kit 底层依赖需要）
- `rustup` 默认 host triple 须为 `x86_64-pc-windows-msvc`（`rustup set default-host x86_64-pc-windows-msvc`）

```powershell
cargo run -p lexi-ui            # 开发运行（dev 构建已实测：DirectX 11.1 + GPU 渲染）
cargo test -p lexi-core         # 核心逻辑独立测试（110 个），无需 GPU/窗口
cargo run -p lexi-core --bin db_probe   # 词库/查询链路探针（调试用）
```

## 运行行为（本机实测）

- 首次启动：自动导入 `assets/dict` → ECDICT **59,137 条** + CC-CEDICT **125,314 条**（约 1 分钟，顶部进度条；数据指纹记入 meta，二启跳过并打印 `offline dicts up to date`）
- 可写数据目录：便携优先（exe 旁 `data/`，dev 构建即 `target/debug/data/`），无权限回退 `%LOCALAPPDATA%\com.cyberlexi\`
- 查询链路（均已在单测覆盖 + db_probe 实测）：英文（缓存→MW→ECDICT 兜底）；中文（CEDICT 精确→拼音前缀→ECDICT 中文 FTS）；候选点击补全详情
- 词库再生成：`bash scripts/fetch_assets.sh`（完整数据源下载 + make_mini 筛选）

## 目录与数据

- `assets/` 只读资源（随包分发，永不写入）：词典数据、字体、主题
  - `assets/dict/ecdict.mini.csv` — ECDICT mini 子集（MIT，由完整版筛选生成，见 `crates/lexi-core/src/bin/make_mini.rs`）
  - `assets/dict/cedict.txt` — CC-CEDICT（CC BY-SA 4.0，MDBG 官方导出）
  - `assets/fonts/` — JetBrains Mono + Noto Sans SC（均 OFL）
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

登录、云同步、皮肤切换、多窗口、插件、Phase 2 学习功能（复习算法/学习计划/统计只有占位表与空 trait）。

## 许可

- 代码：MIT
- ECDICT 数据：MIT（skywind3000/ECDICT）
- CC-CEDICT 数据：CC BY-SA 4.0
- 字体：JetBrains Mono OFL / Noto Sans SC OFL
