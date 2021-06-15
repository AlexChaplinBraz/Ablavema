use super::{BuilderBuildsType, ReleaseType};
use crate::{package::Package, settings::get_setting};
use async_trait::async_trait;
use derive_deref::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deref, DerefMut, Deserialize, PartialEq, Serialize)]
pub struct Branched(Vec<Package>);

#[async_trait]
impl ReleaseType for Branched {
    async fn fetch() -> Self {
        Self::fetch_from_builder(BuilderBuildsType::Branched).await
    }

    fn get_name(&self) -> String {
        String::from("branched")
    }

    fn get_db_path(&self) -> PathBuf {
        get_setting().databases_dir.join("branched.bin")
    }
}
