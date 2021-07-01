pub mod daily_archive;
pub mod daily_latest;
pub mod experimental_archive;
pub mod experimental_latest;
pub mod installed;
pub mod lts;
pub mod patch_archive;
pub mod patch_latest;
pub mod stable_archive;
pub mod stable_latest;
use self::{
    daily_archive::DailyArchive, daily_latest::DailyLatest,
    experimental_archive::ExperimentalArchive, experimental_latest::ExperimentalLatest,
    installed::Installed, lts::Lts, patch_archive::PatchArchive, patch_latest::PatchLatest,
    stable_archive::StableArchive, stable_latest::StableLatest,
};
use crate::{
    helpers::{get_document, get_file_stem, ReturnOption},
    package::{Build, Os, Package, PackageState, PackageStatus},
    settings::{get_setting, init_settings, save_settings, set_setting, CAN_CONNECT},
};
use async_trait::async_trait;
use chrono::{Datelike, NaiveDateTime, Utc};
use select::predicate::{And, Class, Name};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::{remove_file, File},
    iter, mem, ops,
    path::PathBuf,
    sync::atomic::Ordering,
    time::SystemTime,
};
use versions::Versioning;

#[derive(Debug, Default)]
pub struct Releases {
    pub daily_latest: DailyLatest,
    pub daily_archive: DailyArchive,
    pub experimental_latest: ExperimentalLatest,
    pub experimental_archive: ExperimentalArchive,
    pub patch_latest: PatchLatest,
    pub patch_archive: PatchArchive,
    pub stable_latest: StableLatest,
    pub stable_archive: StableArchive,
    pub lts: Lts,
    pub installed: Installed,
}

impl Releases {
    /// Load databases and sync them with the installed packages.
    pub async fn init() -> Releases {
        init_settings();
        let mut releases = Releases::default();
        releases.load_all().await;
        releases.sync();
        releases
    }

    /// Load all databases, or initialise them if non-existent.
    /// Also removes the databases if the Package struct changed.
    async fn load_all(&mut self) {
        self.daily_latest.load();
        self.daily_archive.load();
        self.experimental_latest.load();
        self.experimental_archive.load();
        self.patch_latest.load();
        self.patch_archive.load();
        self.stable_latest.load();
        self.stable_archive.load();
        self.lts.load();
    }

    /// Refreshes the state and status of all packages.
    pub fn sync(&mut self) {
        // TODO: Consider changing how latest packages that became archived are handled.
        // I should probably check if a latest package appeared in the archive and remove it from
        // the latest list. If this is implemented I should change how BuildType works, since they
        // wouldn't be able to be loaded at the same time anyway.
        self.installed.fetch();

        self.daily_latest.refresh_state(&self.installed);
        self.daily_latest
            .refresh_status(get_setting().update_daily_latest);

        self.daily_archive.refresh_state(&self.installed);

        self.experimental_latest.refresh_state(&self.installed);
        self.experimental_latest
            .refresh_status(get_setting().update_experimental_latest);

        self.experimental_archive.refresh_state(&self.installed);

        self.patch_latest.refresh_state(&self.installed);
        self.patch_latest
            .refresh_status(get_setting().update_patch_latest);

        self.patch_archive.refresh_state(&self.installed);

        self.stable_latest.refresh_state(&self.installed);
        self.stable_latest
            .refresh_status(get_setting().update_stable_latest);

        self.stable_archive.refresh_state(&self.installed);

        self.lts.refresh_state(&self.installed);
        self.lts.refresh_status(get_setting().update_lts);
    }

