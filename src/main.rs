#![windows_subsystem = "windows"]

use std::io;

use macroquad::{miniquad::window, prelude::*};

mod app;
mod events;
mod game;
mod ui_skins;
mod visual_board;

const WIDTH_TO_HEIGHT_RATIO: f32 = 1.8;
const WIDTH: u32 = 1000;

#[macroquad::main("chess_tail")]
async fn main() -> io::Result<()> {
    window::set_window_position(100, 100);
    window::set_window_size(WIDTH, (WIDTH as f32 / WIDTH_TO_HEIGHT_RATIO) as u32);

    let mut app = app::App::init().await;
    app.run().await;

    Ok(())
}
