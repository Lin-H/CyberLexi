//! 主视图：搜索框（300ms 防抖 + Enter）、词条区、单词本侧栏、设置页、
//! 离线横幅；后台事件泵把 BackendEvent 送进 UI 状态。

use std::time::Duration;

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::*;
use lexi_core::domain::types::QueryResult;
use lexi_core::domain::wordbook::WordBookItem;
use lexi_core::events::{BackendEvent, UiCommand};
use tokio::sync::mpsc::UnboundedSender;

use crate::components;
use crate::theme::{with_alpha, CyberpunkTheme};

#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Dictionary,
    Settings,
}

pub struct AppView {
    pub cmd_tx: UnboundedSender<UiCommand>,
    pub input: Entity<InputState>,
    pub page: Page,
    pub loading: bool,
    pub result: Option<QueryResult>,
    pub history: Vec<String>,
    pub wordbook: Vec<WordBookItem>,
    pub offline: bool,
    pub quota: Option<(u32, u32)>,
    /// 导入进度 (pct, stage)；Some 时顶部显示
    pub import_progress: Option<(f32, String)>,
    /// 一次性提示（导出/导入/错误）
    pub notice: Option<String>,
    // 设置页输入
    pub key_input: Entity<InputState>,
    pub ttl_input: Entity<InputState>,
    pub max_input: Entity<InputState>,
    pub import_path_input: Entity<InputState>,
    /// 防抖序号（旧 timer 到期但序号已变则丢弃）
    debounce_seq: u64,
    _subs: Vec<Subscription>,
}

impl AppView {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        cmd_tx: UnboundedSender<UiCommand>,
        event_rx: std::sync::mpsc::Receiver<BackendEvent>,
    ) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("查词：English 或 中文 / 拼音（Enter 查询）")
        });
        let key_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Merriam-Webster API Key（留空 = 离线模式）")
        });
        let ttl_input = cx.new(|cx| InputState::new(window, cx).placeholder("30"));
        let max_input = cx.new(|cx| InputState::new(window, cx).placeholder("50000"));
        let import_path_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("数据包 zip 的完整路径…"));

        // 输入订阅：Change 防抖 300ms；Enter 立即查询
        let sub = cx.subscribe_in(&input, window, |this, state, event, _window, cx| {
            let text = state.read(cx).value().to_string();
            match event {
                InputEvent::Change => this.on_input_changed(text, cx),
                InputEvent::PressEnter { .. } => this.dispatch_query(text, cx),
                _ => {}
            }
        });

        // 事件泵：worker → UI（50ms 批量拉取；GPUI 主线程零阻塞）
        cx.spawn_in(window, async move |this, cx| {
            loop {
                loop {
                    match event_rx.try_recv() {
                        Ok(ev) => {
                            let _ = this.update(cx, |v, cx| v.on_backend_event(ev, cx));
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
                        Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    }
                }
                smol::Timer::after(Duration::from_millis(50)).await;
            }
        })
        .detach();

        AppView {
            cmd_tx,
            input,
            page: Page::Dictionary,
            loading: false,
            result: None,
            history: Vec::new(),
            wordbook: Vec::new(),
            offline: true,
            quota: None,
            import_progress: None,
            notice: None,
            key_input,
            ttl_input,
            max_input,
            import_path_input,
            debounce_seq: 0,
            _subs: vec![sub],
        }
    }

    fn on_input_changed(&mut self, text: String, cx: &mut Context<Self>) {
        self.debounce_seq += 1;
        let seq = self.debounce_seq;
        cx.spawn(async move |this, cx| {
            smol::Timer::after(Duration::from_millis(300)).await;
            let _ = this.update(cx, |v, cx| {
                if v.debounce_seq == seq && !text.trim().is_empty() {
                    v.dispatch_query(text.clone(), cx);
                }
            });
        })
        .detach();
    }

    pub fn dispatch_query(&mut self, text: String, cx: &mut Context<Self>) {
        let text = text.trim().to_string();
        if text.is_empty() {
            return;
        }
        self.loading = true;
        let _ = self.cmd_tx.send(UiCommand::Query(text));
        cx.notify();
    }

    pub fn dispatch_detail(&mut self, word: String, cx: &mut Context<Self>) {
        self.loading = true;
        let _ = self.cmd_tx.send(UiCommand::QueryDetail(word));
        cx.notify();
    }

    fn on_backend_event(&mut self, ev: BackendEvent, cx: &mut Context<Self>) {
        match ev {
            BackendEvent::QueryStarted(_) => {
                self.loading = true;
            }
            BackendEvent::QueryFinished { result, .. } => {
                self.loading = false;
                self.result = Some(*result);
            }
            BackendEvent::OfflineMode(offline) => self.offline = offline,
            BackendEvent::Quota { used, limit } => self.quota = Some((used, limit)),
            BackendEvent::ImportProgress { pct, stage } => {
                self.import_progress = Some((pct, stage))
            }
            BackendEvent::ImportDone {
                ecdict_rows,
                cedict_rows,
            } => {
                self.import_progress = None;
                self.notice = Some(format!(
                    "词库导入完成：ECDICT {ecdict_rows} 条 / CEDICT {cedict_rows} 条"
                ));
            }
            BackendEvent::ImportFailed(e) => {
                self.import_progress = None;
                self.notice = Some(format!("词库导入失败：{e}"));
            }
            BackendEvent::AudioReady { .. } => {}
            BackendEvent::AudioFailed { code, reason } => {
                self.notice = Some(format!("音频失败 [{code}]：{reason}"));
            }
            BackendEvent::ConfigSaved(_) => {
                self.notice = Some("配置已保存".into());
            }
            BackendEvent::WordBookChanged(items) => self.wordbook = items,
            BackendEvent::HistoryChanged(items) => self.history = items,
            BackendEvent::PackageDone(res) => {
                self.notice = Some(match res {
                    Ok(msg) => msg,
                    Err(e) => format!("操作失败：{e}"),
                });
            }
            BackendEvent::Fatal(e) => {
                self.notice = Some(format!("后台错误：{e}"));
            }
        }
        cx.notify();
    }

}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let header = self.render_header(cx);
        let search = self.render_search(cx);
        let body = match self.page {
            Page::Dictionary => self.render_dictionary(cx),
            Page::Settings => components::render_settings(self, cx),
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(CyberpunkTheme::bg_base())
            .text_color(CyberpunkTheme::text())
            .font_family("JetBrains Mono")
            .child(header)
            .child(search)
            .child(div().flex_1().overflow_hidden().child(body))
    }
}

