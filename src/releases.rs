pub mod archived;
pub mod daily;
pub mod experimental;
pub mod installed;
pub mod lts;
pub mod stable;
use self::{
    archived::Archived, daily::Daily, experimental::Experimental, installed::Installed, lts::Lts,
    stable::Stable,
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
    mem, ops,
    path::PathBuf,
    sync::atomic::Ordering,
    time::SystemTime,
};
use versions::Versioning;

#[derive(Debug, Default)]
pub struct Releases {
    pub daily: Daily,
    pub experimental: Experimental,
    pub stable: Stable,
    pub lts: Lts,
    pub archived: Archived,
    pub installed: Installed,
}

impl Releases {
    /// Load databases and sync them with installed packages,
    /// returning Releases and true if initialised.
    pub async fn init() -> (Releases, bool) {
        init_settings();
        let mut releases = Releases::default();
        let initialised = releases.load_all().await;
        releases.sync();
        (releases, initialised)
    }

    /// Load all databases, or initialise them if non-existent.
    /// Also reinitialises databases if the Package struct changed.
    /// Returns true if initialised.
    async fn load_all(&mut self) -> bool {
        let mut initialised = false;

        if self.daily.get_db_path().exists() {
            if self.daily.load() {
                initialised = self.daily.init().await;
            }
        } else {
            initialised = self.daily.init().await;
        }

        if self.experimental.get_db_path().exists() {
            if self.experimental.load() {
                initialised = self.experimental.init().await;
            }
        } else {
            initialised = self.experimental.init().await;
        }

        if self.stable.get_db_path().exists() {
            if self.stable.load() {
                initialised = self.stable.init().await;
            }
        } else {
            initialised = self.stable.init().await;
        }

        if self.lts.get_db_path().exists() {
            if self.lts.load() {
                initialised = self.lts.init().await;
            }
        } else {
            initialised = self.lts.init().await;
        }

        if self.archived.get_db_path().exists() {
            if self.archived.load() {
                initialised = self.archived.init().await;
            }
        } else {
            initialised = self.archived.init().await;
        }

        initialised
    }

    /// Refreshes the state and status of all packages.
    pub fn sync(&mut self) {
        self.installed.fetch();

        self.daily.refresh_state(&self.installed);
        self.daily.refresh_status(get_setting().update_daily);

        self.experimental.refresh_state(&self.installed);
        self.experimental
            .refresh_status(get_setting().update_experimental);

        self.stable.refresh_state(&self.installed);
        self.stable.refresh_status(get_setting().update_stable);

        self.lts.refresh_state(&self.installed);
        self.lts.refresh_status(get_setting().update_lts);

        self.archived.refresh_state(&self.installed);
    }

    /// Check for new packages. This returns a tuple where the first item is a boolean
    /// that indicates whether there were any new packages found.
    pub async fn check_updates(
        packages: (Daily, Experimental, Stable, Lts),
    ) -> (bool, Daily, Experimental, Stable, Lts) {
        set_setting().last_update_time = SystemTime::now();
        save_settings();

        let (mut daily, mut experimental, mut stable, mut lts) = packages;

        let mut updated_daily = false;
        if get_setting().update_daily {
            let (updated, fetched_daily) = Releases::check_daily_updates(daily).await;
            updated_daily = updated;
            daily = fetched_daily;
        }

        let mut updated_experimental = false;
        if get_setting().update_experimental {
            let (updated, fetched_experimental) =
                Releases::check_experimental_updates(experimental).await;
            updated_experimental = updated;
            experimental = fetched_experimental;
        }

        let mut updated_stable = false;
        if get_setting().update_stable {
            let (updated, fetched_stable) = Releases::check_stable_updates(stable).await;
            updated_stable = updated;
            stable = fetched_stable;
        }

        let mut updated_lts = false;
        if get_setting().update_lts {
            let (updated, fetched_lts) = Releases::check_lts_updates(lts).await;
            updated_lts = updated;
            lts = fetched_lts;
        }

        (
            updated_daily || updated_experimental || updated_stable || updated_lts,
            daily,
            experimental,
            stable,
            lts,
        )
    }

    /// Used for getting the packages for `Releases::check_updates()`.
    pub fn take(&mut self) -> (Daily, Experimental, Stable, Lts) {
        (
            self.daily.take(),
            self.experimental.take(),
            self.stable.take(),
            self.lts.take(),
        )
    }

