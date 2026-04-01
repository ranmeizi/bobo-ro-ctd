use fs_extra::dir;
use scrap::{Capturer, Display};
use std::io::ErrorKind;
use std::thread;
use std::time::Duration;

use opencv::core::{self, AlgorithmHint, Vector};
use opencv::imgcodecs;
use opencv::imgproc;
use opencv::prelude::*;

/// 从文件路径读取图片，返回 OpenCV BGR Mat（imread 方式）
pub fn test_get_screenshot() -> opencv::Result<Mat> {
    imgcodecs::imread("assets/test_img/1806406962.png", imgcodecs::IMREAD_COLOR)
}

/// 创建一个主显示器的屏幕采集器（只在开始时调用一次）
pub fn create_capturer() -> std::io::Result<Capturer> {
    let display = Display::primary()?;
    Capturer::new(display)
}

/// 从已有的 Capturer 中获取一帧，并转成 OpenCV BGR Mat
/// 注意：不做计时和日志，只负责拿画面
pub fn capture_frame_bgr(capturer: &mut Capturer) -> opencv::Result<Mat> {
    let width = capturer.width() as i32;
    let height = capturer.height() as i32;

    // 等待直到拿到一帧
    let frame = loop {
        match capturer.frame() {
            Ok(frame) => break frame,
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => panic!("failed to capture frame: {}", e),
        }
    };

    let bytes = frame.to_vec(); // Vec<u8>, 长度 = width * height * 4

    let mat_1d = Mat::from_slice(&bytes)?; // 1 行，width * height * 4 列
    let mat_bgra = mat_1d.reshape(4, height)?; // height 行，通道数 4

    let mut mat_bgr = Mat::default();
    imgproc::cvt_color(
        &mat_bgra,
        &mut mat_bgr,
        imgproc::COLOR_BGRA2BGR,
        0,
        AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;

    Ok(mat_bgr)
}

/// 从已有的 Capturer 中获取一帧，并按目标高度缩放后转成 OpenCV BGR Mat
/// target_height <= 0 时不缩放，保持原始分辨率
pub fn capture_frame_bgr_scaled(
    capturer: &mut Capturer,
    target_height: i32,
) -> opencv::Result<Mat> {
    let width = capturer.width() as i32;
    let height = capturer.height() as i32;

    let frame = loop {
        match capturer.frame() {
            Ok(frame) => break frame,
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => panic!("failed to capture frame: {}", e),
        }
    };

    let bytes = frame.to_vec();
    let mat_1d = Mat::from_slice(&bytes)?;
    let mat_bgra = mat_1d.reshape(4, height)?;

    let mut mat_bgr = Mat::default();
    // 先缩放后转色，减少后续像素处理量。
    if target_height > 0 && height > target_height {
        let scale = target_height as f64 / height as f64;
        let target_width = (width as f64 * scale).round() as i32;
        let mut scaled_bgra = Mat::default();
        imgproc::resize(
            &mat_bgra,
            &mut scaled_bgra,
            core::Size::new(target_width, target_height),
            0.0,
            0.0,
            imgproc::INTER_AREA,
        )?;
        imgproc::cvt_color(
            &scaled_bgra,
            &mut mat_bgr,
            imgproc::COLOR_BGRA2BGR,
            0,
            AlgorithmHint::ALGO_HINT_DEFAULT,
        )?;
    } else {
        imgproc::cvt_color(
            &mat_bgra,
            &mut mat_bgr,
            imgproc::COLOR_BGRA2BGR,
            0,
            AlgorithmHint::ALGO_HINT_DEFAULT,
        )?;
    }

    Ok(mat_bgr)
}

fn normalized(filename: String) -> String {
    filename.replace(['|', '\\', ':', '/'], "")
}

/// 测试函数
#[test]
fn test_screen_capture() {
    let start = std::time::Instant::now();
    dir::create_all("target/monitors", true).unwrap();

    // 在测试中创建并使用一次 capturer，顺便统计耗时
    let mut capturer = create_capturer().expect("create_capturer failed");
    let mat = capture_frame_bgr(&mut capturer).unwrap();
    let params: Vector<i32> = Vector::new();
    imgcodecs::imwrite("target/monitors/monitor-primary.png", &mat, &params).unwrap();

    println!("运行耗时: {:?}", start.elapsed());
}

pub fn count_red_blocks(src: &Mat) -> opencv::Result<usize> {
    Ok(analyze_red_blocks(src, 5.0, 50.0, false)?.count)
}

pub struct DebugMask {
    pub width: usize,
    pub height: usize,
    pub gray_pixels: Vec<u8>,
}

pub struct RedBlockAnalysis {
    pub count: usize,
    pub debug_mask_half: Option<DebugMask>,
}

pub fn analyze_red_blocks(
    src: &Mat,
    block_min_width: f32,
    block_max_width: f32,
    debug: bool,
) -> opencv::Result<RedBlockAnalysis> {
    // 优化性能：按固定高度等比例缩放后再计算
    let src_size = src.size()?;
    let original_width = src_size.width;
    let original_height = src_size.height;

    // 目标高度，可按需要调整，例如 720 / 1080 等
    let target_height = 720;
    let scale = target_height as f64 / original_height as f64;
    let target_width = (original_width as f64 * scale).round() as i32;

    let mut small = Mat::default();
    let src = if original_height > target_height {
        imgproc::resize(
            src,
            &mut small,
            core::Size::new(target_width, target_height),
            0.0, // 按目标尺寸缩放，不再使用缩放因子
            0.0,
            imgproc::INTER_AREA,
        )?;
        &small
    } else {
        src
    };

    // 1. BGR -> HSV
    let mut hsv = Mat::default();
    imgproc::cvt_color(
        src,
        &mut hsv,
        imgproc::COLOR_BGR2HSV,
        0,
        AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;

    // 2. 提取红色掩码（两段 H：0-10 和 160-179）
    let mut mask1 = Mat::default();
    let mut mask2 = Mat::default();

    core::in_range(
        &hsv,
        &core::Scalar::new(0.0, 140.0, 90.0, 0.0),
        &core::Scalar::new(10.0, 255.0, 255.0, 0.0),
        &mut mask1,
    )?;
    core::in_range(
        &hsv,
        &core::Scalar::new(160.0, 140.0, 90.0, 0.0),
        &core::Scalar::new(179.0, 255.0, 255.0, 0.0),
        &mut mask2,
    )?;

    let mut mask = Mat::default();
    core::bitwise_or(&mask1, &mask2, &mut mask, &core::no_array());

    // 3. 仅 Debug 打开时，才额外生成展示用二值图（缩小 1/2）
    let debug_mask_half = if debug {
        let mask_size = mask.size()?;
        let half_w = (mask_size.width / 2).max(1);
        let half_h = (mask_size.height / 2).max(1);
        let mut half = Mat::default();
        imgproc::resize(
            &mask,
            &mut half,
            core::Size::new(half_w, half_h),
            0.0,
            0.0,
            imgproc::INTER_NEAREST,
        )?;
        Some(DebugMask {
            width: half_w as usize,
            height: half_h as usize,
            gray_pixels: half.data_bytes()?.to_vec(),
        })
    } else {
        None
    };

    // 4. 使用连通域标记，统计“近似正方形”的白色块个数
    let mut labels = Mat::default();
    let mut stats = Mat::default();
    let mut centroids = Mat::default();
    let n_labels = imgproc::connected_components_with_stats(
        &mask,
        &mut labels,
        &mut stats,
        &mut centroids,
        8,
        core::CV_32S,
    )?;

    let mut count = 0usize;
    for label in 1..n_labels {
        // 第 0 行是背景，从 1 开始是前景连通域
        let w = *stats.at_2d::<i32>(label, imgproc::CC_STAT_WIDTH)? as f32;
        let h = *stats.at_2d::<i32>(label, imgproc::CC_STAT_HEIGHT)? as f32;
        let area = *stats.at_2d::<i32>(label, imgproc::CC_STAT_AREA)? as f32;

        // 过滤掉太小的噪声
        if area < 36.0 {
            continue;
        }
        // 按配置过滤过小方块
        if w <= block_min_width || h <= block_min_width {
            continue;
        }

        // 按配置过滤过大方块
        if w >= block_max_width || h >= block_max_width {
            continue;
        }

        // 宽高比接近 1：近似正方形
        let ratio = w.max(h) / w.min(h);
        if ratio <= 1.3 {
            count += 1;
        }
    }

    println!("红色地块数量: {}", count);

    Ok(RedBlockAnalysis {
        count,
        debug_mask_half,
    })
}

#[test]
fn test_oneloop() {
    // 截图
    let image = test_get_screenshot().unwrap();
    // 红色地块数量

    let cnt = count_red_blocks(&image).unwrap();
    println!("红色地块数量: {}", cnt);
}