impl AppView {
    fn render_header(&mut self, cx: &mut Context<Self>) -> Div {
        let mut row = div().flex().items_center().gap_3().px_4().py_2();

        row = row
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(CyberpunkTheme::cyan())
                    .child("CyberLexi"),
            )
            .child(
                div()
                    .id("tab-dict")
                    .cursor_pointer()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(if self.page == Page::Dictionary {
                        CyberpunkTheme::cyan()
                    } else {
                        CyberpunkTheme::border()
                    })
                    .text_color(if self.page == Page::Dictionary {
                        CyberpunkTheme::cyan()
                    } else {
                        CyberpunkTheme::text()
                    })
                    .on_click(cx.listener(|v, _, _, cx| {
                        v.page = Page::Dictionary;
                        cx.notify();
                    }))
                    .child("词典"),
            )
            .child(
                div()
                    .id("tab-settings")
                    .cursor_pointer()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(if self.page == Page::Settings {
                        CyberpunkTheme::cyan()
                    } else {
                        CyberpunkTheme::border()
                    })
                    .text_color(if self.page == Page::Settings {
                        CyberpunkTheme::cyan()
                    } else {
                        CyberpunkTheme::text()
                    })
                    .on_click(cx.listener(|v, _, _, cx| {
                        v.page = Page::Settings;
                        cx.notify();
                    }))
                    .child("设置"),
            )
            .child(div().flex_1());

        // 配额（有 Key 时）
        if let Some((used, limit)) = self.quota {
            if limit > 0 {
                row = row.child(
                    div()
                        .text_xs()
                        .text_color(CyberpunkTheme::dim())
                        .child(format!("MW {used}/{limit}")),
                );
            }
        }

