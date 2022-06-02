use super::{
    extra::{BuildTypeSettings, Choice, Location},
    package::PackageMessage,
    sort_by::SortBy,
    style::Theme,
    tabs::recent_files::{RecentFile, RecentFileMessage},
    Gui, Tab,
};
use crate::{
    helpers::open_blender,
    package::{Build, Package},
    releases::{
        daily_archive::DailyArchive, daily_latest::DailyLatest,
        experimental_archive::ExperimentalArchive, experimental_latest::ExperimentalLatest,
        lts::Lts, patch_archive::PatchArchive, patch_latest::PatchLatest,
        stable_archive::StableArchive, stable_latest::StableLatest, ReleaseType,
    },
    settings::{
        get_setting, save_settings, set_setting, ModifierKey, FETCHING, INSTALLING, PROJECT_DIRS,
    },
};
use iced::Command;
use native_dialog::{FileDialog, MessageDialog, MessageType};
use self_update::update::Release;
use std::{
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
    process::exit,
    sync::atomic::Ordering,
};

#[derive(Clone, Debug)]
pub enum GuiMessage {
    PackageMessage((usize, PackageMessage)),
    RecentFileMessage((String, RecentFileMessage)),
    Bookmark(Package),
    CheckAvailability(Option<(bool, bool, Package)>),
    InstallPackage(Package),
    CancelInstall(Package),
    PackageInstalled(Package),
    PackageRemoved(Package),
    OpenBlender(String),
    OpenBlenderWithFile(String),
    SelectFile,
    OpenBrowser(String),
    CheckForUpdates,
    UpdatesChecked(
        (
            bool,
            DailyLatest,
            ExperimentalLatest,
            PatchLatest,
            StableLatest,
            Lts,
        ),
    ),
    FetchAll,
    AllFetched(
        (
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
        ),
    ),
    // TODO: Consider reducing all these message variations to one with an enum.
    FetchDailyLatest,
    DailyLatestFetched((bool, DailyLatest)),
    FetchDailyArchive,
    DailyArchiveFetched((bool, DailyArchive)),
    FetchExperimentalLatest,
    ExperimentalLatestFetched((bool, ExperimentalLatest)),
    FetchExperimentalArchive,
    ExperimentalArchiveFetched((bool, ExperimentalArchive)),
    FetchPatchLatest,
    PatchLatestFetched((bool, PatchLatest)),
    FetchPatchArchive,
    PatchArchiveFetched((bool, PatchArchive)),
    FetchStableLatest,
    StableLatestFetched((bool, StableLatest)),
    FetchStableArchive,
    StableArchiveFetched((bool, StableArchive)),
    FetchLts,
    LtsFetched((bool, Lts)),
    FilterUpdatesChanged(bool),
    FilterBookmarksChanged(bool),
    FilterInstalledChanged(bool),
    FilterAllChanged(bool),
    FilterDailyLatestChanged(bool),
    FilterDailyArchiveChanged(bool),
    FilterExperimentalLatestChanged(bool),
    FilterExperimentalArchiveChanged(bool),
    FilterPatchLatestChanged(bool),
    FilterPatchArchiveChanged(bool),
    FilterStableLatestChanged(bool),
    FilterStableArchiveChanged(bool),
    FilterLtsChanged(bool),
    SortingChanged(SortBy),
    TabChanged(Tab),
    BypassLauncher(Choice),
    ModifierKey(ModifierKey),
    UseLatestAsDefault(Choice),
    CheckUpdatesAtLaunch(Choice),
    MinutesBetweenUpdatesChanged(i64),
    UpdateDailyLatest(Choice),
    UpdateExperimentalLatest(Choice),
    UpdatePatchLatest(Choice),
    UpdateStableLatest(Choice),
    UpdateLts(Choice),
    ThemeChanged(Theme),
    ChangeLocation(Location),
    ResetLocation(Location),
    RemoveDatabases(BuildTypeSettings),
    RemovePackages(BuildTypeSettings),
    RemoveCache,
    SelfUpdater(Choice),
    CheckSelfUpdatesAtLaunch(Choice),
    FetchSelfReleases,
    PopulateSelfReleases(Option<Vec<Release>>),
    PickListVersionSelected(String),
    ChangeVersion,
    VersionChanged(()),
    CheckConnection,
    ConnectionChecked(()),
}

