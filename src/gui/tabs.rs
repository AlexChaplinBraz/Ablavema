use self::{
    about::AboutState, packages::PackagesState, self_updater::SelfUpdaterState,
    settings::SettingsState,
};
pub mod about;
pub mod packages;
pub mod self_updater;
pub mod settings;

#[derive(Debug)]
pub struct Tabs {
    // TODO: Save tab in user settings.
    // Will be useful when the recent files tab is introduced.
    pub tab: Tab,
    pub packages_state: PackagesState,
    pub settings_state: SettingsState,
    pub self_updater_state: SelfUpdaterState,
    pub about_state: AboutState,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            tab: Tab::Packages,
            packages_state: Default::default(),
            settings_state: Default::default(),
            self_updater_state: SelfUpdaterState::new(),
            about_state: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tab {
    Packages,
    Settings,
    SelfUpdater,
    About,
}