        // 离线模式横幅（常驻）
        if self.offline {
            row = row.child(
                div()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(CyberpunkTheme::yellow())
                    .text_color(CyberpunkTheme::yellow())
                    .text_xs()
                    .child("离线模式"),
            );
        }

        // 一次性提示
        if let Some(notice) = self.notice.clone() {
            row = row.child(
                div()
                    .id("notice-dismiss")
                    .cursor_pointer()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(with_alpha(CyberpunkTheme::green(), 0.08))
                    .border_1()
                    .border_color(CyberpunkTheme::green())
                    .text_color(CyberpunkTheme::green())
                    .text_xs()
                    .on_click(cx.listener(|v, _, _, cx| {
                        v.notice = None;
                        cx.notify();
                    }))
                    .child(notice),
            );
        }

        // 导入进度
        let mut col = div().flex().flex_col();
        if let Some((pct, stage)) = self.import_progress.clone() {
            col = col.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_1()
                    .text_xs()
                    .text_color(CyberpunkTheme::cyan())
                    .child(format!("首次导入词库… {stage}"))
                    .child(
                        div()
                            .h_1()
                            .w_64()
                            .rounded_full()
                            .bg(CyberpunkTheme::bg_elev())
                            .child(
                                div()
                                    .h_1()
                                    .w(rems(pct / 100.0 * 4.0))
                                    .rounded_full()
                                    .bg(CyberpunkTheme::cyan()),
                            ),
                    ),
            );
        }
        col.child(row)
            .bg(CyberpunkTheme::bg_surf())
            .border_b_1()
            .border_color(CyberpunkTheme::border())
    }

    fn render_search(&mut self, cx: &mut Context<Self>) -> Div {
        let mut col = div().flex().flex_col().gap_1().px_4().py_3();

        let input_value = self.input.read(cx).value().to_string();
        let mut search = div().flex().gap_2().items_center();
        // 搜索框：焦点 ring 由 gpui-kit 绘制（accent = cyan）
        search = search.child(div().flex_1().child(Input::new(&self.input)));

        if self.loading {
            search = search.child(
                div()
                    .text_color(CyberpunkTheme::cyan())
                    .text_xs()
                    .child("查询中…"),
            );
        }
        col = col.child(search);

        // 历史（输入为空时显示最近条目，点击即查；spec 下拉的简化形态）
        if input_value.is_empty() && !self.history.is_empty() {
            let mut chips = div().flex().flex_wrap().gap_1();
            for (i, h) in self.history.iter().take(10).enumerate() {
                let text = h.clone();
                chips = chips.child(
                    div()
                        .id(("hist", i))
                        .cursor_pointer()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(CyberpunkTheme::bg_elev())
                        .border_1()
                        .border_color(CyberpunkTheme::border())
                        .hover(|d| d.border_color(CyberpunkTheme::cyan()))
                        .text_xs()
                        .text_color(CyberpunkTheme::dim())
                        .on_click(cx.listener(move |v, _, _, cx| {
                            v.dispatch_query(text.clone(), cx);
                        }))
                        .child(h.clone()),
                );
            }
            col = col.child(
                div()
                    .text_xs()
                    .text_color(CyberpunkTheme::dim())
                    .child("最近查询")
                    .child(chips),
            );
        }
        col
    }

    fn render_dictionary(&mut self, cx: &mut Context<Self>) -> Div {
        // 先克隆数据避免 &self.result 与 &mut cx 借用冲突
        let result = self.result.clone();
        let wb_keys: Vec<String> = self.wordbook.iter().map(|i| i.word_key.clone()).collect();
        let wb_clone = self.wordbook.clone();

        let content = match &result {
            Some(QueryResult::Entry(entry)) => components::render_entry(entry, &wb_keys, cx),
            Some(QueryResult::Candidates(cands)) => components::render_candidates(cands, cx),
            Some(QueryResult::NotFound {
                input,
                suggestions,
            }) => components::render_not_found(input, suggestions, cx),
            None => components::render_placeholder(),
        };

        div()
            .flex()
            .size_full()
            .overflow_hidden()
            .child(
                div()
                    .id("entry-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .px_4()
                    .pb_4()
                    .child(content),
            )
            .child(components::render_wordbook_panel(&wb_clone, cx))
    }
}
