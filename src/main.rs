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
use settings::TEXT_SIZE;
use std::sync::atomic::Ordering;

// TODO: Fix window cascading on Windows. This will involve creating our own window which we'll
// give to Iced.
// TODO: Remember user's window size.
// TODO: Add self-update for Windows.
// Might not be bad to have a separate version with it for Linux as well.
// Or if it doesn't add much to the file size, just make it toggleable through the CLI.
// TODO: Keep a changelog on the About tab.
// When updating the launcher itself, download the latest CHANGELOG.md and display it.
// There's no support for rendering markdown in Iced, but the plain text would do for now.
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
    run().await;
}

async fn run() {
    let gui_args = run_cli().await;

    if !ONLY_CLI.load(Ordering::Relaxed) {
        if LAUNCH_GUI.load(Ordering::Relaxed) || SETTINGS.read().unwrap().default_package.is_none()
        {
            // TODO: Add icon.
            let mut window = iced::window::Settings::default();
            window.size = (650, 570);
            window.min_size = Some((650, 570));

            let default_settings = iced::Settings::<()>::default();

            let settings = iced::Settings {
                flags: gui_args,
                window,
                default_font: default_settings.default_font,
                default_text_size: TEXT_SIZE,
                exit_on_close_request: default_settings.exit_on_close_request,
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
