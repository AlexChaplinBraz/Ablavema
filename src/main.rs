//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
mod cli;
mod gui;
mod helpers;
mod installed;
mod releases;
mod settings;
use crate::{cli::*, helpers::*, installed::*, settings::*};
use std::{error::Error, process::exit};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}.", e);
        exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let mut gui_args = run_cli().await?;

    if gui_args.launch_gui {
        // TODO: Move all this logic into the GUI.
        if SETTINGS.read().unwrap().check_updates_at_launch {
            if is_time_to_update() {
                gui_args.installed.update(&mut gui_args.releases).await?;
            } else {
                println!("Not yet time to check for updates.");
            }
        }

        if SETTINGS.read().unwrap().default_package.is_empty() {
            todo!("Launch GUI");
        } else {
            if gui_args.file_path.is_empty() {
                Installed::open_blender()?;
            } else {
                Installed::open_blender_with_file(&gui_args.file_path)?;
            }
        }
    }

    Ok(())
}