    /// Check for new packages. This returns a tuple where the first item is a boolean
    /// that indicates whether there were any new packages found.
    pub async fn check_updates(
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
        set_setting().last_update_time = SystemTime::now();
        save_settings();

        let (
            mut daily_latest,
            mut experimental_latest,
            mut patch_latest,
            mut stable_latest,
            mut lts,
        ) = packages;

        let mut updated_daily_latest = false;
        if get_setting().update_daily_latest && daily_latest.get_db_path().exists() {
            let (updated, fetched_daily_latest) = DailyLatest::check_updates(daily_latest).await;
            updated_daily_latest = updated;
            daily_latest = fetched_daily_latest;
        }

        let mut updated_experimental_latest = false;
        if get_setting().update_experimental_latest && experimental_latest.get_db_path().exists() {
            let (updated, fetched_experimental_latest) =
                ExperimentalLatest::check_updates(experimental_latest).await;
            updated_experimental_latest = updated;
            experimental_latest = fetched_experimental_latest;
        }

        let mut updated_patch_latest = false;
        if get_setting().update_patch_latest && patch_latest.get_db_path().exists() {
            let (updated, fetched_patch_latest) = PatchLatest::check_updates(patch_latest).await;
            updated_patch_latest = updated;
            patch_latest = fetched_patch_latest;
        }

        let mut updated_stable_latest = false;
        if get_setting().update_stable_latest && stable_latest.get_db_path().exists() {
            let (updated, fetched_stable_latest) = StableLatest::check_updates(stable_latest).await;
            updated_stable_latest = updated;
            stable_latest = fetched_stable_latest;
        }

        let mut updated_lts = false;
        if get_setting().update_lts && lts.get_db_path().exists() {
            let (updated, fetched_lts) = Lts::check_updates(lts).await;
            updated_lts = updated;
            lts = fetched_lts;
        }

        (
            updated_daily_latest
                || updated_experimental_latest
                || updated_patch_latest
                || updated_stable_latest
                || updated_lts,
            daily_latest,
            experimental_latest,
            patch_latest,
            stable_latest,
            lts,
        )
    }

    /// Used for getting the packages for `Releases::check_updates()`.
    pub fn take(
        &mut self,
    ) -> (
        DailyLatest,
        ExperimentalLatest,
        PatchLatest,
        StableLatest,
        Lts,
    ) {
        (
            self.daily_latest.take(),
            self.experimental_latest.take(),
            self.patch_latest.take(),
            self.stable_latest.take(),
            self.lts.take(),
        )
    }

    /// Used for adding the results of `Releases::check_updates()` back into itself.
    pub fn add_new_packages(
        &mut self,
        packages: (
            bool,
            DailyLatest,
            ExperimentalLatest,
            PatchLatest,
            StableLatest,
            Lts,
        ),
    ) {
        self.daily_latest = packages.1;
        self.experimental_latest = packages.2;
        self.patch_latest = packages.3;
        self.stable_latest = packages.4;
        self.lts = packages.5;
    }

    /// Returns the amount of updates for each build type if there are any.
    pub fn count_updates(&self) -> UpdateCount {
        let daily_count = self
            .daily_latest
            .iter()
            .filter(|package| package.status == PackageStatus::Update)
            .count();
        let experimental_count = self
            .experimental_latest
            .iter()
            .filter(|package| package.status == PackageStatus::Update)
            .count();
        let patch_count = self
            .patch_latest
            .iter()
            .filter(|package| package.status == PackageStatus::Update)
            .count();
        let stable_count = self
            .stable_latest
            .iter()
            .filter(|package| package.status == PackageStatus::Update)
            .count();
        let lts_count = self
            .lts
            .iter()
            .filter(|package| package.status == PackageStatus::Update)
            .count();
        let all_count = daily_count + experimental_count + patch_count + stable_count + lts_count;

        UpdateCount {
            all: all_count.return_option(),
            daily: daily_count.return_option(),
            experimental: experimental_count.return_option(),
            patch: patch_count.return_option(),
            stable: stable_count.return_option(),
            lts: lts_count.return_option(),
        }
    }

    pub fn build_vec(&self) -> Vec<Package> {
        let mut index = 0;
        let mut packages: Vec<Package> = Vec::new();

        for package in iter::empty()
            .chain(self.daily_latest.iter())
            .chain(self.daily_archive.iter())
            .chain(self.experimental_latest.iter())
            .chain(self.experimental_archive.iter())
            .chain(self.patch_latest.iter())
            .chain(self.patch_archive.iter())
            .chain(self.stable_latest.iter())
            .chain(self.stable_archive.iter())
            .chain(self.lts.iter())
        {
            match packages
                .iter_mut()
                .find(|a_package| a_package.name == package.name)
            {
                Some(found_package) => {
                    found_package.build_type.update(&package.build);
                }
                None => {
                    let mut new_package = package.clone();
                    new_package.index = index;
                    new_package.build_type.update(&package.build);
                    packages.push(new_package);
                    index += 1;
                }
            }
        }

        packages
    }
}

pub struct UpdateCount {
    pub all: Option<usize>,
    pub daily: Option<usize>,
    pub experimental: Option<usize>,
    pub patch: Option<usize>,
    pub stable: Option<usize>,
    pub lts: Option<usize>,
}

