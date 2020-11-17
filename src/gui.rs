//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use crate::installed::*;
use crate::releases::*;

pub struct GuiArgs {
    pub releases: Releases,
    pub installed: Installed,
    pub file_path: String,
    pub launch_gui: bool,
}
