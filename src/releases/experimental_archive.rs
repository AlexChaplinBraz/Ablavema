use super::{BuilderBuild, ReleaseType};
use crate::{package::Package, settings::get_setting};
use async_trait::async_trait;
use derive_deref::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deref, DerefMut, Deserialize, PartialEq, Serialize)]
pub struct ExperimentalArchive(Vec<Package>);

#[async_trait]
impl ReleaseType for ExperimentalArchive {
    async fn fetch() -> Self {
        Self(BuilderBuild::ExperimentalArchive.fetch().await)
    }

    fn get_db_path(&self) -> PathBuf {
        get_setting().databases_dir.join("experimental_archive.ron")
    }
}
