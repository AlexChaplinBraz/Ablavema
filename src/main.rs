//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
mod cli;
mod gui;
mod helpers;
mod installed;
mod releases;
mod settings;
use crate::{cli::*, installed::*, settings::*};
use std::{error::Error, fs::File, path::PathBuf, process::exit, time::SystemTime};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}.", e);
        exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    Settings::load()?;

    let mut gui_args = run_cli().await?;

    if gui_args.launch_gui {
        // TODO: Move all this logic into the GUI.
        if SETTINGS
            .read()
            .unwrap()
            .get_bool("check_updates_at_launch")?
        {
            let last_update_time = SETTINGS
                .read()
                .unwrap()
                .get::<PathBuf>("temp_dir")
                .unwrap()
                .join("last_update_time.bin");

            if last_update_time.exists() {
                let file = File::open(&last_update_time)?;
                let old_time: SystemTime = bincode::deserialize_from(file)?;

                if old_time
                    .elapsed()
                    .unwrap()
                    .as_secs()
                    .checked_div(60)
                    .unwrap()
                    >= SETTINGS
                        .read()
                        .unwrap()
                        .get::<u64>("minutes_between_updates")?
                {
                    gui_args.installed.update(&mut gui_args.releases).await?;

                    let now = SystemTime::now();
                    let file = File::create(&last_update_time)?;
                    bincode::serialize_into(file, &now)?;
                } else {
                    println!("Not yet time to check for updates.");
                }
            } else {
                gui_args.installed.update(&mut gui_args.releases).await?;

                let now = SystemTime::now();
                let file = File::create(&last_update_time)?;
                bincode::serialize_into(file, &now)?;
            }
        }

        if SETTINGS
            .read()
            .unwrap()
            .get_str("default_package")?
            .is_empty()
        {
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
