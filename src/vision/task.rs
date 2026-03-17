use crate::vision::util;
use scrap::Capturer;

pub struct TaskCtx {
    pub block_threshold: usize,
}

/// 后台循环任务：使用已有的屏幕采集器获取一帧并做检测
pub fn loop_task(ctx: &TaskCtx, capturer: &mut Capturer) -> bool {
    println!("loop_task");
    // 截图：按固定高度降采样，减少高分辨率屏幕下的处理耗时
    let image = util::capture_frame_bgr_scaled(capturer, 720).unwrap();
    // 地震检测
    if util::count_red_blocks(&image).unwrap() > ctx.block_threshold {
        true
    } else {
        false
    }
}