impl Gui {
    pub fn update_message(&mut self, message: GuiMessage) -> Command<GuiMessage> {
        match message {
            GuiMessage::PackageMessage((index, package_message)) => {
                match self.packages.get_mut(index) {
                    Some(package) => package.update(package_message),
                    None => unreachable!("index out of bounds"),
                }
            }
            GuiMessage::RecentFileMessage((file, recent_file_message)) => match recent_file_message
            {
                RecentFileMessage::OpenWithLastBlender(blender) => {
                    self.file_path = Some(file);
                    Command::perform(Gui::pass_string(blender), GuiMessage::OpenBlenderWithFile)
                }
                RecentFileMessage::OpenWithDefaultBlender => {
                    self.file_path = Some(file);
                    Command::perform(
                        Gui::pass_string(get_setting().default_package.clone().unwrap().name),
                        GuiMessage::OpenBlenderWithFile,
                    )
                }
                RecentFileMessage::Select => {
                    self.file_path = Some(file);
                    Command::none()
                }
                RecentFileMessage::Remove => {
                    set_setting().recent_files.remove(&PathBuf::from(file));
                    save_settings();
                    self.recent_files = get_setting().recent_files.to_vec();
                    Command::none()
                }
            },
            GuiMessage::Bookmark(package) => {
                set_setting().bookmarks.update(package.name);
                set_setting().bookmarks.clean(&self.packages);
                save_settings();
                Command::none()
            }
            GuiMessage::CheckAvailability(option) => match option {
                Some((available, for_install, package)) => {
                    if available && for_install {
                        Command::perform(Gui::pass_package(package), GuiMessage::InstallPackage)
                    } else if !for_install {
                        self.sync();
                        Command::none()
                    } else {
                        match package.build {
                            Build::DailyLatest(_) => {
                                let index = self
                                    .releases
                                    .daily_latest
                                    .iter()
                                    .position(|a_package| *a_package == package)
                                    .unwrap();
                                self.releases.daily_latest.remove(index);
                                self.releases.daily_latest.save();
                            }
                            Build::DailyArchive(_) => {
                                let index = self
                                    .releases
                                    .daily_archive
                                    .iter()
                                    .position(|a_package| *a_package == package)
                                    .unwrap();
                                self.releases.daily_archive.remove(index);
                                self.releases.daily_archive.save();
                            }
                            Build::ExperimentalLatest(_) => {
                                let index = self
                                    .releases
                                    .experimental_latest
                                    .iter()
                                    .position(|a_package| *a_package == package)
                                    .unwrap();
                                self.releases.experimental_latest.remove(index);
                                self.releases.experimental_latest.save();
                            }
                            Build::ExperimentalArchive(_) => {
                                let index = self
                                    .releases
                                    .experimental_archive
                                    .iter()
                                    .position(|a_package| *a_package == package)
                                    .unwrap();
                                self.releases.experimental_archive.remove(index);
                                self.releases.experimental_archive.save();
                            }
                            Build::PatchLatest(_) => {
                                let index = self
                                    .releases
                                    .patch_latest
                                    .iter()
                                    .position(|a_package| *a_package == package)
                                    .unwrap();
                                self.releases.patch_latest.remove(index);
                                self.releases.patch_latest.save();
                            }
                            Build::PatchArchive(_) => {
                                let index = self
                                    .releases
                                    .patch_archive
                                    .iter()
                                    .position(|a_package| *a_package == package)
                                    .unwrap();
                                self.releases.patch_archive.remove(index);
                                self.releases.patch_archive.save();
                            }
                            Build::StableLatest => {
                                let index = self
                                    .releases
                                    .stable_latest
                                    .iter()
                                    .position(|a_package| *a_package == package)
                                    .unwrap();
                                self.releases.stable_latest.remove(index);
                                self.releases.stable_latest.save();
                            }
                            Build::StableArchive => {
                                let index = self
                                    .releases
                                    .stable_archive
                                    .iter()
                                    .position(|a_package| *a_package == package)
                                    .unwrap();
                                self.releases.stable_archive.remove(index);
                                self.releases.stable_archive.save();
                            }
                            Build::Lts => {
                                let index = self
                                    .releases
                                    .lts
                                    .iter()
                                    .position(|a_package| *a_package == package)
                                    .unwrap();
                                self.releases.lts.remove(index);
                                self.releases.lts.save();
                            }
                        }
                        if for_install {
                            let message =
                                format!("Package '{}' is no longer available.", package.name);
                            if MessageDialog::new()
                                .set_type(MessageType::Info)
                                .set_title("Ablavema")
                                .set_text(&message)
                                .show_alert()
                                .is_err()
                            {
                                // TODO: Show a tooltip if dependencies not found.
                                // Or just spawn a tooltip in the first place.
                                #[cfg(target_os = "linux")]
                                println!(
                                    "Error: {}\nProbably need to install 'zenity' or 'kdialog' for a graphical dialog.",
                                    &message
                                );
                            }
                        }
                        self.sync();
                        Command::none()
                    }
                }
                None => {
                    self.sync();
                    Command::none()
                }
            },
            GuiMessage::InstallPackage(package) => {
                if self.installing.is_empty() {
                    INSTALLING.store(true, Ordering::Relaxed);
                }
                self.installing.push(package);
                Command::none()
            }
            GuiMessage::CancelInstall(package) => {
                let index = self
                    .installing
                    .iter()
                    .enumerate()
                    .find(|(_, a_package)| *a_package == &package)
                    .unwrap()
                    .0;
                self.installing.remove(index);
                if self.installing.is_empty() {
                    INSTALLING.store(false, Ordering::Relaxed);
                }
                Command::none()
            }
            GuiMessage::PackageInstalled(package) => {
                let index = self
                    .installing
                    .iter()
                    .enumerate()
                    .find(|(_, a_package)| *a_package == &package)
                    .unwrap()
                    .0;
                self.installing.remove(index);
                self.releases.installed.fetch();
                self.releases.installed.update_default();
                self.sync();
                if self.installing.is_empty() {
                    INSTALLING.store(false, Ordering::Relaxed);
                }
                Command::none()
            }
            GuiMessage::PackageRemoved(package) => {
                let default_package_option = get_setting().default_package.clone();
                if let Some(default_package) = default_package_option {
                    if default_package == package {
                        set_setting().default_package = None;
                        save_settings();
                    }
                }
                Command::perform(
                    Gui::check_availability(false, package),
                    GuiMessage::CheckAvailability,
                )
            }
            GuiMessage::OpenBlender(package) => {
                open_blender(package, None);
                exit(0);
            }
            GuiMessage::OpenBlenderWithFile(package) => {
                let file_path = self.file_path.clone().unwrap();
                let path = PathBuf::from(&file_path);
                let recent_file = RecentFile::new(path.clone(), package.clone());
                set_setting().recent_files.insert(path, recent_file);
                save_settings();
                open_blender(package, Some(file_path));
                exit(0);
            }
            GuiMessage::SelectFile => {
                if let Some(new_file_path) = FileDialog::new()
                    .add_filter("BLEND archive", &["blend*"])
                    .add_filter("All files", &["*"])
                    .show_open_single_file()
                    .unwrap()
                {
                    self.file_path = Some(new_file_path.to_str().unwrap().to_string());
                }
                Command::none()
            }
            GuiMessage::OpenBrowser(url) => {
                let _ = webbrowser::open(&url);
                Command::none()
            }
            GuiMessage::CheckForUpdates => {
                FETCHING.store(true, Ordering::Relaxed);
                Command::perform(
                    Gui::check_for_updates(self.releases.take()),
                    GuiMessage::UpdatesChecked,
                )
            }
            GuiMessage::UpdatesChecked(tuple) => {
                self.releases.add_new_packages(tuple);
                self.sync();
                FETCHING.store(false, Ordering::Relaxed);
                Command::none()
            }
            GuiMessage::FetchAll => {
                FETCHING.store(true, Ordering::Relaxed);
                Command::perform(
                    Gui::check_all(
                        self.releases.daily_latest.take(),
                        self.releases.daily_archive.take(),
                        self.releases.experimental_latest.take(),
                        self.releases.experimental_archive.take(),
                        self.releases.patch_latest.take(),
                        self.releases.patch_archive.take(),
                        self.releases.stable_latest.take(),
                        self.releases.stable_archive.take(),
                        self.releases.lts.take(),
                    ),
                    GuiMessage::AllFetched,
                )
            }
            GuiMessage::AllFetched((
                _,
                daily_latest,
                daily_archive,
                experimental_latest,
                experimental_archive,
                patch_latest,
                patch_archive,
                stable_latest,
                stable_archive,
                lts,
            )) => {
                self.releases.daily_latest = daily_latest;
                self.releases.daily_archive = daily_archive;
                self.releases.experimental_latest = experimental_latest;
                self.releases.experimental_archive = experimental_archive;
                self.releases.patch_latest = patch_latest;
                self.releases.patch_archive = patch_archive;
                self.releases.stable_latest = stable_latest;
                self.releases.stable_archive = stable_archive;
                self.releases.lts = lts;
                self.sync();
                FETCHING.store(false, Ordering::Relaxed);
                Command::none()
            }
            GuiMessage::FetchDailyLatest => {
                FETCHING.store(true, Ordering::Relaxed);
                Command::perform(
                    Gui::check_daily_latest(self.releases.daily_latest.take()),
                    GuiMessage::DailyLatestFetched,
                )
            }
            GuiMessage::DailyLatestFetched((_, daily_latest)) => {
                self.releases.daily_latest = daily_latest;
                self.sync();
                FETCHING.store(false, Ordering::Relaxed);
                Command::none()
            }
            GuiMessage::FetchDailyArchive => {
                FETCHING.store(true, Ordering::Relaxed);
                Command::perform(
                    Gui::check_daily_archive(self.releases.daily_archive.take()),
                    GuiMessage::DailyArchiveFetched,
                )
            }
            GuiMessage::DailyArchiveFetched((_, daily_archive)) => {
                self.releases.daily_archive = daily_archive;
                self.sync();
                FETCHING.store(false, Ordering::Relaxed);
                Command::none()
            }
            GuiMessage::FetchExperimentalLatest => {
                FETCHING.store(true, Ordering::Relaxed);
                Command::perform(
                    Gui::check_experimental_latest(self.releases.experimental_latest.take()),
                    GuiMessage::ExperimentalLatestFetched,
                )
            }
            GuiMessage::ExperimentalLatestFetched((_, experimental_latest)) => {
                self.releases.experimental_latest = experimental_latest;
                self.sync();
                FETCHING.store(false, Ordering::Relaxed);
                Command::none()
            }
            GuiMessage::FetchExperimentalArchive => {
                FETCHING.store(true, Ordering::Relaxed);
                Command::perform(
                    Gui::check_experimental_archive(self.releases.experimental_archive.take()),
                    GuiMessage::ExperimentalArchiveFetched,
                )
            }
            GuiMessage::ExperimentalArchiveFetched((_, experimental_archive)) => {
                self.releases.experimental_archive = experimental_archive;
                self.sync();
                FETCHING.store(false, Ordering::Relaxed);
                Command::none()
            }
            GuiMessage::FetchPatchLatest => {
                FETCHING.store(true, Ordering::Relaxed);
                Command::perform(
                    Gui::check_patch_latest(self.releases.patch_latest.take()),
                    GuiMessage::PatchLatestFetched,
                )
            }
            GuiMessage::PatchLatestFetched((_, patch_latest)) => {
                self.releases.patch_latest = patch_latest;
                self.sync();
                FETCHING.store(false, Ordering::Relaxed);
                Command::none()
            }
            GuiMessage::FetchPatchArchive => {
                FETCHING.store(true, Ordering::Relaxed);
                Command::perform(
                    Gui::check_patch_archive(self.releases.patch_archive.take()),
                    GuiMessage::PatchArchiveFetched,
                )
            }
            GuiMessage::PatchArchiveFetched((_, patch_archive)) => {
                self.releases.patch_archive = patch_archive;
                self.sync();
                FETCHING.store(false, Ordering::Relaxed);
                Command::none()
            }
            GuiMessage::FetchStableLatest => {
                FETCHING.store(true, Ordering::Relaxed);
                Command::perform(
                    Gui::check_stable_latest(self.releases.stable_latest.take()),
                    GuiMessage::StableLatestFetched,
                )
            }
            GuiMessage::StableLatestFetched((_, stable_latest)) => {
                self.releases.stable_latest = stable_latest;
                self.sync();
                FETCHING.store(false, Ordering::Relaxed);
                Command::none()
            }
            GuiMessage::FetchStableArchive => {
                FETCHING.store(true, Ordering::Relaxed);
                Command::perform(
                    Gui::check_stable_archive(self.releases.stable_archive.take()),
                    GuiMessage::StableArchiveFetched,
                )
            }
            GuiMessage::StableArchiveFetched((_, stable_archive)) => {
                self.releases.stable_archive = stable_archive;
                self.sync();
                FETCHING.store(false, Ordering::Relaxed);
                Command::none()
            }
            GuiMessage::FetchLts => {
                FETCHING.store(true, Ordering::Relaxed);
                Command::perform(
                    Gui::check_lts(self.releases.lts.take()),
                    GuiMessage::LtsFetched,
                )
            }
            GuiMessage::LtsFetched((_, lts)) => {
                self.releases.lts = lts;
                self.sync();
                FETCHING.store(false, Ordering::Relaxed);
                Command::none()
            }
            GuiMessage::FilterUpdatesChanged(change) => {
                set_setting().filters.updates = change;
                save_settings();
                Command::none()
            }

            GuiMessage::FilterBookmarksChanged(change) => {
                set_setting().filters.bookmarks = change;
                save_settings();
                Command::none()
            }
            GuiMessage::FilterInstalledChanged(change) => {
                set_setting().filters.installed = change;
                save_settings();
                Command::none()
            }
            GuiMessage::FilterAllChanged(change) => {
                set_setting().filters.all = change;
                set_setting().filters.daily_latest = change;
                set_setting().filters.daily_archive = change;
                set_setting().filters.experimental_latest = change;
                set_setting().filters.experimental_archive = change;
                set_setting().filters.patch_latest = change;
                set_setting().filters.patch_archive = change;
                set_setting().filters.stable_latest = change;
                set_setting().filters.stable_archive = change;
                set_setting().filters.lts = change;
                save_settings();
                Command::none()
            }
            GuiMessage::FilterDailyLatestChanged(change) => {
                set_setting().filters.daily_latest = change;
                set_setting().filters.refresh_all();
                save_settings();
                Command::none()
            }
            GuiMessage::FilterDailyArchiveChanged(change) => {
                set_setting().filters.daily_archive = change;
                set_setting().filters.refresh_all();
                save_settings();
                Command::none()
            }
            GuiMessage::FilterExperimentalLatestChanged(change) => {
                set_setting().filters.experimental_latest = change;
                set_setting().filters.refresh_all();
                save_settings();
                Command::none()
            }
            GuiMessage::FilterExperimentalArchiveChanged(change) => {
                set_setting().filters.experimental_archive = change;
                set_setting().filters.refresh_all();
                save_settings();
                Command::none()
            }
            GuiMessage::FilterPatchLatestChanged(change) => {
                set_setting().filters.patch_latest = change;
                set_setting().filters.refresh_all();
                save_settings();
                Command::none()
            }
            GuiMessage::FilterPatchArchiveChanged(change) => {
                set_setting().filters.patch_archive = change;
                set_setting().filters.refresh_all();
                save_settings();
                Command::none()
            }
            GuiMessage::FilterStableLatestChanged(change) => {
                set_setting().filters.stable_latest = change;
                set_setting().filters.refresh_all();
                save_settings();
                Command::none()
            }
            GuiMessage::FilterStableArchiveChanged(change) => {
                set_setting().filters.stable_archive = change;
                set_setting().filters.refresh_all();
                save_settings();
                Command::none()
            }
            GuiMessage::FilterLtsChanged(change) => {
                set_setting().filters.lts = change;
                set_setting().filters.refresh_all();
                save_settings();
                Command::none()
            }
            GuiMessage::SortingChanged(sort_by) => {
                set_setting().sort_by = sort_by;
                save_settings();
                Command::none()
            }
            GuiMessage::TabChanged(tab) => {
                set_setting().tab = tab;
                save_settings();
                Command::none()
            }
            GuiMessage::BypassLauncher(choice) => {
                match choice {
                    Choice::Enable => set_setting().bypass_launcher = true,
                    Choice::Disable => set_setting().bypass_launcher = false,
                }
                save_settings();
                Command::none()
            }
            GuiMessage::ModifierKey(modifier_key) => {
                set_setting().modifier_key = modifier_key;
                save_settings();
                Command::none()
            }
            GuiMessage::UseLatestAsDefault(choice) => {
                match choice {
                    Choice::Enable => set_setting().use_latest_as_default = true,
                    Choice::Disable => set_setting().use_latest_as_default = false,
                }
                save_settings();
                Command::none()
            }
            GuiMessage::CheckUpdatesAtLaunch(choice) => {
                match choice {
                    Choice::Enable => set_setting().check_updates_at_launch = true,
                    Choice::Disable => set_setting().check_updates_at_launch = false,
                }
                save_settings();
                Command::none()
            }
            GuiMessage::MinutesBetweenUpdatesChanged(change) => {
                if change.is_positive() {
                    let mut current = get_setting().minutes_between_updates;
                    current += change as u64;
                    if current > 1440 {
                        set_setting().minutes_between_updates = 1440;
                    } else {
                        set_setting().minutes_between_updates = current;
                    }
                } else {
                    let current = get_setting().minutes_between_updates;
                    set_setting().minutes_between_updates =
                        current.saturating_sub(change.abs() as u64);
                }
                save_settings();
                Command::none()
            }
            GuiMessage::UpdateDailyLatest(choice) => {
                match choice {
                    Choice::Enable => set_setting().update_daily_latest = true,
                    Choice::Disable => set_setting().update_daily_latest = false,
                }
                save_settings();
                self.sync();
                Command::none()
            }
            GuiMessage::UpdateExperimentalLatest(choice) => {
                match choice {
                    Choice::Enable => set_setting().update_experimental_latest = true,
                    Choice::Disable => set_setting().update_experimental_latest = false,
                }
                save_settings();
                self.sync();
                Command::none()
            }
            GuiMessage::UpdatePatchLatest(choice) => {
                match choice {
                    Choice::Enable => set_setting().update_patch_latest = true,
                    Choice::Disable => set_setting().update_patch_latest = false,
                }
                save_settings();
                self.sync();
                Command::none()
            }
            GuiMessage::UpdateStableLatest(choice) => {
                match choice {
                    Choice::Enable => set_setting().update_stable_latest = true,
                    Choice::Disable => set_setting().update_stable_latest = false,
                }
                save_settings();
                self.sync();
                Command::none()
            }
            GuiMessage::UpdateLts(choice) => {
                match choice {
                    Choice::Enable => set_setting().update_lts = true,
                    Choice::Disable => set_setting().update_lts = false,
                }
                save_settings();
                self.sync();
                Command::none()
            }
            GuiMessage::ThemeChanged(theme) => {
                set_setting().theme = theme;
                save_settings();
                Command::none()
            }
            GuiMessage::ChangeLocation(location) => {
                match location {
                    Location::Databases => {
                        if let Some(directory) = FileDialog::new().show_open_single_dir().unwrap() {
                            set_setting().databases_dir = directory;
                            save_settings();
                        }
                    }
                    Location::Packages => {
                        if let Some(directory) = FileDialog::new().show_open_single_dir().unwrap() {
                            set_setting().packages_dir = directory;
                            save_settings();
                            self.sync();
                        }
                    }
                    Location::Cache => {
                        if let Some(directory) = FileDialog::new().show_open_single_dir().unwrap() {
                            set_setting().cache_dir = directory;
                            save_settings();
                        }
                    }
                }
                Command::none()
            }
            GuiMessage::ResetLocation(location) => {
                match location {
                    Location::Databases => {
                        set_setting().databases_dir = PROJECT_DIRS.config_dir().to_path_buf();
                        save_settings();
                    }
                    Location::Packages => {
                        set_setting().packages_dir = PROJECT_DIRS.data_local_dir().to_path_buf();
                        save_settings();
                        self.sync();
                    }
                    Location::Cache => {
                        set_setting().cache_dir = PROJECT_DIRS.cache_dir().to_path_buf();
                        save_settings();
                    }
                }
                Command::none()
            }
            GuiMessage::RemoveDatabases(build_type) => {
                match build_type {
                    BuildTypeSettings::All => {
                        remove_dir_all(&get_setting().databases_dir).unwrap();
                        create_dir_all(&get_setting().databases_dir).unwrap();
                    }
                    BuildTypeSettings::DailyLatest => {
                        self.releases.daily_latest.remove_db();
                    }
                    BuildTypeSettings::DailyArchive => {
                        self.releases.daily_archive.remove_db();
                    }
                    BuildTypeSettings::ExperimentalLatest => {
                        self.releases.experimental_latest.remove_db();
                    }
                    BuildTypeSettings::ExperimentalArchive => {
                        self.releases.experimental_archive.remove_db();
                    }
                    BuildTypeSettings::PatchLatest => {
                        self.releases.patch_latest.remove_db();
                    }
                    BuildTypeSettings::PatchArchive => {
                        self.releases.patch_archive.remove_db();
                    }
                    BuildTypeSettings::StableLatest => {
                        self.releases.stable_latest.remove_db();
                    }
                    BuildTypeSettings::StableArchive => {
                        self.releases.stable_archive.remove_db();
                    }
                    BuildTypeSettings::Lts => {
                        self.releases.lts.remove_db();
                    }
                }
                self.sync();
                Command::none()
            }
            GuiMessage::RemovePackages(build_type) => {
                match build_type {
                    BuildTypeSettings::All => {
                        self.releases.installed.remove_all();
                    }
                    BuildTypeSettings::DailyLatest => {
                        self.releases.installed.remove_daily_latest();
                    }
                    BuildTypeSettings::DailyArchive => {
                        self.releases.installed.remove_daily_archive();
                    }
                    BuildTypeSettings::ExperimentalLatest => {
                        self.releases.installed.remove_experimental_latest();
                    }
                    BuildTypeSettings::ExperimentalArchive => {
                        self.releases.installed.remove_experimental_archive();
                    }
                    BuildTypeSettings::PatchLatest => {
                        self.releases.installed.remove_patch_latest();
                    }
                    BuildTypeSettings::PatchArchive => {
                        self.releases.installed.remove_patch_archive();
                    }
                    BuildTypeSettings::StableLatest => {
                        self.releases.installed.remove_stable_latest();
                    }
                    BuildTypeSettings::StableArchive => {
                        self.releases.installed.remove_stable_archive();
                    }
                    BuildTypeSettings::Lts => {
                        self.releases.installed.remove_lts();
                    }
                }
                self.sync();
                Command::none()
            }
            GuiMessage::RemoveCache => {
                remove_dir_all(&get_setting().cache_dir).unwrap();
                create_dir_all(&get_setting().cache_dir).unwrap();
                Command::none()
            }
            GuiMessage::SelfUpdater(choice) => {
                match choice {
                    Choice::Enable => set_setting().self_updater = true,
                    Choice::Disable => set_setting().self_updater = false,
                }
                save_settings();
                Command::none()
            }
            GuiMessage::CheckSelfUpdatesAtLaunch(choice) => {
                match choice {
                    Choice::Enable => set_setting().check_self_updates_at_launch = true,
                    Choice::Disable => set_setting().check_self_updates_at_launch = false,
                }
                save_settings();
                Command::none()
            }
            GuiMessage::FetchSelfReleases => {
                self.state.fetching_releases = true;
                Command::perform(Gui::fetch_self_releases(), GuiMessage::PopulateSelfReleases)
            }
            GuiMessage::PopulateSelfReleases(self_releases) => {
                self.self_releases = self_releases;
                if let Some(s_releases) = &self.self_releases {
                    self.state.release_versions = s_releases
                        .iter()
                        .map(|release| release.version.clone())
                        .collect();
                }
                self.state.fetching_releases = false;
                Command::none()
            }
            GuiMessage::PickListVersionSelected(version) => {
                self.state.pick_list_selected_releases = version;
                Command::none()
            }
            GuiMessage::ChangeVersion => {
                self.state.installing_release = true;
                Command::perform(
                    Gui::change_self_version(
                        self.self_releases.clone().unwrap(),
                        self.state.pick_list_selected_releases.clone(),
                    ),
                    GuiMessage::VersionChanged,
                )
            }
            GuiMessage::VersionChanged(()) => {
                self.state.installing_release = false;
                self.state.installed_release = true;
                Command::none()
            }
            GuiMessage::CheckConnection => {
                self.controls.checking_connection = true;
                Command::perform(Gui::check_connection(), GuiMessage::ConnectionChecked)
            }
            GuiMessage::ConnectionChecked(()) => {
                self.controls.checking_connection = false;
                Command::none()
            }
        }
    }
}