    /// Used for adding the results of `Releases::check_updates()`
    /// back into the variable and syncing.
    pub fn add_new_packages(&mut self, packages: (bool, Daily, Experimental, Stable, Lts)) {
        self.daily = packages.1;
        self.experimental = packages.2;
        self.stable = packages.3;
        self.lts = packages.4;
        self.sync();
    }

    pub async fn check_daily_updates(mut daily: Daily) -> (bool, Daily) {
        print!("Checking for daily updates... ");
        match daily.get_new_packages().await {
            Some(new_packages) => {
                println!("Found:");
                daily.add_new_packages(new_packages);
                daily.remove_dead_packages().await;
                daily.save();
                (true, daily)
            }
            None => {
                println!("None found.");
                (false, daily)
            }
        }
    }

    pub async fn check_experimental_updates(
        mut experimental: Experimental,
    ) -> (bool, Experimental) {
        print!("Checking for experimental updates... ");
        match experimental.get_new_packages().await {
            Some(new_packages) => {
                println!("Found:");
                experimental.add_new_packages(new_packages);
                experimental.remove_dead_packages().await;
                experimental.save();
                (true, experimental)
            }
            None => {
                println!("None found.");
                (false, experimental)
            }
        }
    }

    pub async fn check_stable_updates(mut stable: Stable) -> (bool, Stable) {
        print!("Checking for stable updates... ");
        match stable.get_new_packages().await {
            Some(new_packages) => {
                println!("Found:");
                stable.add_new_packages(new_packages);
                stable.save();
                (true, stable)
            }
            None => {
                println!("None found.");
                (false, stable)
            }
        }
    }

    pub async fn check_lts_updates(mut lts: Lts) -> (bool, Lts) {
        print!("Checking for LTS updates... ");
        match lts.get_new_packages().await {
            Some(new_packages) => {
                println!("Found:");
                lts.add_new_packages(new_packages);
                lts.save();
                (true, lts)
            }
            None => {
                println!("None found.");
                (false, lts)
            }
        }
    }

    pub async fn check_archived_updates(mut archived: Archived) -> (bool, Archived) {
        print!("Checking for archived updates... ");
        match archived.get_new_packages().await {
            Some(new_packages) => {
                println!("Found:");
                archived.add_new_packages(new_packages);
                archived.save();
                (true, archived)
            }
            None => {
                println!("None found.");
                (false, archived)
            }
        }
    }

    /// Returns the amount of updates for each build type if there are any.
    /// The returned tuple of options is:
    /// (all_count, daily_count, experimental_count, stable_count, lts_count)
    pub fn count_updates(&self) -> UpdateCount {
        let daily_count = self
            .daily
            .iter()
            .filter(|package| package.status == PackageStatus::Update)
            .count();
        let experimental_count = self
            .experimental
            .iter()
            .filter(|package| package.status == PackageStatus::Update)
            .count();
        let stable_count = self
            .stable
            .iter()
            .filter(|package| package.status == PackageStatus::Update)
            .count();
        let lts_count = self
            .lts
            .iter()
            .filter(|package| package.status == PackageStatus::Update)
            .count();
        let all_count = daily_count + experimental_count + stable_count + lts_count;

        UpdateCount {
            all: all_count.return_option(),
            daily: daily_count.return_option(),
            experimental: experimental_count.return_option(),
            stable: stable_count.return_option(),
            lts: lts_count.return_option(),
        }
    }
}

pub struct UpdateCount {
    pub all: Option<usize>,
    pub daily: Option<usize>,
    pub experimental: Option<usize>,
    pub stable: Option<usize>,
    pub lts: Option<usize>,
}

