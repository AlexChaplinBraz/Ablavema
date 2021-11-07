use crate::releases::Releases;
use iced::{button, Executor};
use self_update::update::Release;

#[derive(Debug)]
pub struct GuiFlags {
    pub releases: Releases,
    pub file_path: Option<String>,
    pub self_releases: Option<Vec<Release>>,
}
#[derive(Debug, Default)]
pub struct GuiState {
    pub recent_files_button: button::State,
    pub packages_button: button::State,
    pub settings_button: button::State,
    pub self_updater_button: button::State,
    pub about_button: button::State,
}

pub struct GlobalTokio;

impl Executor for GlobalTokio {
    fn new() -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Self)
    }

    fn spawn(&self, future: impl std::future::Future<Output = ()> + Send + 'static) {
        tokio::task::spawn(future);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Choice {
    Enable,
    Disable,
}

impl Choice {
    pub const ALL: [Choice; 2] = [Choice::Enable, Choice::Disable];
}

#[derive(Clone, Debug)]
pub enum BuildTypeSettings {
    All,
    DailyLatest,
    DailyArchive,
    ExperimentalLatest,
    ExperimentalArchive,
    PatchLatest,
    PatchArchive,
    StableLatest,
    StableArchive,
    Lts,
}

#[derive(Clone, Debug)]
pub enum Location {
    Databases,
    Packages,
    Cache,
}
