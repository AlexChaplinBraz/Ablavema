//#![allow(dead_code, unused_imports, unused_variables)]
pub mod archived;
pub mod branched;
pub mod daily;
pub mod installed;
pub mod lts;
pub mod stable;
use self::{
    archived::Archived, branched::Branched, daily::Daily, installed::Installed, lts::Lts,
    stable::Stable,
};
use crate::{
    helpers::ReturnOption,
    package::{Build, Package, PackageState, PackageStatus},
    settings::{CAN_CONNECT, SETTINGS},
};
use async_trait::async_trait;
use bincode;
use indicatif::MultiProgress;
use lazy_static::initialize;
use reqwest;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::{remove_file, File},
    iter, mem, ops,
    path::PathBuf,
    sync::atomic::Ordering,
    time::SystemTime,
};

#[derive(Debug, Default)]
pub struct Releases {
    pub daily: Daily,
    pub branched: Branched,
    pub stable: Stable,
    pub lts: Lts,
    pub archived: Archived,
    pub installed: Installed,
}

impl Releases {
    /// Load databases and sync them with installed packages,
    /// returning Releases and true if initialised.
    pub async fn init() -> (Releases, bool) {
        initialize(&SETTINGS);
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

        if self.branched.get_db_path().exists() {
            if self.branched.load() {
                initialised = self.branched.init().await;
            }
        } else {
            initialised = self.branched.init().await;
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
        self.daily
            .refresh_status(SETTINGS.read().unwrap().update_daily);

        self.branched.refresh_state(&self.installed);
        self.branched
            .refresh_status(SETTINGS.read().unwrap().update_branched);

        self.stable.refresh_state(&self.installed);
        self.stable
            .refresh_status(SETTINGS.read().unwrap().update_stable);

        self.lts.refresh_state(&self.installed);
        self.lts.refresh_status(SETTINGS.read().unwrap().update_lts);

        self.archived.refresh_state(&self.installed);
    }

    /// Check for new packages. This returns a tuple where the first item is a boolean
    /// that indicates whether there were any new packages found.
    pub async fn check_updates(
        packages: (Daily, Branched, Stable, Lts),
    ) -> (bool, Daily, Branched, Stable, Lts) {
        SETTINGS.write().unwrap().last_update_time = SystemTime::now();
        SETTINGS.read().unwrap().save();

        let (mut daily, mut branched, mut stable, mut lts) = packages;

        let mut updated_daily = false;
        if SETTINGS.read().unwrap().update_daily {
            let (updated, fetched_daily) = Releases::check_daily_updates(daily).await;
            updated_daily = updated;
            daily = fetched_daily;
        }

        let mut updated_branched = false;
        if SETTINGS.read().unwrap().update_branched {
            let (updated, fetched_branched) = Releases::check_branched_updates(branched).await;
            updated_branched = updated;
            branched = fetched_branched;
        }

        let mut updated_stable = false;
        if SETTINGS.read().unwrap().update_stable {
            let (updated, fetched_stable) = Releases::check_stable_updates(stable).await;
            updated_stable = updated;
            stable = fetched_stable;
        }

        let mut updated_lts = false;
        if SETTINGS.read().unwrap().update_lts {
            let (updated, fetched_lts) = Releases::check_lts_updates(lts).await;
            updated_lts = updated;
            lts = fetched_lts;
        }

        (
            updated_daily || updated_branched || updated_stable || updated_lts,
            daily,
            branched,
            stable,
            lts,
        )
    }

    /// Used for getting the packages for `Releases::check_updates()`.
    pub fn take(&mut self) -> (Daily, Branched, Stable, Lts) {
        (
            self.daily.take(),
            self.branched.take(),
            self.stable.take(),
            self.lts.take(),
        )
    }

    /// Used for adding the results of `Releases::check_updates()`
    /// back into the variable and syncing.
    pub fn add_new_packages(&mut self, packages: (bool, Daily, Branched, Stable, Lts)) {
        self.daily = packages.1;
        self.branched = packages.2;
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

    pub async fn check_branched_updates(mut branched: Branched) -> (bool, Branched) {
        print!("Checking for branched updates... ");
        match branched.get_new_packages().await {
            Some(new_packages) => {
                println!("Found:");
                branched.add_new_packages(new_packages);
                branched.remove_dead_packages().await;
                branched.save();
                (true, branched)
            }
            None => {
                println!("None found.");
                (false, branched)
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
    /// (all_count, daily_count, branched_count, stable_count, lts_count)
    pub fn count_updates(
        &self,
    ) -> (
        Option<usize>,
        Option<usize>,
        Option<usize>,
        Option<usize>,
        Option<usize>,
    ) {
        let daily_count = self
            .daily
            .iter()
            .filter(|package| package.status == PackageStatus::Update)
            .count();
        let branched_count = self
            .branched
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
        let all_count = daily_count + branched_count + stable_count + lts_count;

        (
            all_count.return_option(),
            daily_count.return_option(),
            branched_count.return_option(),
            stable_count.return_option(),
            lts_count.return_option(),
        )
    }

    /// Installs the latest packages for each build, as long as there's one older package
    /// of that build already installed. Operates based on user settings, so it updates only the
    /// enabled types and can delete old packages of the same build. Can also update the default
    /// package to the latest of its build type if one was installed.
    pub async fn cli_install_updates(&mut self) {
        let updates_found = iter::empty()
            .chain(self.daily.iter())
            .chain(self.branched.iter())
            .chain(self.stable.iter())
            .chain(self.lts.iter())
            .chain(self.archived.iter())
            .filter(|package| package.status == PackageStatus::Update)
            .collect::<Vec<_>>();

        if updates_found.is_empty() {
            println!("No updates to install.");
        } else {
            let multi_progress = MultiProgress::new();
            let mut install_completion = Vec::new();

            for package in updates_found {
                install_completion.push(
                    package
                        .cli_install(&multi_progress, &(true, true))
                        .await
                        .unwrap(),
                );
            }

            multi_progress.join().unwrap();
            for handle in install_completion {
                handle.await.unwrap();
            }

            self.installed.fetch();
            self.installed.update_default();
            let (daily_removed, branched_removed) = self.installed.remove_old_packages();
            self.sync();

            if SETTINGS.read().unwrap().keep_only_latest_daily && daily_removed {
                self.daily.remove_dead_packages().await;
            }

            if SETTINGS.read().unwrap().keep_only_latest_branched && branched_removed {
                self.branched.remove_dead_packages().await;
            }
        }
    }
}

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
                        Build::Daily(_) | Build::Branched(_) => {
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
                    Build::Daily(_) | Build::Branched(_) => {
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

    fn purge(&mut self) {
        let database = self.get_db_path();
        if database.exists() {
            remove_file(database).unwrap();
            *self = Self::default();
        }
    }
}
