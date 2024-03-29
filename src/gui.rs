mod controls;
pub mod extra;
pub mod filters;
mod install;
mod message;
mod package;
pub mod sort_by;
pub mod style;
pub mod tabs;
use self::{
    controls::Controls,
    extra::{DiskSpace, GlobalTokio, GuiFlags, GuiState},
    install::Install,
    message::GuiMessage,
    tabs::recent_files::RecentFile,
};
use crate::{
    gui::tabs::Tab,
    helpers::check_connection,
    package::Package,
    releases::{
        daily_archive::DailyArchive, daily_latest::DailyLatest,
        experimental_archive::ExperimentalArchive, experimental_latest::ExperimentalLatest,
        lts::Lts, patch_archive::PatchArchive, patch_latest::PatchLatest,
        stable_archive::StableArchive, stable_latest::StableLatest, ReleaseType, Releases,
    },
    self_updater::SelfUpdater,
    settings::{get_setting, save_settings, set_setting, CAN_CONNECT},
};
use fs2::available_space;
use fs_extra::dir;
use iced::{
    alignment::Horizontal,
    pure::{
        widget::{Button, Column, Container, Row, Text},
        Application, Element,
    },
    Command, Length, Space, Subscription,
};
use self_update::update::Release;
use std::sync::atomic::Ordering;
use tokio::task::spawn_blocking;

macro_rules! build_fetching {
    ($name:ident, $release:ident) => {
        async fn $name(packages: $release) -> (bool, $release) {
            check_connection().await;

            if CAN_CONNECT.load(Ordering::Relaxed) {
                $release::check_updates(packages).await
            } else {
                (false, packages)
            }
        }
    };
}

#[derive(Debug)]
pub struct Gui {
    releases: Releases,
    packages: Vec<Package>,
    installing: Vec<Package>,
    file_path: Option<String>,
    recent_files: Vec<RecentFile>,
    state: GuiState,
    controls: Controls,
    self_releases: Option<Vec<Release>>,
}

impl Gui {
    pub fn sync(&mut self) {
        self.releases.sync();
        self.packages = self.releases.build_vec();
    }

    /// A tuple is returned where:
    /// (true_if_available, true_if_for_install, package)
    async fn check_availability(
        for_install: bool,
        package: Package,
    ) -> Option<(bool, bool, Package)> {
        match reqwest::get(&package.url).await {
            Ok(response) => {
                if response.status().is_client_error() {
                    Some((false, for_install, package))
                } else {
                    Some((true, for_install, package))
                }
            }
            Err(_) => {
                CAN_CONNECT.store(false, Ordering::Relaxed);
                None
            }
        }
    }

    async fn pass_package(package: Package) -> Package {
        package
    }

    async fn pass_string(string: String) -> String {
        string
    }

