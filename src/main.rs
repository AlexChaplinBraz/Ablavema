#![warn(rust_2018_idioms)]
//#![allow(dead_code, unused_imports, unused_variables)]
mod cli;
mod gui;
mod helpers;
mod package;
mod releases;
mod settings;
use crate::{
    cli::run_cli,
    gui::Gui,
    helpers::open_blender,
    settings::{LAUNCH_GUI, ONLY_CLI, SETTINGS},
};
use iced::Application;
use std::sync::atomic::Ordering;

// TODO: Find a better way of not showing the console on Windows.
// This option practically disables the CLI. Can't have that.
// Also, this option doesn't solve the issue of the window spawning south-east of the centre,
// same placement with and without this option. For some reason it cascades anyway.
// So a custom solution will probably solve both these issues.
//#![windows_subsystem = "windows"]

#[tokio::main]
async fn main() {
    // TODO: Error handling.
    run().await;
}

async fn run() {
    let gui_args = run_cli().await;

    if !ONLY_CLI.load(Ordering::Relaxed) {
        if LAUNCH_GUI.load(Ordering::Relaxed) || SETTINGS.read().unwrap().default_package.is_none()
        {
            let mut window = iced::window::Settings::default();
            window.size = (1100, 600);
            window.min_size = Some((1100, 600));

            let default_settings = iced::Settings::<()>::default();

            let settings = iced::Settings {
                window,
                flags: gui_args,
                default_font: default_settings.default_font,
                default_text_size: default_settings.default_text_size,
                antialiasing: default_settings.antialiasing,
            };

            Gui::run(settings).unwrap();
        } else {
            match &gui_args.file_path {
                Some(file_path) => open_blender(
                    SETTINGS
                        .read()
                        .unwrap()
                        .default_package
                        .clone()
                        .unwrap()
                        .name,
                    Some(file_path.to_owned()),
                ),
                None => open_blender(
                    SETTINGS
                        .read()
                        .unwrap()
                        .default_package
                        .clone()
                        .unwrap()
                        .name,
                    None,
                ),
            }
        }
    }
}
