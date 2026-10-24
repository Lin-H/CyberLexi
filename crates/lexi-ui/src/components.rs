//! 词条 / 候选列表 / 未找到 / 单词本侧栏 / 设置页。

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::*;
use lexi_core::domain::types::{Candidate, MergedEntry, SourceTag};
use lexi_core::domain::wordbook::{WordBookItem, WordStatus};
use lexi_core::events::UiCommand;
use lexi_core::infra::config::Config;

use crate::app::AppView;
use crate::theme::{with_alpha, CyberpunkTheme};

/// 词条视图：头部（词/音标/词性/来源/发音）→ 中文 → 释义序列 → 标签/词源。
pub fn render_entry(
    entry: &MergedEntry,
    wordbook_keys: &[String],
    _cx: &mut Context<AppView>,
) -> Div {
    let mut col = div().flex().flex_col().gap_3().py_2();

    // ── 词条头 ──
    let mut head = div().flex().flex_wrap().items_baseline().gap_3();
    head = head.child(
        div()
            .text_3xl()
            .font_weight(FontWeight::BOLD)
            .text_color(CyberpunkTheme::text())
            .child(entry.headword_display.clone()),
    );
    if !entry.phonetic.is_empty() {
        head = head.child(
            div()
                .text_base()
                .text_color(CyberpunkTheme::cyan())
                .child(format!("/{}", entry.phonetic)),
        );
    }
    if !entry.pos.is_empty() {
        head = head.child(
            div()
                .text_sm()
                .italic()
                .text_color(CyberpunkTheme::dim())
                .child(entry.pos.clone()),
        );
    }
    // 来源标签（spec：词条头显示 MW / offline / ecdict）
    for tag in &entry.sources {
        let (label, color) = match tag {
            SourceTag::Mw => ("MW", CyberpunkTheme::cyan()),
            SourceTag::Ecdict => ("ecdict", CyberpunkTheme::green()),
            SourceTag::Cedict => ("cedict", CyberpunkTheme::green()),
        };
        head = head.child(
            div()
                .px_1()
                .rounded_sm()
                .border_1()
                .border_color(with_alpha(color, 0.5))
                .text_color(color)
                .text_xs()
                .child(label),
        );
    }
    // 发音按钮（首个音频码；其余已由 worker 预取缓存，离线可播）
    if let Some(code) = entry.audio_codes.first() {
        let code = code.clone();
        head = head.child(
            Button::new("play-audio")
                .outline()
                .label("▶ 发音")
                .on_click(move |_, _, _| {
                    // 按钮回调无 view 上下文：经全局命令通道转发
                    crate::send_global(UiCommand::PlayAudio { code: code.clone(), ref_: "c".into() });
                }),
        );
    }
    // 加入单词本（引用已保存内容，key 去重）
    {
        let key = entry.normalized_key.clone();
        let headword = entry.headword_display.clone();
        let already = wordbook_keys.iter().any(|k| *k == key);
        if !already {
            head = head.child(
                Button::new("add-wordbook")
                    .ghost()
                    .label("+ 单词本")
                    .on_click(move |_, _, _| {
                        crate::send_global(UiCommand::AddToWordBook {
                            key: key.clone(),
                            headword: headword.clone(),
                        });
                    }),
            );
        }
    }
    col = col.child(head);

    // ── 中文释义（永远来自 ECDICT）──
    if let Some(chinese) = &entry.chinese {
        col = col.child(
            div()
                .p_3()
                .rounded_lg()
                .bg(CyberpunkTheme::bg_surf())
                .border_1()
                .border_color(CyberpunkTheme::border())
                .child(div().text_xs().text_color(CyberpunkTheme::dim()).child("中文释义"))
                .child(
                    div()
                        .text_base()
                        .line_height(relative(1.6))
                        .child(chinese.clone()),
                ),
        );
    }

    // ── shortdef（英英速览）──
    if !entry.short_defs.is_empty() {
        let mut defs = div().flex().flex_col().gap_1();
        for (i, d) in entry.short_defs.iter().enumerate() {
            defs = defs.child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .text_color(CyberpunkTheme::cyan())
                            .text_sm()
                            .child(format!("{}.", i + 1)),
                    )
                    .child(div().text_sm().child(d.clone())),
            );
        }
        col = col.child(section("英英释义 shortdef", defs));
    }

    // ── sseq 释义序列（含例句）──
    if !entry.senses.is_empty() {
        let mut senses = div().flex().flex_col().gap_2();
        for s in &entry.senses {
            let mut sdiv = div().flex().flex_col().gap_1();
            let mut line = div().flex().gap_2().items_baseline();
            if let Some(sn) = &s.sn {
                line = line.child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_color(CyberpunkTheme::cyan())
                        .text_sm()
                        .child(sn.clone()),
                );
            }
            line = line.child(div().text_sm().child(s.text.clone()));
            sdiv = sdiv.child(line);
            for ex in &s.examples {
                sdiv = sdiv.child(
                    div()
                        .pl_6()
                        .text_sm()
                        .italic()
                        .text_color(CyberpunkTheme::dim())
                        .child(format!("— {ex}")),
                );
            }
            senses = senses.child(sdiv);
        }
        col = col.child(section("释义序列 senses", senses));
    }

    // ── 词频 / 考试标签 ──
    if entry.freq.is_some() || !entry.exam_tags.is_empty() {
        let mut chips = div().flex().flex_wrap().gap_1();
        if let Some((bnc, frq, collins, oxford)) = entry.freq {
            if bnc > 0 {
                chips = chips.child(freq_chip(format!("BNC #{bnc}")));
            }
            if frq > 0 {
                chips = chips.child(freq_chip(format!("COCA #{frq}")));
            }
            if collins > 0 {
                chips = chips.child(freq_chip(format!("Collins {collins}★")));
            }
            if oxford > 0 {
                chips = chips.child(freq_chip("牛津3000".into()));
            }
        }
        for tag in &entry.exam_tags {
            chips = chips.child(freq_chip(tag.to_uppercase()));
        }
        col = col.child(section("词频 / 考试", chips));
    }

    // ── 词源 / 首见年代 ──
    if entry.etymology.is_some() || entry.date.is_some() {
        let mut meta = div().flex().flex_col().gap_1();
        if let Some(et) = &entry.etymology {
            meta = meta.child(
                div()
                    .text_sm()
                    .text_color(CyberpunkTheme::dim())
                    .child(format!("词源：{et}")),
            );
        }
        if let Some(d) = &entry.date {
            meta = meta.child(
                div()
                    .text_sm()
                    .text_color(CyberpunkTheme::dim())
                    .child(format!("首见：{d}")),
            );
        }
        col = col.child(section("词源", meta));
    }

    col
}

