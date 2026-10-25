# CyberLexi Phase 1 完成总结

> 状态：**已完成**（2026-10-26）。126 项核心测试全绿、工作区零警告、全部功能经 GUI 实测验证。
> 仓库：<https://github.com/Lin-H/CyberLexi>（主分支 master，CI 见 `.github/workflows/release.yml`）

## 1. 一期目标

个人桌面端英汉词典 MVP：离线可用、在线增强（Merriam-Webster）、赛博朋克 UI、单词本与学习统计的轻量闭环。明确不做：登录/云同步/多窗口/插件/SRS 复习算法（Phase 2）。

## 2. 交付清单（按模块，均已实测）

### 2.1 查询链路（lexi-core）

| 链路 | 行为 | 兜底 |
|---|---|---|
| 英文词 | 缓存 → MW → ECDICT | ECDICT 释义包装为 sense 序列（行首 n./vt. 解析为 sense 级词性） |
| 中文词 | CEDICT 精确 → 拼音前缀 | 候选列表（≤20）点击后 QueryDetail 补全 |
| 未命中 | MW 拼写建议 | NotFound 界面 + 建议 chips |

- 归一化（Normalizer L1）+ 去重合并：同形词（remain:1 动词 / remain:2 名词）合并为多 sense 词条并保留 hom 上标；**短语词条**（remain to be seen 等）分离为 Phrases 区块
- 历史 300ms 防抖 + Enter 立即查；in-flight 去重

### 2.2 MW 数据全字段（文档 §2.1–2.29 核对）

- 解析：meta/hom/hwi(hw,prs,psl,altprs)/fl/lbs/sls（词级+块级+sense 级）/vrs/ahws/ins(顶层，il+if/ifc)/cxs/def(vd+fl+sseq)/sense(sn,dt,sdsense,sgram,ins,et)/dt 内嵌(audio,vis+aq,ri,bnw,uns,snote,ca)/uros(含 utxt,sls,lbs,prs)/dros/dxnls/usages/syns/quotes/art/table/et/date/shortdef
- 不展示（有意）：meta.uuid、ld_link
- markup token 转纯文本覆盖 §2.29 全部 token 类
- 专项测试 `parses_all_documented_fields` 全链路断言

### 2.3 UI（gpui-kit 0.8，对照 docs/cyberpunk ui.html）

- 布局：单词信息（hero）独占一行；下方双列 = 释义卡片 + 右侧信息栏；启动窗口居中于主显示器
- 侧边栏：导航（词典/单词本/设置）+ 今日查词目标/连续打卡（history UTC 日聚合）
- 顶栏：搜索框（青光圈焦点、Ctrl K）、**返回上一词**（导航栈 ≤50，覆盖所有跳转入口）、MW 配额 chip、OFFLINE、随机单词（ECDICT 高频 20k）、历史弹层（50 条点击即查）、导入进度条
- 词典视图：sense 卡片（sn 原生编号 "1 a"/裸字母/"(x)" 三级缩进、强调色轮转、词性徽标、[T/I]、sls 标签、USAGE/NOTE 块、called also、例句带出处）、Phrases / Run-On Phrases / Synonym Discussion / Usage Notes / Quotations 区块、交叉引用 chips、词源/屈折/派生/首见年代
- 右侧栏：Short Definitions / Stems（可点击）/ 中文释义（ECDICT 全量）/ 词频档案 / 学习档案 / 插图 / 参考表格
- 单词本：四态筛选+计数、卡片网格、状态流转、添加词
- 设置：主题强调色（cyan/magenta/violet 即改即存）、扫描线/网格开关、发音（自动播放、语速 0.75/1.0/1.25×）、MW Key/TTL/上限、强制重导、数据目录/导出/导入 zip、配额重置
- 特效层：网格 + CRT 扫描线 + 氛围光（canvas 逐像素绘制），均可关
- toast：仅真实保存/异常时出现（启动同步静默）

