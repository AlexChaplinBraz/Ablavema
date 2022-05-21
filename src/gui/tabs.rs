use serde::{Deserialize, Serialize};
pub mod about;
pub mod packages;
pub mod recent_files;
pub mod self_updater;
pub mod settings;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Tab {
    RecentFiles,
    Packages,
    Settings,
    SelfUpdater,
    About,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Packages
    }
}