    async fn check_for_updates(
        packages: (
            DailyLatest,
            ExperimentalLatest,
            PatchLatest,
            StableLatest,
            Lts,
        ),
    ) -> (
        bool,
        DailyLatest,
        ExperimentalLatest,
        PatchLatest,
        StableLatest,
        Lts,
    ) {
        check_connection().await;

        if CAN_CONNECT.load(Ordering::Relaxed) {
            Releases::check_updates(packages).await
        } else {
            (
                false, packages.0, packages.1, packages.2, packages.3, packages.4,
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn check_all(
        daily_latest: DailyLatest,
        daily_archive: DailyArchive,
        experimental_latest: ExperimentalLatest,
        experimental_archive: ExperimentalArchive,
        patch_latest: PatchLatest,
        patch_archive: PatchArchive,
        stable_latest: StableLatest,
        stable_archive: StableArchive,
        lts: Lts,
    ) -> (
        bool,
        DailyLatest,
        DailyArchive,
        ExperimentalLatest,
        ExperimentalArchive,
        PatchLatest,
        PatchArchive,
        StableLatest,
        StableArchive,
        Lts,
    ) {
        check_connection().await;

        if CAN_CONNECT.load(Ordering::Relaxed) {
            let daily_latest = DailyLatest::check_updates(daily_latest).await.1;
            let daily_archive = DailyArchive::check_updates(daily_archive).await.1;
            let experimental_latest = ExperimentalLatest::check_updates(experimental_latest)
                .await
                .1;
            let experimental_archive = ExperimentalArchive::check_updates(experimental_archive)
                .await
                .1;
            let patch_latest = PatchLatest::check_updates(patch_latest).await.1;
            let patch_archive = PatchArchive::check_updates(patch_archive).await.1;
            let stable_latest = StableLatest::check_updates(stable_latest).await.1;
            let stable_archive = StableArchive::check_updates(stable_archive).await.1;
            let lts = Lts::check_updates(lts).await.1;

            (
                true,
                daily_latest,
                daily_archive,
                experimental_latest,
                experimental_archive,
                patch_latest,
                patch_archive,
                stable_latest,
                stable_archive,
                lts,
            )
        } else {
            (
                false,
                daily_latest,
                daily_archive,
                experimental_latest,
                experimental_archive,
                patch_latest,
                patch_archive,
                stable_latest,
                stable_archive,
                lts,
            )
        }
    }

    build_fetching!(check_daily_latest, DailyLatest);

    build_fetching!(check_daily_archive, DailyArchive);

    build_fetching!(check_experimental_latest, ExperimentalLatest);

    build_fetching!(check_experimental_archive, ExperimentalArchive);

    build_fetching!(check_patch_latest, PatchLatest);

    build_fetching!(check_patch_archive, PatchArchive);

    build_fetching!(check_stable_latest, StableLatest);

    build_fetching!(check_stable_archive, StableArchive);

    build_fetching!(check_lts, Lts);

    async fn check_connection() {
        check_connection().await;
    }

    async fn fetch_self_releases() -> Option<Vec<Release>> {
        spawn_blocking(SelfUpdater::fetch).await.unwrap()
    }

    async fn change_self_version(releases: Vec<Release>, version: String) {
        spawn_blocking(|| SelfUpdater::change(releases, version))
            .await
            .unwrap();
    }

    async fn calculate_disk_space() -> DiskSpace {
        DiskSpace {
            cache_size: dir::get_size(&get_setting().cache_dir).unwrap() as f32
                / 1024.0
                / 1024.0
                / 1024.0,
            cache_available: available_space(&get_setting().cache_dir).unwrap() as f32
                / 1024.0
                / 1024.0
                / 1024.0,
            packages_size: dir::get_size(&get_setting().packages_dir).unwrap() as f32
                / 1024.0
                / 1024.0
                / 1024.0,
            packages_available: available_space(&get_setting().packages_dir).unwrap() as f32
                / 1024.0
                / 1024.0
                / 1024.0,
        }
    }
}

impl Application for Gui {
    type Executor = GlobalTokio;
    type Message = GuiMessage;
    type Flags = GuiFlags;

    fn new(flags: Self::Flags) -> (Self, Command<GuiMessage>) {
        let releases = flags.releases;

        let packages = releases.build_vec();

        let default_package = get_setting().default_package.clone();
        if let Some(package) = default_package {
            if !releases.installed.contains(&package) {
                set_setting().default_package = None;
                save_settings();
            }
        }

        let mut state = GuiState::new();

        let self_releases = flags.self_releases;

        if let Some(s_releases) = &self_releases {
            state.release_versions = s_releases
                .iter()
                .map(|release| release.version.clone())
                .collect();
        }

        (
            Gui {
                releases,
                packages,
                file_path: flags.file_path,
                recent_files: get_setting().recent_files.to_vec(),
                installing: Vec::default(),
                state,
                controls: Controls::default(),
                self_releases,
            },
            Command::perform(Gui::calculate_disk_space(), GuiMessage::CalculateDiskSpace),
        )
    }

    fn title(&self) -> String {
        match self.releases.count_updates().all {
            Some(count) => format!(
                "Ablavema - {} update{} available!",
                count,
                if count > 1 { "s" } else { "" }
            ),
            None => String::from("Ablavema"),
        }
    }

    fn update(&mut self, message: GuiMessage) -> Command<GuiMessage> {
        self.update_message(message)
    }

    fn subscription(&self) -> Subscription<GuiMessage> {
        Subscription::batch(
            self.installing
                .iter()
                .map(|package| Install::package(package.to_owned())),
        )
    }

    fn view(&self) -> Element<'_, GuiMessage> {
        let file_exists = self.file_path.is_some();
        let current_tab = get_setting().tab;
        let update_count = self.releases.count_updates();

        let tab_button = |label, tab| {
            let button = Button::new(Text::new(label).horizontal_alignment(Horizontal::Center))
                .width(Length::Units(100))
                .style(get_setting().theme.tab_button());

            if tab == current_tab {
                Container::new(button).padding(2)
            } else {
                Container::new(button.on_press(GuiMessage::TabChanged(tab))).padding(2)
            }
        };

        let self_update_tab_label = format!(
            "Self-updater{}",
            match SelfUpdater::count_new(&self.self_releases) {
                Some(count) => {
                    format!(" [{}]", count)
                }
                None => {
                    String::new()
                }
            }
        );

        let tabs = Container::new(
            Row::new()
                .push(tab_button("Recent files", Tab::RecentFiles))
                .push(tab_button("Packages", Tab::Packages))
                .push(tab_button("Settings", Tab::Settings))
                .push(if get_setting().self_updater {
                    tab_button(&self_update_tab_label, Tab::SelfUpdater)
                } else {
                    Container::new(Space::with_width(Length::Units(0)))
                })
                .push(tab_button("About", Tab::About)),
        )
        .width(Length::Fill)
        .center_x()
        .style(get_setting().theme.tab_container());

        let body = match current_tab {
            Tab::RecentFiles => Tab::recent_files_body(self.file_path.clone(), &self.recent_files),
            Tab::Packages => Tab::packages_body(
                &self.packages,
                self.file_path.clone(),
                update_count,
                file_exists,
                &self.controls,
            ),
            Tab::Settings => Tab::settings_body(&self.releases, self.state.disk_space),
            Tab::SelfUpdater => Tab::self_updater_body(&self.state, &self.self_releases),
            Tab::About => Tab::about_body(),
        };

        Column::new().push(tabs).push(body).into()
    }
}
