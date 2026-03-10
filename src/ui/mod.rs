use crate::vision::task;
use eframe::egui;
use egui::ViewportBuilder;
use scrap::{Capturer, Display};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

pub fn create_window() {
    // 尺寸
    let size = Some(egui::vec2(240.0, 140.0));

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
    /** 共享给后台线程的方格阈值 */
    shared_block_threshold: Arc<AtomicUsize>,
    /** 是否需要开始倒计时（由后台线程置位） */
    should_start_countdown: Arc<AtomicBool>,
    /** 倒数秒数 */
    counting_seconds: usize,
    /** 播放语音 */
    play_voice: bool,
    /** 运行状态 */
    running: bool,
    /** 停止标志 */
    stop_flag: Option<Arc<AtomicBool>>,
    /** 提醒秒数 */
    remind_seconds: Option<usize>,
    /** 倒数开始时间 */
    countdown_start: Option<Instant>,
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

        let initial_threshold = 60;
        Self {
            block_threshold: initial_threshold,
            shared_block_threshold: Arc::new(AtomicUsize::new(initial_threshold)),
            should_start_countdown: Arc::new(AtomicBool::new(false)),
            counting_seconds: 30,
            play_voice: false,
            running: false,
            stop_flag: None,
            remind_seconds: None,
            countdown_start: None,
        }
    }

    /// 同步更新 UI 显示值和线程共享值
    fn set_block_threshold(&mut self, value: usize) {
        self.block_threshold = value;
        self.shared_block_threshold.store(value, Ordering::Relaxed);
    }

    fn start_worker(&mut self, _ctx: &egui::Context) {
        if self.running {
            return;
        }
        let flag = Arc::new(AtomicBool::new(false));
        let flag_worker = Arc::clone(&flag);
        // 线程中通过 shared_block_threshold 实时读取最新阈值
        let shared_block_threshold = Arc::clone(&self.shared_block_threshold);
        // 真正的后台线程
        let should_start_countdown = Arc::clone(&self.should_start_countdown);

        // 在线程内部创建并持有屏幕采集器，避免将非 Send 的类型跨线程移动
        thread::spawn(move || {
            let display = Display::primary().expect("Display::primary failed");
            let mut capturer = Capturer::new(display).expect("Capturer::new failed");

            while !flag_worker.load(Ordering::Relaxed) {
                // 每轮读取一次最新的共享阈值
                let block_threshold = shared_block_threshold.load(Ordering::Relaxed);
                let result = task::loop_task(&task::TaskCtx { block_threshold }, &mut capturer);

                if result {
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
        // 如果你保存了 JoinHandle，这里再 join 一下会更干净

        self.reset();
    }

    fn reset(&mut self) {
        self.remind_seconds = None;
        self.countdown_start = None;
    }

    // 倒数开始
    fn counting_down(&mut self) {
        self.remind_seconds = Some(self.counting_seconds);
        self.countdown_start = Some(Instant::now());
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

            println!("elapsed_secs: {}", elapsed_secs);
            println!("counting_seconds: {}", self.counting_seconds);

            self.remind_seconds = Some(self.counting_seconds - elapsed_secs as usize);

            if self.counting_seconds - elapsed_secs as usize <= 0 {
                // 还原
                self.reset();
            }
        }

        // 100 ms 更新一下
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("参数");

            let mut threshold_tmp = self.block_threshold;
            if ui
                .add(egui::Slider::new(&mut threshold_tmp, 10..=300).text("方格个数"))
                .changed()
            {
                self.set_block_threshold(threshold_tmp);
            }

            ui.add(egui::Slider::new(&mut self.counting_seconds, 0..=60).text("倒数秒数"));

            ui.checkbox(&mut self.play_voice, "播放声音");

            let enabled = self.running;

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
    }
}