fn section(title: &str, body: Div) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_lg()
        .bg(CyberpunkTheme::bg_surf())
        .border_1()
        .border_color(CyberpunkTheme::border())
        .child(
            div()
                .text_xs()
                .text_color(CyberpunkTheme::dim())
                .child(title.to_string()),
        )
        .child(body)
}

fn freq_chip(label: String) -> Div {
    div()
        .px_1()
        .py_1()
        .rounded_sm()
        .bg(with_alpha(CyberpunkTheme::yellow(), 0.08))
        .border_1()
        .border_color(with_alpha(CyberpunkTheme::yellow(), 0.5))
        .text_color(CyberpunkTheme::yellow())
        .text_xs()
        .child(label)
}

/// 中文/拼音候选列表（≤20 条；点击补全详情）。
pub fn render_candidates(cands: &[Candidate], cx: &mut Context<AppView>) -> Div {
    let mut col = div().flex().flex_col().gap_1().py_2();
    col = col.child(
        div()
            .text_sm()
            .text_color(CyberpunkTheme::dim())
            .child(format!("候选 {} 条（点击补全详情）", cands.len())),
    );
    for (i, c) in cands.iter().enumerate() {
        let word = c.word.clone();
        let mut row = div()
            .id(("cand", i))
            .cursor_pointer()
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .rounded_md()
            .bg(CyberpunkTheme::bg_surf())
            .border_1()
            .border_color(CyberpunkTheme::border())
            .hover(|d| {
                d.border_color(CyberpunkTheme::cyan()).shadow(vec![BoxShadow {
                    color: with_alpha(CyberpunkTheme::cyan(), 0.2),
                    offset: Point::new(px(0.), px(0.)),
                    blur_radius: px(8.),
                    spread_radius: px(0.),
                    inset: false,
                }])
            })
            .on_click(cx.listener(move |v, _, _, cx| {
                v.dispatch_detail(word.clone(), cx);
            }));
        row = row.child(
            div()
                .text_base()
                .text_color(CyberpunkTheme::text())
                .child(c.word.clone()),
        );
        if let Some(py) = &c.pinyin {
            row = row.child(
                div()
                    .text_sm()
                    .text_color(CyberpunkTheme::cyan())
                    .child(py.clone()),
            );
        }
        row = row.child(div().flex_1());
        row = row.child(
            div()
                .text_sm()
                .text_color(CyberpunkTheme::dim())
                .child(c.gloss.clone()),
        );
        col = col.child(row);
    }
    col
}

