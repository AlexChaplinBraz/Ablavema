use self::{
    about::AboutState, packages::PackagesState, recent_files::RecentFilesState,
    self_updater::SelfUpdaterState, settings::SettingsState,
};
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

#[derive(Debug)]
pub struct TabState {
    pub recent_files: RecentFilesState,
    pub packages: PackagesState,
    pub settings: SettingsState,
    pub self_updater: SelfUpdaterState,
    pub about: AboutState,
}

impl Default for TabState {
    fn default() -> Self {
        Self {
            recent_files: Default::default(),
            packages: Default::default(),
            settings: Default::default(),
            self_updater: SelfUpdaterState::new(),
            about: Default::default(),
        }
    }
}
