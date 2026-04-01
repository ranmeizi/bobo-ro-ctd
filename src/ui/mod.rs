use crate::config::{AppConfig, load_config, save_config};
use crate::vision::task;
use eframe::egui;
use egui::ViewportBuilder;
use rodio::{Decoder, OutputStreamBuilder, Sink};
use scrap::{Capturer, Display};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn AllocConsole() -> i32;
    fn FreeConsole() -> i32;
}

#[cfg(target_os = "windows")]
fn set_debug_console(enabled: bool) {
    unsafe {
        if enabled {
            let _ = AllocConsole();
        } else {
            let _ = FreeConsole();
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn set_debug_console(_enabled: bool) {}

pub fn create_window() {
    // 尺寸
    let size = Some(egui::vec2(360.0, 360.0));

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder {
            inner_size: size,
            min_inner_size: size,
            max_inner_size: size,
            ..Default::default()
        },
        // 其他选项保持默认
        ..Default::default()
    };
    eframe::run_native(
        "倒数计时器",
        native_options,
        Box::new(|cc| {
            let app = MyEguiApp::new(cc);
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    );
}

struct MyEguiApp {
    /** 方格阈值（UI 显示） */
    block_threshold: usize,
    /** 倒数秒数 */
    counting_seconds: usize,
    /** 播放语音 */
    play_voice: bool,
    /** 方格最小宽度 */
    block_min_width: usize,
    /** 方格最大宽度 */
    block_max_width: usize,

    /**
     * 上面是配置参数
     * 下面是运行时状态
     */
    /** 运行状态 */
    running: bool,
    /** 停止标志 */
    stop_flag: Option<Arc<AtomicBool>>,
    /** 倒数开始时间 */
    countdown_start: Option<Instant>,
    /** 提醒秒数 */
    remind_seconds: Option<usize>,
    /** 上一次已播报的秒数（防止重复播放） */
    last_announced_second: Option<usize>,
    /** 共享给后台线程的方格阈值 */
    shared_block_threshold: Arc<AtomicUsize>,
    /** 共享给后台线程的最小方块宽度 */
    shared_block_min_width: Arc<AtomicUsize>,
    /** 共享给后台线程的最大方块宽度 */
    shared_block_max_width: Arc<AtomicUsize>,
    /** 是否需要开始倒计时（由后台线程置位） */
    should_start_countdown: Arc<AtomicBool>,
    /** 是否展示调试窗口 */
    debug_enabled: bool,
    /** 共享给后台线程的 debug 开关 */
    shared_debug_enabled: Arc<AtomicBool>,
    /** 后台线程输出的最新调试帧（二值图） */
    debug_frame_shared: Arc<Mutex<Option<task::DebugFrame>>>,
    /** UI 纹理缓存 */
    debug_texture: Option<egui::TextureHandle>,
    /** Debug 窗口尺寸（跟随二值图尺寸） */
    debug_view_size: Option<[f32; 2]>,
    /** 配置脏标记 */
    config_dirty: bool,
    /** 上次配置同步时间 */
    last_config_sync: Instant,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 为 egui 配置支持中文的字体
        let mut fonts = egui::FontDefinitions::default();

        if let Ok(bytes) = std::fs::read("assets/fonts/NotoSansSC-Regular.otf") {
            fonts.font_data.insert(
                "my_chinese_font".to_owned(),
                egui::FontData::from_owned(bytes).into(),
            );

            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.insert(0, "my_chinese_font".to_owned());
            }
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.insert(0, "my_chinese_font".to_owned());
            }

            cc.egui_ctx.set_fonts(fonts);
        } else {
            eprintln!(
                "Chinese font file not found: assets/fonts/NotoSansSC-Regular.otf; using default fonts"
            );
        }

        let cfg = load_config();
        let initial_threshold = cfg.block_threshold;
        Self {
            block_threshold: initial_threshold,
            block_min_width: cfg.block_min_width,
            block_max_width: cfg.block_max_width,
            shared_block_threshold: Arc::new(AtomicUsize::new(initial_threshold)),
            shared_block_min_width: Arc::new(AtomicUsize::new(cfg.block_min_width)),
            shared_block_max_width: Arc::new(AtomicUsize::new(cfg.block_max_width)),
            should_start_countdown: Arc::new(AtomicBool::new(false)),
            counting_seconds: cfg.counting_seconds,
            play_voice: cfg.play_voice,
            running: false,
            stop_flag: None,
            remind_seconds: None,
            last_announced_second: None,
            countdown_start: None,
            debug_enabled: false,
            shared_debug_enabled: Arc::new(AtomicBool::new(false)),
            debug_frame_shared: Arc::new(Mutex::new(None)),
            debug_texture: None,
            debug_view_size: None,
            config_dirty: false,
            last_config_sync: Instant::now(),
        }
    }

    /// 同步更新 UI 显示值和线程共享值
    fn set_block_threshold(&mut self, value: usize) {
        self.block_threshold = value;
        self.shared_block_threshold.store(value, Ordering::Relaxed);
        self.config_dirty = true;
    }

    fn current_config(&self) -> AppConfig {
        AppConfig {
            block_threshold: self.block_threshold,
            counting_seconds: self.counting_seconds,
            play_voice: self.play_voice,
            block_min_width: self.block_min_width,
            block_max_width: self.block_max_width,
        }
    }

    fn flush_config_if_needed(&mut self, force: bool) {
        if !self.config_dirty && !force {
            return;
        }
        if !force && self.last_config_sync.elapsed() < Duration::from_secs(2) {
            return;
        }
        if let Err(e) = save_config(&self.current_config()) {
            eprintln!("{e}");
            return;
        }
        self.config_dirty = false;
        self.last_config_sync = Instant::now();
    }

    fn start_worker(&mut self, _ctx: &egui::Context) {
        if self.running {
            return;
        }
        let flag = Arc::new(AtomicBool::new(false));
        let flag_worker = Arc::clone(&flag);
        // 线程中通过 shared_block_threshold 实时读取最新阈值
        let shared_block_threshold = Arc::clone(&self.shared_block_threshold);
        let shared_block_min_width = Arc::clone(&self.shared_block_min_width);
        let shared_block_max_width = Arc::clone(&self.shared_block_max_width);
        let shared_debug_enabled = Arc::clone(&self.shared_debug_enabled);
        let debug_frame_shared = Arc::clone(&self.debug_frame_shared);
        // 真正的后台线程
        let should_start_countdown = Arc::clone(&self.should_start_countdown);

        // 在线程内部创建并持有屏幕采集器，避免将非 Send 的类型跨线程移动
        thread::spawn(move || {
            let display = Display::primary().expect("Display::primary failed");
            let mut capturer = Capturer::new(display).expect("Capturer::new failed");

            while !flag_worker.load(Ordering::Relaxed) {
                // 每轮读取一次最新的共享阈值
                let block_threshold = shared_block_threshold.load(Ordering::Relaxed);
                let block_min_width = shared_block_min_width.load(Ordering::Relaxed);
                let block_max_width = shared_block_max_width.load(Ordering::Relaxed);
                let debug_enabled = shared_debug_enabled.load(Ordering::Relaxed);
                let result = task::loop_task(
                    &task::TaskCtx {
                        block_threshold,
                        block_min_width,
                        block_max_width,
                        debug_enabled,
                    },
                    &mut capturer,
                );

                if debug_enabled {
                    if let Some(frame) = result.debug_frame {
                        if let Ok(mut slot) = debug_frame_shared.lock() {
                            *slot = Some(frame);
                        }
                    }
                }

                if result.triggered {
                    // 通知 UI 线程开始倒计时
                    should_start_countdown.store(true, Ordering::Relaxed);
                }
                // 不要忙等
                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
            println!("worker exit");
        });
        self.stop_flag = Some(flag);
        self.running = true;

        // 测试代码
        // self.counting_down();
    }
    fn stop_worker(&mut self) {
        if let Some(flag) = &self.stop_flag {
            flag.store(true, Ordering::Relaxed);
        }
        self.running = false;
        self.stop_flag = None;
        self.debug_texture = None;
        self.debug_view_size = None;
        self.last_announced_second = None;
        // 如果你保存了 JoinHandle，这里再 join 一下会更干净

        self.reset();
    }

    fn reset(&mut self) {
        self.remind_seconds = None;
        self.countdown_start = None;
        self.last_announced_second = None;
    }

    // 倒数开始
    fn counting_down(&mut self) {
        self.remind_seconds = Some(self.counting_seconds);
        self.countdown_start = Some(Instant::now());
        self.last_announced_second = None;
    }

    fn maybe_play_countdown_voice(&mut self, sec: usize) {
        if !self.play_voice || sec == 0 || sec > 5 {
            return;
        }
        if self.last_announced_second == Some(sec) {
            return;
        }
        self.last_announced_second = Some(sec);
        play_countdown_voice(sec);
    }

    fn counting_down_tick(&mut self, ctx: &egui::Context) {
        // 如果后台线程请求开始倒计时，并且当前尚未开始，则在 UI 线程真正启动
        if self.should_start_countdown.swap(false, Ordering::Relaxed)
            && self.countdown_start.is_none()
        {
            println!("要开始倒数了: {}", self.counting_seconds);
            self.counting_down();
        }

        if let Some(start) = self.countdown_start {
            let now = Instant::now();
            let elapsed = now.duration_since(start);
            let elapsed_secs = elapsed.as_secs();

            self.remind_seconds = Some(self.counting_seconds - elapsed_secs as usize);
            if let Some(sec) = self.remind_seconds {
                self.maybe_play_countdown_voice(sec);
            }

            if self.counting_seconds - elapsed_secs as usize <= 0 {
                // 还原
                self.reset();
            }
        }

        // 100 ms 更新一下
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}

fn find_voice_file(sec: usize) -> Option<PathBuf> {
    let base = Path::new("assets/voice");
    let exts = ["mp3", "wav", "ogg", "m4a", "flac"];
    for ext in exts {
        let path = base.join(format!("{sec}.{ext}"));
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn play_countdown_voice(sec: usize) {
    let Some(path) = find_voice_file(sec) else {
        eprintln!("voice file not found for second {sec} in assets/voice/");
        return;
    };

    std::thread::spawn(move || {
        let Ok(stream) = OutputStreamBuilder::open_default_stream() else {
            eprintln!("failed to open default audio output");
            return;
        };
        let mixer = stream.mixer().clone();
        let Ok(file) = File::open(&path) else {
            eprintln!("failed to open voice file: {}", path.display());
            return;
        };
        let Ok(source) = Decoder::new(BufReader::new(file)) else {
            eprintln!("failed to decode voice file: {}", path.display());
            return;
        };
        let sink = Sink::connect_new(&mixer);
        sink.append(source);
        sink.sleep_until_end();
        drop(stream);
    });
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // tick 倒数
        self.counting_down_tick(ctx);

        // 1. 如果在倒计时阶段，就显示置顶透明倒计时窗口
        if let Some(sec) = self.remind_seconds {
            let (w, h) = (120.0_f32, 70.0_f32);

            // 使用 scrap 获取主显示器宽度，用于居中窗口
            let display = Display::primary().expect("Display::primary failed");
            let screen_width = display.width() as f32;

            let pos = egui::pos2(
                screen_width / 2.0 - w / 2.0,
                50.0, // 距离屏幕顶端 50 像素
            );

            let builder = egui::ViewportBuilder::default()
                .with_always_on_top() // 总在最上层
                .with_position(pos)
                .with_decorations(false) // 无标题栏/边框
                .with_transparent(true) // 背景透明
                .with_inner_size([w, h]);
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("countdown_overlay"),
                builder,
                |overlay_ctx, _class| {
                    egui::CentralPanel::default().show(overlay_ctx, |ui| {
                        ui.centered_and_justified(|ui| {
                            let text = format!("{sec} 秒");
                            let rich = egui::RichText::new(text)
                                .color(egui::Color32::RED)
                                .size(32.0);
                            ui.label(rich);
                        });
                    });
                    // 这里可以顺便做整秒播报逻辑（只改状态/发消息）
                },
            );
        }

        // 将后台线程的调试帧转成纹理
        if self.debug_enabled {
            if let Ok(mut slot) = self.debug_frame_shared.lock() {
                if let Some(frame) = slot.take() {
                    let image = egui::ColorImage::from_gray(
                        [frame.width, frame.height],
                        &frame.gray_pixels,
                    );
                    // 窗口内还有标题文字和默认 padding，这里预留额外空间，确保图像完整显示
                    const EXTRA_WIDTH: f32 = 24.0;
                    const EXTRA_HEIGHT: f32 = 72.0;
                    self.debug_view_size = Some([
                        frame.width as f32 + EXTRA_WIDTH,
                        frame.height as f32 + EXTRA_HEIGHT,
                    ]);
                    self.debug_texture = Some(ctx.load_texture(
                        "debug_mask_half",
                        image,
                        egui::TextureOptions::NEAREST,
                    ));
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("参数");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if self.debug_enabled {
                        "Debug: 开"
                    } else {
                        "Debug: 关"
                    };
                    if ui.button(label).clicked() {
                        self.debug_enabled = !self.debug_enabled;
                        self.shared_debug_enabled
                            .store(self.debug_enabled, Ordering::Relaxed);
                        set_debug_console(self.debug_enabled);
                    }
                });
            });

            let mut threshold_tmp = self.block_threshold;
            if ui
                .add(egui::Slider::new(&mut threshold_tmp, 10..=300).text("方格个数"))
                .changed()
            {
                self.set_block_threshold(threshold_tmp);
            }
            if ui
                .add(egui::Slider::new(&mut self.counting_seconds, 0..=60).text("倒数秒数"))
                .changed()
            {
                self.config_dirty = true;
            }

            if ui.checkbox(&mut self.play_voice, "播放声音").changed() {
                self.config_dirty = true;
            }

            let enabled = self.running;
            let prev_min = self.block_min_width;
            let prev_max = self.block_max_width;

            if ui
                .add(egui::Slider::new(&mut self.block_min_width, 1..=100).text("最小宽度"))
                .changed()
            {
                // 先记变化，统一在后面做区间约束和同步
            }
            if ui
                .add(egui::Slider::new(&mut self.block_max_width, 2..=200).text("最大宽度"))
                .changed()
            {
                // 先记变化，统一在后面做区间约束和同步
            }

            // 输入限制：min 不得超过 max，max 不得小于 min（保持至少 1px 间隔）
            if self.block_min_width >= self.block_max_width {
                self.block_max_width = self.block_min_width.saturating_add(1);
            }
            if self.block_max_width <= self.block_min_width {
                self.block_min_width = self.block_max_width.saturating_sub(1).max(1);
            }
            self.shared_block_min_width
                .store(self.block_min_width, Ordering::Relaxed);
            self.shared_block_max_width
                .store(self.block_max_width, Ordering::Relaxed);
            if self.block_min_width != prev_min || self.block_max_width != prev_max {
                self.config_dirty = true;
            }

            ui.separator();
            ui.label("方块尺寸预览");
            let preview_side = self.block_max_width.max(24) as f32;
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(preview_side + 20.0, preview_side + 20.0),
                egui::Sense::hover(),
            );
            let painter = ui.painter();
            let origin = rect.min + egui::vec2(10.0, 10.0);

            let max_side = self.block_max_width as f32;
            let max_rect = egui::Rect::from_min_size(origin, egui::vec2(max_side, max_side));
            painter.rect_stroke(
                max_rect,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 140, 0)),
                egui::StrokeKind::Outside,
            );

            let min_side = self.block_min_width as f32;
            let min_offset = (max_side - min_side) * 0.5;
            let min_rect = egui::Rect::from_min_size(
                origin + egui::vec2(min_offset, min_offset),
                egui::vec2(min_side, min_side),
            );
            painter.rect_stroke(
                min_rect,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 200, 120)),
                egui::StrokeKind::Outside,
            );
            ui.label(format!(
                "最小: {} px, 最大: {} px",
                self.block_min_width, self.block_max_width
            ));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!enabled, egui::Button::new("开始"))
                        .clicked()
                    {
                        // 只有 enabled == true 时，这里才会触发
                        self.start_worker(ctx);
                        std::println!("开始");
                    }

                    if ui.add_enabled(enabled, egui::Button::new("停止")).clicked() {
                        // 只有 enabled == true 时，这里才会触发
                        self.stop_worker();
                        std::println!("结束");
                    }
                });
            });
        });

        if self.debug_enabled {
            let [debug_w, debug_h] = self.debug_view_size.unwrap_or([640.0, 360.0]);
            let debug_builder = egui::ViewportBuilder::default()
                .with_title("Debug")
                .with_inner_size([debug_w, debug_h])
                .with_resizable(true);
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("debug_viewport"),
                debug_builder,
                |debug_ctx, _class| {
                    egui::CentralPanel::default().show(debug_ctx, |ui| {
                        ui.heading("Debug");
                        ui.label("二值图（1/2 缩放）");
                        if let Some(tex) = &self.debug_texture {
                            let size = tex.size_vec2();
                            ui.image((tex.id(), size));
                        } else {
                            ui.label("等待首帧...");
                        }
                    });
                },
            );
        }

        self.flush_config_if_needed(false);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.flush_config_if_needed(true);
    }
}
