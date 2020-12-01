//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
mod cli;
mod gui;
mod helpers;
mod installed;
mod releases;
mod settings;
use crate::{cli::*, gui::*, helpers::*, installed::*, settings::*};
use std::{error::Error, process::exit, sync::atomic::Ordering};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}.", e);
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
            todo!("Launch GUI");
        } else {
            if gui_args.file_path.is_empty() {
                open_blender(SETTINGS.read().unwrap().default_package.clone(), None)?;
            } else {
                open_blender(
                    SETTINGS.read().unwrap().default_package.clone(),
                    Some(gui_args.file_path),
                )?;
            }
        }
    }

    Ok(())
}