#[async_trait]
pub trait ReleaseType:
    Sized
    + Sync
    + Default
    + Serialize
    + DeserializeOwned
    + ops::Deref<Target = Vec<Package>>
    + ops::DerefMut<Target = Vec<Package>>
{
    async fn fetch() -> Self;

    async fn check_updates(mut packages: Self) -> (bool, Self) {
        match packages.get_new_packages().await {
            Some(new_packages) => {
                packages.add_new_packages(new_packages);
                packages.save();
                (true, packages)
            }
            None => (false, packages),
        }
    }

    async fn get_new_packages(&self) -> Option<Self> {
        let mut fetched_packages = Self::fetch().await;
        let mut new_packages = Self::default();

        for package in &mut *fetched_packages {
            if !self.contains(package) {
                new_packages.push(package.take());
            }
        }

        if new_packages.is_empty() {
            None
        } else {
            Some(new_packages)
        }
    }

    fn add_new_packages(&mut self, mut new_packages: Self) {
        for mut package in new_packages.iter_mut() {
            package.status = PackageStatus::New;
            self.push(package.take());
        }
        self.sort();
    }

    fn refresh_state(&mut self, installed: &Installed) {
        for package in self.iter_mut() {
            if matches!(package.state, PackageState::Installed { .. }) {
                package.state = PackageState::default();
            }
            if installed.contains(package) {
                package.state = PackageState::default_installed();
            }
        }
    }

    fn unset_status(&mut self) {
        for package in self.iter_mut() {
            if package.status == PackageStatus::Update {
                package.status = PackageStatus::Old;
            }
        }
    }

    fn refresh_status(&mut self, refresh: bool) {
        self.unset_status();

        if !refresh {
            return;
        }

        let mut installed_packages: Vec<Package> = Vec::new();

        for package in self.iter() {
            if matches!(package.state, PackageState::Installed { .. }) {
                match package.build {
                    Build::DailyLatest(_)
                    | Build::ExperimentalLatest(_)
                    | Build::PatchLatest(_) => {
                        match installed_packages.iter().find(|installed_package| {
                            installed_package.version == package.version
                                && installed_package.build == package.build
                        }) {
                            Some(_) => continue,
                            None => installed_packages.push(package.clone()),
                        }
                    }
                    Build::StableLatest => {
                        installed_packages.push(package.clone());
                        break;
                    }
                    Build::Lts => {
                        if installed_packages
                            .iter()
                            .find(|installed_package| {
                                installed_package.version.nth(0).unwrap()
                                    == package.version.nth(0).unwrap()
                                    && installed_package.version.nth(1).unwrap()
                                        == package.version.nth(1).unwrap()
                            })
                            .is_none()
                        {
                            installed_packages.push(package.clone());
                        }
                    }
                    Build::DailyArchive(_)
                    | Build::ExperimentalArchive(_)
                    | Build::PatchArchive(_)
                    | Build::StableArchive => {
                        break;
                    }
                }
            }
        }

        for installed_package in installed_packages {
            match installed_package.build {
                Build::DailyLatest(_) | Build::ExperimentalLatest(_) | Build::PatchLatest(_) => {
                    if let Some(package) = self.iter_mut().find(|package| {
                        installed_package.version == package.version
                            && installed_package.build == package.build
                    }) {
                        if package.date > installed_package.date {
                            package.status = PackageStatus::Update;
                        }
                    }
                }
                Build::StableLatest => {
                    if let Some(package) = self
                        .iter_mut()
                        .find(|package| installed_package.build == package.build)
                    {
                        if package.date > installed_package.date {
                            package.status = PackageStatus::Update;
                        }
                    }
                }
                Build::Lts => {
                    if let Some(package) = self.iter_mut().find(|package| {
                        installed_package.version.nth(0).unwrap() == package.version.nth(0).unwrap()
                            && installed_package.version.nth(1).unwrap()
                                == package.version.nth(1).unwrap()
                    }) {
                        if package.date > installed_package.date {
                            package.status = PackageStatus::Update;
                        }
                    }
                }
                Build::DailyArchive(_)
                | Build::ExperimentalArchive(_)
                | Build::PatchArchive(_)
                | Build::StableArchive => {
                    break;
                }
            }
        }
    }

    /// This method tends to temporarily ban the user due to the large amount of requests sent
    /// over a short period of time, so it shouldn't be used in places like .sync().
    /// It's better to check the availability of a package on Un/Installing.
    async fn remove_dead_packages(&mut self) {
        // TODO: Figure out what to do with dead packages.
        // Now that there are more categories fetching from the experimental builds, it's much
        // easier to get temp banned.
        if CAN_CONNECT.load(Ordering::Relaxed) {
            let mut checkables = Vec::new();
            for (index, package) in self.iter().enumerate() {
                if !matches!(package.state, PackageState::Installed { .. }) {
                    checkables.push((index, package.url.clone()));
                }
            }

            let mut handles = Vec::new();
            for (index, url) in checkables {
                let handle = tokio::task::spawn(async move {
                    if reqwest::get(&url).await.unwrap().status().is_client_error() {
                        Some(index)
                    } else {
                        None
                    }
                });
                handles.push(handle);
            }

            let mut removed = 0;
            for handle in handles {
                if let Some(index) = handle.await.unwrap() {
                    self.remove(index - removed);
                    removed += 1;
                }
            }
        }
    }

    fn take(&mut self) -> Self {
        mem::take(self)
    }

    fn get_db_path(&self) -> PathBuf;

    fn save(&self) {
        let file = File::create(self.get_db_path()).unwrap();
        bincode::serialize_into(file, self).unwrap();
    }

    fn load(&mut self) {
        if let Ok(file) = File::open(self.get_db_path()) {
            match bincode::deserialize_from(file) {
                Ok(bin) => {
                    *self = bin;
                }
                Err(e) => {
                    // TODO: Consider moving to a diferent serialiser.
                    // Since even after Package was modified bincode may just error with:
                    // memory allocation of 7809lotsofbytes6536 bytes failed
                    // abort (core dumped)
                    eprintln!("Failed to load database with: {}.", e);
                    self.remove_db();
                }
            }
        }
    }

    fn remove_db(&mut self) {
        let database = self.get_db_path();
        if database.exists() {
            remove_file(database).unwrap();
            *self = Self::default();
        }
    }
}

