use crate::releases::Releases;
use clap::crate_version;
use iced::Executor;
use self_update::update::Release;

#[derive(Debug)]
pub struct GuiFlags {
    pub releases: Releases,
    pub file_path: Option<String>,
    pub self_releases: Option<Vec<Release>>,
}
#[derive(Debug, Default)]
pub struct GuiState {
    pub release_versions: Vec<String>,
    pub fetching_releases: bool,
    pub pick_list_selected_releases: String,
    pub installing_release: bool,
    pub installed_release: bool,
}

impl GuiState {
    pub fn new() -> Self {
        Self {
            pick_list_selected_releases: crate_version!().to_string(),
            ..Default::default()
        }
    }
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
