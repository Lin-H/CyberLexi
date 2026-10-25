//! 主视图：侧边栏 + 顶栏搜索 + 三视图（词典/单词本/设置）+ 特效层。
//! 后台事件泵把 BackendEvent 送进 UI 状态；查询 300ms 防抖。

use std::time::Duration;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;

use lexi_core::domain::types::{QueryResult, WordSummary};
use lexi_core::domain::wordbook::WordBookItem;
use lexi_core::events::{BackendEvent, UiCommand};
use lexi_core::infra::config::Config;
use tokio::sync::mpsc::UnboundedSender;

use crate::theme::AccentPalette;
use crate::FocusSearch;

#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Dict,
    Book,
    Settings,
}

pub struct AppView {
    pub cmd_tx: UnboundedSender<UiCommand>,
    pub input: Entity<InputState>,
    pub page: Page,
    pub loading: bool,
    pub result: Option<QueryResult>,
    pub history: Vec<String>,
    pub history_open: bool,
    pub wordbook: Vec<WordBookItem>,
    pub wb_summaries: Vec<WordSummary>,
    pub offline: bool,
    pub quota: Option<(u32, u32)>,
    /// 导入进度 (pct, stage)；Some 时顶栏显示细条
    pub import_progress: Option<(f32, String)>,
    /// 一次性提示（导出/导入/错误）→ 右下角 toast
    pub notice: Option<String>,
    /// 侧边栏统计 (今日, 目标, 连续天数)
    pub stats: (u32, u32, u32),
    /// 离线词库行数
    pub dict_counts: (i64, i64),
    /// 本地 TTS 英语声音（启动枚举；MW 录音不足 3 个时补足发音按钮）
    pub tts_voices: Vec<String>,
    /// 词条后退导航栈（最近展示的词，栈顶=最近；返回时弹出重查）
    pub nav_history: Vec<String>,
    /// 返回触发的重查不再入栈
    nav_backing: bool,
    /// 当前展示的词（Entry 完成时更新；NotFound/候选列表不变）
    current_word: Option<String>,
    /// 已收到过 worker 的启动配置同步（其后 ConfigSaved 才算用户保存）
    boot_synced: bool,
    /// 配置镜像（worker 权威；ConfigSaved 回流刷新）
    pub cfg: Config,
    /// 强调色板（cfg.appearance.accent 派生）
    pub accent: AccentPalette,
    /// 单词本筛选
    pub book_filter: crate::book_view::BookFilter,
    pub add_word_open: bool,
    pub add_word_input: Entity<InputState>,
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
            InputState::new(window, cx)
                .placeholder("搜索单词、短语或例句…")
        });
        let add_word_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("输入单词后回车（例：ephemeral / 你好）")
        });
        let key_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Merriam-Webster API Key（留空 = 离线模式）")
        });
        let ttl_input = cx.new(|cx| InputState::new(window, cx).placeholder("30"));
        let max_input = cx.new(|cx| InputState::new(window, cx).placeholder("50000"));
        let import_path_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("数据包 zip 的完整路径…"));

        // 输入订阅：Change 防抖 300ms；Enter 立即查询
        let sub_input = cx.subscribe_in(&input, window, |this, state, event, _window, cx| {
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

        // 添加单词输入：Enter = 查询并加入单词本
        let sub_add = crate::book_view::subscribe_add_input(&add_word_input, window, cx);

        AppView {
            cmd_tx,
            input,
            page: Page::Dict,
            loading: false,
            result: None,
            history: Vec::new(),
            history_open: false,
            wordbook: Vec::new(),
            wb_summaries: Vec::new(),
            offline: true,
            quota: None,
            import_progress: None,
            notice: None,
            stats: (0, 20, 0),
            dict_counts: (0, 0),
            tts_voices: Vec::new(),
            nav_history: Vec::new(),
            nav_backing: false,
            current_word: None,
            boot_synced: false,
            cfg: Config::default(),
            accent: AccentPalette::Cyan,
            book_filter: crate::book_view::BookFilter::All,
            add_word_open: false,
            add_word_input,
            key_input,
            ttl_input,
            max_input,
            import_path_input,
            debounce_seq: 0,
            _subs: vec![sub_input, sub_add],
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

    /// 返回上一个词：弹出导航栈顶并重查（该次查询不入栈），
    /// 同时把搜索框文本同步为目标词。
    pub fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(word) = self.nav_history.pop() else {
            return;
        };
        self.nav_backing = true;
        let input = self.input.clone();
        input.update(cx, |s, cx| {
            s.set_value(word.clone(), window, cx);
        });
        self.dispatch_query(word, cx);
    }

    pub fn dispatch_detail(&mut self, word: String, cx: &mut Context<Self>) {
        self.loading = true;
        let _ = self.cmd_tx.send(UiCommand::QueryDetail(word));
        cx.notify();
    }

    fn apply_config(&mut self, cfg: Config) {
        self.accent = AccentPalette::parse(&cfg.appearance.accent)
            .unwrap_or(AccentPalette::Cyan);
        self.cfg = cfg.clone();
        crate::update_cfg_mirror(cfg);
    }

    fn on_backend_event(&mut self, ev: BackendEvent, cx: &mut Context<Self>) {
        match ev {
            BackendEvent::QueryStarted(input) => {
                self.loading = true;
                // 后退导航：返回触发的重查不入栈；普通查询把当前词
                // 压栈（与栈顶/新查询相同则跳过，上限 50）
                if self.nav_backing {
                    self.nav_backing = false;
                } else if let Some(cur) = self.current_word.clone() {
                    if cur != input && self.nav_history.last() != Some(&cur) {
                        self.nav_history.push(cur);
                        if self.nav_history.len() > 50 {
                            self.nav_history.remove(0);
                        }
                    }
                }
            }
            BackendEvent::QueryFinished { result, .. } => {
                self.loading = false;
                // 自动发音（设置开启 + 词条有音频码）
                if self.cfg.audio.auto_play {
                    if let QueryResult::Entry(entry) = &*result {
                        if let Some(code) = entry.audio_codes.first() {
                            let _ = self.cmd_tx.send(UiCommand::PlayAudio {
                                code: code.clone(),
                                ref_: "c".into(),
                            });
                        }
                    }
                }
                if let QueryResult::Entry(entry) = &*result {
                    self.current_word = Some(entry.headword_display.clone());
                }
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
                self.dict_counts = (ecdict_rows as i64, cedict_rows as i64);
                self.notice = Some(format!(
                    "词库导入完成：ECDICT {ecdict_rows} 条 / CEDICT {cedict_rows} 条"
                ));
            }
            BackendEvent::ImportFailed(e) => {
                self.import_progress = None;
                self.notice = Some(format!("词库导入失败：{e}"));
            }
            BackendEvent::DictStats { ecdict, cedict } => {
                self.dict_counts = (ecdict, cedict);
            }
            BackendEvent::TtsVoices(voices) => {
                self.tts_voices = voices;
            }
            BackendEvent::AudioReady { .. } => {}
            BackendEvent::AudioFailed { code, reason } => {
                self.notice = Some(format!("音频失败 [{code}]：{reason}"));
            }
            BackendEvent::ConfigSaved(cfg) => {
                self.apply_config(cfg);
                // 启动时的初始同步（boot 广播）不提示——"已保存"只在
                // 设置页真实保存动作后显示
                if self.boot_synced {
                    self.notice = Some("配置已保存".into());
                } else {
                    self.boot_synced = true;
                }
            }
            BackendEvent::WordBookChanged(items) => {
                self.wordbook = items;
                // 词表变化 → 请求刷新卡片摘要（离线聚合）
                let _ = self.cmd_tx.send(UiCommand::RequestWordBookDetails);
            }
            BackendEvent::WordBookDetails(summaries) => {
                self.wb_summaries = summaries;
            }
            BackendEvent::HistoryChanged(items) => self.history = items,
            BackendEvent::StatsUpdated {
                today_count,
                goal,
                streak_days,
            } => {
                self.stats = (today_count, goal, streak_days);
            }
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Ctrl+K → 聚焦搜索框
        let input = self.input.clone();

        // 特效层（可关：扫描线 / 网格）
        let grid = if self.cfg.appearance.grid {
            div().child(crate::effects::grid_layer())
        } else {
            div()
        };
        let scanlines = if self.cfg.appearance.scanlines {
            div().child(crate::effects::scanline_layer())
        } else {
            div()
        };

        // 主布局
        let sidebar = crate::sidebar::render(self, cx);
        let topbar = crate::topbar::render(self, window, cx);
        let body = match self.page {
            Page::Dict => {
                let content = crate::dict_view::render(self, cx);
                div()
                    .id("view-dict")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(28.))
                    .py(px(26.))
                    .pb(px(80.))
                    .child(content)
            }
            Page::Book => {
                let content = crate::book_view::render(self, cx);
                div()
                    .id("view-book")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(28.))
                    .py(px(26.))
                    .pb(px(80.))
                    .child(content)
            }
            Page::Settings => {
                let content = crate::settings_view::render(self, cx);
                div()
                    .id("view-settings")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(28.))
                    .py(px(26.))
                    .pb(px(80.))
                    .child(content)
            }
        };

        // 右下角 toast
        let toast = self.notice.clone().map(|text| {
            div()
                .absolute()
                .bottom(px(20.))
                .right(px(20.))
                .max_w(px(420.))
                .id("toast")
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px(10.))
                .px(px(14.))
                .py(px(10.))
                .rounded_lg()
                .bg(crate::theme::Neon::bg_surf())
                .border_1()
                .border_color(crate::theme::with_alpha(crate::theme::Neon::lime(), 0.5))
                .shadow(vec![crate::theme::glow(
                    crate::theme::Neon::lime(),
                    0.2,
                    24.,
                )])
                .on_click(cx.listener(|v, _, _, cx| {
                    v.notice = None;
                    cx.notify();
                }))
                .child(
                    div()
                        .text_sm()
                        .line_height(relative(1.4))
                        .text_color(crate::theme::Neon::text())
                        .child(text),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(crate::theme::Neon::muted())
                        .child("✕"),
                )
        });

        let base = div()
            .relative()
            .flex()
            .size_full()
            .bg(crate::theme::Neon::bg())
            .text_color(crate::theme::Neon::text())
            .font_family(crate::theme::UI_FONT)
            // Ctrl+K
            .on_action(cx.listener(move |_this, _: &FocusSearch, window, cx| {
                let handle = input.read(cx).focus_handle(cx);
                window.focus(&handle, cx);
                cx.notify();
            }))
            // 底层：氛围光 + 网格
            .child(crate::effects::glow_layer())
            .child(grid)
            // 内容层
            .child(
                div()
                    .relative()
                    .flex()
                    .size_full()
                    .child(sidebar)
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .flex_col()
                            .child(topbar)
                            .child(body),
                    ),
            )
            // 顶层：扫描线（盖在内容上，pointer-events none 天然成立）
            .child(scanlines);
        // 历史弹层在根层渲染（晚于内容绘制，不被右栏面板遮挡）
        let with_pop = if self.history_open {
            base.child(crate::topbar::history_popover(self, cx))
        } else {
            base
        };
        with_pop.children(toast)
    }
}