pub enum BuilderBuild {
    DailyLatest,
    DailyArchive,
    ExperimentalLatest,
    ExperimentalArchive,
    PatchLatest,
    PatchArchive,
}

impl BuilderBuild {
    pub async fn fetch(&self) -> Vec<Package> {
        let url = match self {
            BuilderBuild::DailyLatest => "https://builder.blender.org/download/daily/",
            BuilderBuild::DailyArchive => "https://builder.blender.org/download/daily/archive/",
            BuilderBuild::ExperimentalLatest => {
                "https://builder.blender.org/download/experimental/"
            }
            BuilderBuild::ExperimentalArchive => {
                "https://builder.blender.org/download/experimental/archive/"
            }
            BuilderBuild::PatchLatest => "https://builder.blender.org/download/patch/",
            BuilderBuild::PatchArchive => "https://builder.blender.org/download/patch/archive/",
        };
        let document = get_document(url).await;
        let mut packages = Vec::new();

        let (platform, os) = {
            if cfg!(target_os = "linux") {
                ("platform-linux", Os::Linux)
            } else if cfg!(target_os = "windows") {
                ("platform-windows", Os::Windows)
            } else if cfg!(target_os = "macos") {
                ("platform-darwin", Os::MacOs)
            } else {
                unreachable!("Unsupported OS");
            }
        };

        let builds_list = document
            .find(And(Class("builds-list"), Class(platform)))
            .next()
            .unwrap();

        for build_node in builds_list.find(Class("os")) {
            let url = build_node
                .find(Name("a"))
                .next()
                .unwrap()
                .attr("href")
                .unwrap()
                .to_string();

            if url.ends_with(".sha256") {
                continue;
            }

            let name = get_file_stem(&url).to_string();

            let build_name = build_node.find(Class("build-var")).next().unwrap().text();
            let build = match self {
                BuilderBuild::DailyLatest => Build::DailyLatest(build_name),
                BuilderBuild::DailyArchive => Build::DailyArchive(build_name),
                BuilderBuild::ExperimentalLatest => Build::ExperimentalLatest(build_name),
                BuilderBuild::ExperimentalArchive => Build::ExperimentalArchive(build_name),
                BuilderBuild::PatchLatest => Build::PatchLatest(build_name),
                BuilderBuild::PatchArchive => Build::PatchArchive(build_name),
            };

            let version = Versioning::new(
                build_node
                    .find(Class("name"))
                    .next()
                    .unwrap()
                    .text()
                    .split_whitespace()
                    .nth(1)
                    .unwrap(),
            )
            .unwrap();

            let small_subtext = build_node.find(Name("small")).next().unwrap().text();
            let parts: Vec<&str> = small_subtext.split_terminator(" - ").collect();
            let date_string = format!("{}-{}", parts[0], Utc::today().year());
            let date = NaiveDateTime::parse_from_str(&date_string, "%B %d, %T-%Y").unwrap();

            let package = Package {
                version,
                name,
                build,
                date,
                commit: parts[2].to_string(),
                url,
                os,
                ..Default::default()
            };

            packages.push(package);
        }

        packages.sort();
        packages
    }
}
