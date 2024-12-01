use std::collections::BTreeSet;
use std::ops::Deref;

use bevy::asset::AssetLoader;
use bevy::prelude::*;
use bevy::reflect::TypePath;

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
///     mut level_index_handle: Local<Option<Handle<YoleckLevelIndex>>>,
///     asset_server: Res<AssetServer>,
///     level_index_assets: Res<Assets<YoleckLevelIndex>>,
///     mut commands: Commands,
/// ) {
///     # let level_number: usize = todo!();
///     // Keep the handle in local resource, so that Bevy will not unload the level index asset
///     // between frames.
///     let level_index_handle = level_index_handle
///         .get_or_insert_with(|| asset_server.load("levels/index.yoli"))
///         .clone();
///     let Some(level_index) = level_index_assets.get(&level_index_handle) else {
///         // During the first invocation of this system, the level index asset is not going to be
///         // loaded just yet. Since this system is going to run on every frame during the Loading
///         // state, it just has to keep trying until it starts in a frame where it is loaded.
///         return;
///     };
///     let level_to_load = level_index[level_number];
///     let level_handle: Handle<YoleckRawLevel> = asset_server.load(&format!("levels/{}", level_to_load.filename));
///     commands.spawn(YoleckLoadLevel(level_handle));
/// }
/// ```
#[derive(Asset, TypePath, Debug, Serialize, Deserialize)]
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

    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        _load_context: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let json = std::str::from_utf8(&bytes)?;
        let level_index: YoleckLevelIndex = serde_json::from_str(json)?;
        Ok(level_index)
    }

    fn extensions(&self) -> &[&str] {
        &["yoli"]
    }
}

/// Accessible only to edit systems - provides information about available levels.
#[derive(Resource)]
pub struct YoleckEditableLevels {
    pub(crate) levels: BTreeSet<String>,
}

impl YoleckEditableLevels {
    /// The names of the level files (relative to the levels directory, not the assets directory)
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.levels.iter().map(|l| l.as_str())
    }
}