/// 未找到：显示 MW 拼写建议（点击即查）。
pub fn render_not_found(input: &str, suggestions: &[String], cx: &mut Context<AppView>) -> Div {
    let mut col = div().flex().flex_col().gap_2().py_2();
    col = col.child(
        div()
            .text_xl()
            .text_color(CyberpunkTheme::magenta())
            .child(format!("未找到「{input}」")),
    );
    if !suggestions.is_empty() {
        col = col.child(
            div()
                .text_sm()
                .text_color(CyberpunkTheme::dim())
                .child("你是想查："),
        );
        let mut chips = div().flex().flex_wrap().gap_1();
        for (si, s) in suggestions.iter().enumerate() {
            let text = s.clone();
            chips = chips.child(
                div()
                    .id(("sugg", si))
                    .cursor_pointer()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(CyberpunkTheme::border())
                    .hover(|d| d.border_color(CyberpunkTheme::cyan()))
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.dispatch_query(text.clone(), cx);
                    }))
                    .child(s.clone()),
            );
        }
        col = col.child(chips);
    }
    col
}

/// 初始占位。
pub fn render_placeholder() -> Div {
    div()
        .flex()
        .size_full()
        .items_center()
        .justify_center()
        .text_color(CyberpunkTheme::dim())
        .child("输入以查询 · Enter 立即搜索")
}

/// 单词本侧栏：列表 + 四态流转 + 删除。
/// 说明：中文候选固定 ≤20 条；单词本为手工积累（量级 ~10²），Phase 1
/// 直接渲染即可；Phase 2 若引入大列表再切换 gpui-kit VirtualList。
pub fn render_wordbook_panel(items: &[WordBookItem], _cx: &mut Context<AppView>) -> Stateful<Div> {
    let mut col = div()
        .id("wordbook-panel")
        .flex()
        .flex_col()
        .gap_1()
        .w(rems(16.0))
        .px_2()
        .py_2()
        .border_l_1()
        .border_color(CyberpunkTheme::border())
        .bg(CyberpunkTheme::bg_surf())
        .overflow_y_scroll();
    col = col.child(
        div()
            .text_sm()
            .font_weight(FontWeight::BOLD)
            .text_color(CyberpunkTheme::cyan())
            .child(format!("单词本 ({})", items.len())),
    );
    if items.is_empty() {
        col = col.child(
            div()
                .text_xs()
                .text_color(CyberpunkTheme::dim())
                .child("在词条页点击「+ 单词本」"),
        );
    }
    for (wi, item) in items.iter().enumerate() {
        let key = item.word_key.clone();
        let next_status = match item.status {
            WordStatus::New => WordStatus::Learning,
            WordStatus::Learning => WordStatus::Familiar,
            WordStatus::Familiar => WordStatus::Mastered,
            WordStatus::Mastered => WordStatus::New,
        };
        let status_label = item.status.as_str().to_string();
        let key2 = key.clone();

        let row = div()
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .rounded_md()
            .hover(move |d| d.bg(CyberpunkTheme::bg_elev()))
            .child(div().text_sm().child(item.headword.clone()))
            .child(div().flex_1())
            .child(
                div()
                    .id(("wb-status", wi))
                    .cursor_pointer()
                    .text_xs()
                    .px_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(status_color(item.status))
                    .text_color(status_color(item.status))
                    .hover(move |d| d.bg(with_alpha(status_color(next_status), 0.1)))
                    .on_click(move |_, _, _| {
                        crate::send_global(UiCommand::SetWordStatus {
                            key: key.clone(),
                            status: next_status,
                        });
                    })
                    .child(status_label),
            )
            .child(
                div()
                    .id(("wb-del", wi))
                    .cursor_pointer()
                    .text_xs()
                    .text_color(CyberpunkTheme::dim())
                    .hover(|d| d.text_color(CyberpunkTheme::magenta()))
                    .on_click(move |_, _, _| {
                        crate::send_global(UiCommand::RemoveFromWordBook(key2.clone()));
                    })
                    .child("✕"),
            );
        col = col.child(row);
    }
    col
}

fn status_color(status: WordStatus) -> Hsla {
    match status {
        WordStatus::New => CyberpunkTheme::dim(),
        WordStatus::Learning => CyberpunkTheme::cyan(),
        WordStatus::Familiar => CyberpunkTheme::yellow(),
        WordStatus::Mastered => CyberpunkTheme::green(),
    }
}

