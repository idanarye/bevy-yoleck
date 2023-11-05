use std::ops::Deref;

use bevy::asset::{AssetLoader, AsyncReadExt};
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};

use serde::{Deserialize, Serialize};

use crate::errors::YoleckAssetLoaderError;

/// Describes a level in the index.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YoleckLevelIndexEntry {
    /// The name of the file containing the level, relative to where the levels index file is.
    pub filename: String,
}

/// An asset loaded from a `.yoli` file (usually `index.yoli`) representing the game's levels.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::prelude::*;
/// fn load_level_system(
///     asset_server: Res<AssetServer>,
///     level_index_assets: Res<Assets<YoleckLevelIndex>>,
///     mut yoleck_loading_command: ResMut<YoleckLoadingCommand>,
/// ) {
///     # let level_number: usize = todo!();
///     let level_index_handle: Handle<YoleckLevelIndex> = asset_server.load("levels/index.yoli");
///     if let Some(level_index) = level_index_assets.get(&level_index_handle) {
///         let level_to_load = level_index[level_number];
///         let level_handle: Handle<YoleckRawLevel> = asset_server.load(&format!("levels/{}", level_to_load.filename));
///         *yoleck_loading_command = YoleckLoadingCommand::FromAsset(level_handle);
///     }
/// }
/// ```
#[derive(Asset, TypeUuid, TypePath, Debug, Serialize, Deserialize)]
#[uuid = "ca0c185d-eb75-4a19-a188-3bc633a76cf7"]
pub struct YoleckLevelIndex(YoleckLevelIndexHeader, Vec<YoleckLevelIndexEntry>);

/// Internal Yoleck metadata for the levels index.
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
    type Asset = YoleckLevelIndex;
    type Settings = ();
    type Error = YoleckAssetLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let json = std::str::from_utf8(&bytes)?;
            let level_index: YoleckLevelIndex = serde_json::from_str(json)?;
            Ok(level_index)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["yoli"]
    }
}
