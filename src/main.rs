#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use opencv;

mod vision;
mod ui;

fn main() {
    println!("OpenCV 版本: {}", opencv::core::get_version_string().unwrap());

    ui::create_window();
}
