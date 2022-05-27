use std::ops::Deref;

use bevy::asset::{AssetLoader, LoadedAsset};
//use bevy::prelude::*;
use bevy::reflect::TypeUuid;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YoleckLevelIndexEntry {
    pub filename: String,
}

#[derive(TypeUuid, Debug, Serialize, Deserialize)]
#[uuid = "ca0c185d-eb75-4a19-a188-3bc633a76cf7"]
pub struct YoleckLevelIndex(YoleckLevelIndexHeader, Vec<YoleckLevelIndexEntry>);

#[derive(Debug, Serialize, Deserialize)]
pub struct YoleckLevelIndexHeader {
    format_version: usize,
}

impl YoleckLevelIndex {
    pub fn new(entries: impl IntoIterator<Item = YoleckLevelIndexEntry>) -> Self {
        Self(
            YoleckLevelIndexHeader { format_version: 1 },
            entries.into_iter().collect(),
        )
    }
}

impl Deref for YoleckLevelIndex {
    type Target = [YoleckLevelIndexEntry];

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

pub(crate) struct YoleckLevelIndexLoader;

impl AssetLoader for YoleckLevelIndexLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::asset::BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let json = std::str::from_utf8(bytes)?;
            let level_index: YoleckLevelIndex = serde_json::from_str(json)?;
            load_context.set_default_asset(LoadedAsset::new(level_index));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["yoli"]
    }
}
