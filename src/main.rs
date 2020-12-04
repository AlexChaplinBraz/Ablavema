//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
mod cli;
mod gui;
mod helpers;
mod installed;
mod package;
mod releases;
mod settings;
mod style;
use crate::{cli::*, gui::*, helpers::*, settings::*};
use iced::Application;
use std::{error::Error, process::exit, sync::atomic::Ordering};

// TODO: Find a better way of not showing the console on Windows.
// This option practically disables the CLI. Can't have that.
// Also, this option doesn't solve the issue of the window spawning south-east of the centre,
// same placement with and without this option. For some reason it cascades anyway.
// So a custom solution will probably solve both these issues.
//#![windows_subsystem = "windows"]

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}.", e);
        // TODO: This may be flawed logic. If there's an error trying to run Blender
        // while bypassing the launcher it won't be shown to the user unless they
        // run it from the terminal. Probably better to always pop a msgbox, or
        // maybe not pop it only when CLI commands are ran.
        if LAUNCH_GUI.load(Ordering::Relaxed) {
            msgbox::create("BlenderLauncher", &e.to_string(), msgbox::IconType::Error).unwrap();
        }
        exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let (gui_args, only_cli) = run_cli().await?;

    if !only_cli {
        if LAUNCH_GUI.load(Ordering::Relaxed) || SETTINGS.read().unwrap().default_package.is_empty()
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

            Gui::run(settings)?;
        } else {
            match &gui_args.file_path {
                Some(file_path) => open_blender(
                    SETTINGS.read().unwrap().default_package.clone(),
                    Some(file_path.to_owned()),
                )?,
                None => open_blender(SETTINGS.read().unwrap().default_package.clone(), None)?,
            }
        }
    }

    Ok(())
}
