use crate::vision::util;
use scrap::Capturer;

pub struct TaskCtx {
    pub block_threshold: usize,
    pub block_min_width: usize,
    pub block_max_width: usize,
    pub debug_enabled: bool,
}

#[derive(Clone)]
pub struct DebugFrame {
    pub width: usize,
    pub height: usize,
    pub gray_pixels: Vec<u8>,
}

pub struct LoopResult {
    pub triggered: bool,
    pub debug_frame: Option<DebugFrame>,
}

/// 后台循环任务：使用已有的屏幕采集器获取一帧并做检测
pub fn loop_task(ctx: &TaskCtx, capturer: &mut Capturer) -> LoopResult {
    println!("loop_task");
    // 截图：按固定高度降采样，减少高分辨率屏幕下的处理耗时
    let image = util::capture_frame_bgr_scaled(capturer, 720).unwrap();

    let analysis = util::analyze_red_blocks(
        &image,
        ctx.block_min_width as f32,
        ctx.block_max_width as f32,
        ctx.debug_enabled,
    )
    .unwrap();
    let triggered = analysis.count > ctx.block_threshold;

    let debug_frame = analysis.debug_mask_half.map(|mask| DebugFrame {
        width: mask.width,
        height: mask.height,
        gray_pixels: mask.gray_pixels,
    });

    LoopResult {
        triggered,
        debug_frame,
    }
}
