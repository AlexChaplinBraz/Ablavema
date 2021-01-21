#![windows_subsystem = "windows"]
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
use helpers::check_connection;
use iced::Application;
use std::sync::atomic::Ordering;

// TODO: Fix window cascading on Windows. This will involve creating our own window which we'll
// give to Iced.
// TODO: Remember user's window size.

#[tokio::main]
async fn main() {
    #[cfg(target_os = "windows")]
    {
        use winapi::um::wincon;
        unsafe { wincon::AttachConsole(wincon::ATTACH_PARENT_PROCESS) };
    }

    check_connection().await;

    // TODO: Error handling.
    run().await;
}

async fn run() {
    let gui_args = run_cli().await;

    if !ONLY_CLI.load(Ordering::Relaxed) {
        if LAUNCH_GUI.load(Ordering::Relaxed) || SETTINGS.read().unwrap().default_package.is_none()
        {
            let mut window = iced::window::Settings::default();
            window.size = (900, 630);
            window.min_size = Some((900, 630));

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
