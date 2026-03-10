use crate::vision::util;
use opencv::prelude::*;
use scrap::Capturer;

pub struct TaskCtx {
    pub block_threshold: usize,
}

/// 后台循环任务：使用已有的屏幕采集器获取一帧并做检测
pub fn loop_task(ctx: &TaskCtx, capturer: &mut Capturer) -> bool {
    println!("loop_task");
    // 截图（只负责获取一帧画面）
    let image = util::capture_frame_bgr(capturer).unwrap();
    // 地震检测
    if util::count_red_blocks(&image).unwrap() > ctx.block_threshold {
        true
    } else {
        false
    }
}