// TODO: Add a "last fetched date-time" to each release type.
// This would be useful for letting the user know how long ago was the last time a release type
// was fetched. Would probably have to be through a bin file, instead of a field somewhere.
// TODO: Don't initialise on first startup.
// It takes time and the user may not even want all of the release types.
#[async_trait]
pub trait ReleaseType:
    Sized
    + Default
    + Serialize
    + DeserializeOwned
    + ops::Deref<Target = Vec<Package>>
    + ops::DerefMut<Target = Vec<Package>>
{
    async fn fetch() -> Self;

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
            println!("    {} | {}", package.name, package.date);
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

        if refresh {
            let mut installed_packages: Vec<Package> = Vec::new();
            for package in self.iter() {
                if matches!(package.state, PackageState::Installed { .. }) {
                    match package.build {
                        Build::Daily(_) | Build::Experimental(_) => {
                            match installed_packages.iter().find(|installed_package| {
                                installed_package.version == package.version
                                    && installed_package.build == package.build
                            }) {
                                Some(_) => continue,
                                None => installed_packages.push(package.clone()),
                            }
                        }
                        Build::Stable => {
                            installed_packages.push(package.clone());
                            break;
                        }
                        Build::Lts => {
                            match installed_packages.iter().find(|installed_package| {
                                installed_package.version.nth(0).unwrap()
                                    == package.version.nth(0).unwrap()
                                    && installed_package.version.nth(1).unwrap()
                                        == package.version.nth(1).unwrap()
                            }) {
                                Some(_) => break,
                                None => installed_packages.push(package.clone()),
                            }
                        }
                        Build::Archived => {
                            break;
                        }
                    }
                }
            }

            for installed_package in installed_packages {
                match installed_package.build {
                    Build::Daily(_) | Build::Experimental(_) => {
                        if let Some(package) = self.iter_mut().find(|package| {
                            installed_package.version == package.version
                                && installed_package.build == package.build
                        }) {
                            if package.date > installed_package.date {
                                package.status = PackageStatus::Update;
                            }
                        }
                    }
                    Build::Stable => {
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
                            installed_package.version.nth(0).unwrap()
                                == package.version.nth(0).unwrap()
                                && installed_package.version.nth(1).unwrap()
                                    == package.version.nth(1).unwrap()
                        }) {
                            if package.date > installed_package.date {
                                package.status = PackageStatus::Update;
                            }
                        }
                    }
                    Build::Archived => {
                        continue;
                    }
                }
            }
        }
    }

    /// This method tends to temporarily ban the user due to the large amount of requests sent
    /// over a short period of time, so it shouldn't be used in places like .sync().
    /// It's better to check the availability of a package on Un/Installing.
    async fn remove_dead_packages(&mut self) {
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

            let mut deviation = 0;
            for handle in handles {
                if let Some(index) = handle.await.unwrap() {
                    self.remove(index - deviation);
                    deviation += 1;
                }
            }
        }
    }

    fn take(&mut self) -> Self {
        mem::take(self)
    }

    fn get_name(&self) -> String;

    /// Fetches packages and saves them. Always returns true.
    async fn init(&mut self) -> bool {
        print!(
            "No database for {} packages found. Fetching... ",
            self.get_name()
        );
        *self = Self::fetch().await;
        self.save();
        println!("Done");
        true
    }

    fn get_db_path(&self) -> PathBuf;

    fn save(&self) {
        let file = File::create(self.get_db_path()).unwrap();
        bincode::serialize_into(file, self).unwrap();
    }

    /// Returns true if Self changed in any way so it can be reinitialised.
    fn load(&mut self) -> bool {
        let file = File::open(self.get_db_path()).unwrap();
        match bincode::deserialize_from(file) {
            Ok(bin) => {
                *self = bin;
                false
            }
            Err(_) => true,
        }
    }

    fn remove_db(&mut self) {
        let database = self.get_db_path();
        let mut name = self.get_name();
        if let Some(c) = name.get_mut(0..1) {
            c.make_ascii_uppercase();
        }
        if database.exists() {
            remove_file(database).unwrap();
            println!("{} database removed.", name);
            *self = Self::default();
        }
    }
}

pub enum BuilderBuildsType {
    // TODO: Add support for all the other new types.
    // Would require more than a few changes to the GUI and the logic behind some parts.
    Daily,
    Experimental,
}

impl BuilderBuildsType {
    const DAILY_URL: &'static str = "https://builder.blender.org/download/daily/";
    const EXPERIMENTAL_URL: &'static str = "https://builder.blender.org/download/experimental/";

    pub async fn fetch(&self) -> Vec<Package> {
        let url = match self {
            BuilderBuildsType::Daily => Self::DAILY_URL,
            BuilderBuildsType::Experimental => Self::EXPERIMENTAL_URL,
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
                BuilderBuildsType::Daily => Build::Daily(build_name),
                BuilderBuildsType::Experimental => Build::Experimental(build_name),
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