/// 设置页：MW Key（改完即用）/ TTL / 缓存上限 / 打开数据目录 / 导出导入。
pub fn render_settings(view: &AppView, _cx: &mut Context<AppView>) -> Div {
    let mut col = div().flex().flex_col().gap_4().px_4().py_4().max_w(rems(50.0));

    // 在线配置
    let key_input = view.key_input.clone();
    col = col.child(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .rounded_lg()
            .bg(CyberpunkTheme::bg_surf())
            .border_1()
            .border_color(CyberpunkTheme::border())
            .child(div().font_weight(FontWeight::BOLD).child("Merriam-Webster 在线词典"))
            .child(
                div()
                    .text_xs()
                    .text_color(CyberpunkTheme::dim())
                    .child(
                        "API Key 仅存本机 config.toml（已 gitignore）；留空 = 离线模式。\
                         非商用额度 1000 次/天。请勿外传 Key。",
                    ),
            )
            .child(Input::new(&key_input))
            .child(
                div().flex().gap_2().child(
                    Button::new("save-config")
                        .primary()
                        .label("保存（改完即用）")
                        .on_click({
                            let key_input = key_input.clone();
                            let ttl_input = view.ttl_input.clone();
                            let max_input = view.max_input.clone();
                            move |_, window, cx| {
                                let key = key_input.read(cx).value().to_string();
                                let ttl = ttl_input
                                    .read(cx)
                                    .value()
                                    .trim()
                                    .parse()
                                    .unwrap_or(30);
                                let max = max_input
                                    .read(cx)
                                    .value()
                                    .trim()
                                    .parse()
                                    .unwrap_or(50_000);
                                let _ = window;
                                crate::send_global(UiCommand::SaveConfig(Config {
                                    mw_api_key: key,
                                    cache_ttl_days: ttl,
                                    cache_max_rows: max,
                                }));
                            }
                        }),
                ),
            ),
    );

    // 缓存设置
    let ttl_input = view.ttl_input.clone();
    let max_input = view.max_input.clone();
    col = col.child(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .rounded_lg()
            .bg(CyberpunkTheme::bg_surf())
            .border_1()
            .border_color(CyberpunkTheme::border())
            .child(div().font_weight(FontWeight::BOLD).child("缓存"))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(div().text_sm().w(rems(8.0)).child("TTL（天）"))
                    .child(div().w(rems(12.0)).child(Input::new(&ttl_input))),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(div().text_sm().w(rems(8.0)).child("行数上限"))
                    .child(div().w(rems(12.0)).child(Input::new(&max_input))),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(CyberpunkTheme::dim())
                    .child("超限按 hit_count/fetched_at 冷淘汰；TTL 过期自动清理"),
            ),
    );

    // 数据目录
    let import_path_input = view.import_path_input.clone();
    col = col.child(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .rounded_lg()
            .bg(CyberpunkTheme::bg_surf())
            .border_1()
            .border_color(CyberpunkTheme::border())
            .child(div().font_weight(FontWeight::BOLD).child("数据"))
            .child(
                Button::new("open-data-dir")
                    .outline()
                    .label("打开数据目录")
                    .on_click(|_, _, _| {
                        crate::send_global(UiCommand::OpenDataDir);
                    }),
            )
            .child(
                Button::new("export-pkg")
                    .outline()
                    .label("导出数据包（db + 配置 + 音频）")
                    .on_click(|_, _, _| {
                        let dir = lexi_core::infra::paths::AppPaths::resolve()
                            .map(|p| p.export_dir())
                            .unwrap_or_default();
                        let _ = std::fs::create_dir_all(&dir);
                        let out = dir.join(format!("cyberlexi-{}.zip", day_stamp()));
                        crate::send_global(UiCommand::ExportPackage(out));
                    }),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(div().flex_1().child(Input::new(&import_path_input)))
                    .child(
                        Button::new("import-pkg")
                            .outline()
                            .label("导入数据包")
                            .on_click({
                                let import_path_input = import_path_input.clone();
                                move |_, _, cx| {
                                    let path = import_path_input.read(cx).value().trim().to_string();
                                    if !path.is_empty() {
                                        crate::send_global(UiCommand::ImportPackage(
                                            std::path::PathBuf::from(path),
                                        ));
                                    }
                                }
                            }),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(CyberpunkTheme::dim())
                    .child("导入完成后重启应用生效"),
            ),
    );

    col
}

/// 日期戳（不引入 chrono：以 SystemTime 天序号近似，仅用于文件名）。
fn day_stamp() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (secs / 86_400 + 19_000) as i64
}
