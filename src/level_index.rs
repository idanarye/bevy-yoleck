use std::ops::Deref;

use bevy::asset::{AssetLoader, LoadedAsset};
//use bevy::prelude::*;
use bevy::reflect::TypeUuid;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YoleckLevelIndexEntry {
    pub filename: String,
}

#[derive(TypeUuid, Debug)]
#[uuid = "ca0c185d-eb75-4a19-a188-3bc633a76cf7"]
pub struct YoleckLevelIndex(Vec<YoleckLevelIndexEntry>);

impl Deref for YoleckLevelIndex {
    type Target = [YoleckLevelIndexEntry];

    fn deref(&self) -> &Self::Target {
        &self.0
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
            let entries: Vec<YoleckLevelIndexEntry> = serde_json::from_str(json)?;
            load_context.set_default_asset(LoadedAsset::new(YoleckLevelIndex(entries)));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["yoli"]
    }
}
