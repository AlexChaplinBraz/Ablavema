//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use crate::settings::*;
use clap::ArgMatches;
use std::{error::Error, path::Path};

pub fn process_str_arg(a: &ArgMatches, name: &str) -> Result<(), Box<dyn Error>> {
    if a.is_present(name) {
        let arg_str = a.value_of(name).unwrap();
        let old_str = SETTINGS.read().unwrap().get_str(name)?;

        if arg_str == old_str {
            println!("'{}' is unchanged from '{}'.", name, old_str);
        } else {
            SETTINGS.write().unwrap().set(name, arg_str)?;

            println!("'{}' changed from '{}' to '{}'.", name, old_str, arg_str);
        }
    }

    Ok(())
}

pub fn process_bool_arg(a: &ArgMatches, name: &str) -> Result<(), Box<dyn Error>> {
    if a.is_present(name) {
        let arg_bool = expand_bool(a.value_of(name).unwrap());
        let old_bool = SETTINGS.read().unwrap().get_bool(name)?;

        if arg_bool == old_bool {
            println!("'{}' is unchanged from '{}'.", name, old_bool);
        } else {
            SETTINGS.write().unwrap().set(name, arg_bool)?;

            println!("'{}' changed from '{}' to '{}'.", name, old_bool, arg_bool);
        }
    }

    Ok(())
}

fn expand_bool(boolean: &str) -> bool {
    match boolean {
        "t" => true,
        "f" => false,
        "true" => true,
        "false" => false,
        _ => unreachable!("Unexpected boolean value"),
    }
}

pub fn get_file_stem(filename: &str) -> &str {
    if filename.contains(".tar.") {
        let f = Path::new(filename).file_stem().unwrap().to_str().unwrap();
        Path::new(f).file_stem().unwrap().to_str().unwrap()
    } else {
        Path::new(filename).file_stem().unwrap().to_str().unwrap()
    }
}
