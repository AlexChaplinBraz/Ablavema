//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
use crate::{installed::*, releases::*};

pub struct GuiArgs {
    pub releases: Releases,
    pub installed: Installed,
    pub updates: Option<Vec<Package>>,
    pub file_path: String,
}