### 2.4 发音

- MW 录音变体（词头 prs + 义项内嵌 dt audio，去重）+ 本地 Windows SAPI TTS（David/Zira）补足至 ≤3 个按钮；青色序号 = 真人录音、紫色字母 = TTS
- rodio 播放 + `Source::speed` 变速；音频 sha256 缓存离线可播；设置语速对 MW 与 TTS 同时生效
- 派生词条行内播放自己的录音（不与词头混用）

### 2.5 缓存与数据

- MW 缓存**永久**：`cache_ttl_days`（默认 30 天）仅是刷新周期；命中重放 `raw_json`（combine 前全词条数组）——新增字段零迁移
- 配额：UTC 日计数 + 跨午夜自动滚动 + 超限熔断 + 手动重置；持久化跨重启
- 数据目录：便携优先（exe 旁 data/）→ %LOCALAPPDATA% 回退；导出/导入 zip 迁移
- 首启自动导入 ECDICT 59,137 + CEDICT 125,314 条（指纹跳过重复导入）

### 2.6 工程化

- 126 项 lexi-core 单测（解析/合并/缓存/配额/TTS/音频 URL/markup/分词/wordbook/history）
- CI：v* tag → Windows/macOS/Linux 三平台构建 → GitHub Release（二进制 + assets/）
- README / AGENTS.md / docs（本文）三层文档

## 3. 关键决策记录

| 决策 | 理由 |
|---|---|
| GPUI 经 gpui-kit 0.8（间接锁定 gpui-pre） | 隔离 pre-1.0 破坏性变更；升级点收敛为一个版本号 |
| raw_json 全字段重放而非列扩展 | MW 字段多且会继续补，避免每次加列迁移 |
| 短语词条在 combine_entries 分离 | MW 把短语词条混在单词查询结果里，是义项编号/缩进错乱的根因 |
| TTS 用 PowerShell System.Speech 而非引依赖 | 零新依赖、开发机有声可用；失败仅日志不崩 |
| 配额按 UTC 日 + roll_day | 服务端重置时刻未知（待实测），UTC 日是最简单可测近似；对齐只需改 utc_day |
| 返回按钮走「弹栈重查」而非缓存视图 | 保证与词条数据/收藏态一致；命中缓存即时返回 |

## 4. 实测验证记录（摘要）

- 查询链：db_probe + GUI 实测（voluminous/remain/abide/sound/你好/nihao 等）
- remain 专项：4 词条结构（2 同形 + 2 短语）→ 7 sense + 2 phrase 卡片、编号/缩进正确、词性各自标注
- 发音：remain 3 钮（1=录音、Z、D），TTS 出声；派生词录音不串词
- 配额：跨午夜滚动单测 + 熔断/重置/持久化测试
- UI 回归：面板布局（panel flex_col）、词条布局（hero 独占行）、返回导航（多词 + 短语跳转）、toast 静默
- 驱动方法（可复用）：PowerShell SetForegroundWindow + 鼠标点击 + SendKeys + GetWindowRect 截图（详见 AGENTS.md「验证方法」）

## 5. 已知限制 / 待实测

1. MW 配额服务端重置时刻与 UTC 的偏差（待实测；对齐点：`utc_day`）
2. MW 限流响应形态（按 403/429 探测，待实测确认）
3. Windows HiDPI 100/150/200% 人工验收未做
4. macOS/Linux：CI 构建清单未跑通验证；TTS 在非 Windows 平台不可用（运行时报错不崩）
5. 音标偏好静态「美式」（MW 仅美音）；英音待新数据源
6. ECDICT mini 覆盖率有限（可换全量 csv 重导）

## 6. Phase 2 待办（预留）

- SRS 复习算法与学习计划（wordbook 表已有 status 字段、侧边栏统计已聚合）
- 英音/多人声数据源、形近词面板、云端同步
- 视觉打磨：播放涟波、入场动画等设计稿动画项
