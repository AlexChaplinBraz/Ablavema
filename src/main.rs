#![windows_subsystem = "windows"]
#![warn(rust_2018_idioms)]
//#![allow(dead_code, unused_imports, unused_variables)]
mod cli;
mod gui;
mod helpers;
mod package;
mod releases;
mod self_updater;
mod settings;
use crate::{
    cli::run_cli,
    gui::Gui,
    helpers::open_blender,
    settings::{get_setting, LAUNCH_GUI},
};
use helpers::check_connection;
use iced::pure::Application;
use settings::TEXT_SIZE;
use std::{env, sync::atomic::Ordering};

// TODO: Fix window cascading on Windows. This will involve creating our own window which we'll
// give to Iced.
// TODO: Remember user's window size.
// TODO: Add Windows metadata.
// TODO: Consider building custom window decorations.
// Something along the lines of how browsers have tabs next to the window buttons.

#[tokio::main]
async fn main() {
    #[cfg(target_os = "windows")]
    {
        // TODO: Investigate whether the console that's toggled by Blender
        // can still receive output.
        use winapi::um::wincon;
        unsafe { wincon::AttachConsole(wincon::ATTACH_PARENT_PROCESS) };
    }

    check_connection().await;

    // TODO: Error reporting on unrecoverable failure.
    // TODO: Implement error logging.
    // Saving the file and line of the error for easier debugging.
    run().await;
}

async fn run() {
    let gui_args = run_cli().await;

    if LAUNCH_GUI.load(Ordering::Relaxed) || get_setting().default_package.is_none() {
        let mut window = iced::window::Settings::default();
        window.size = (680, 620);
        window.min_size = Some((680, 620));
        window.icon = Some(
            iced::window::Icon::from_rgba(
                include_bytes!(env!("ICED_ICON_DATA_PATH")).to_vec(),
                env!("ICED_ICON_WIDTH").parse().unwrap(),
                env!("ICED_ICON_HEIGHT").parse().unwrap(),
            )
            .unwrap(),
        );

        let mut settings = iced::Settings::with_flags(gui_args);
        settings.id = Some(String::from("Ablavema"));
        settings.window = window;
        settings.default_font = Some(include_bytes!("../extra/fonts/Lambda.ttf"));
        settings.default_text_size = TEXT_SIZE;

        Gui::run(settings).unwrap();
    } else {
        match &gui_args.file_path {
            Some(file_path) => open_blender(
                get_setting().default_package.clone().unwrap().name,
                Some(file_path.to_owned()),
            ),
            None => open_blender(get_setting().default_package.clone().unwrap().name, None),
        }
    }
}
