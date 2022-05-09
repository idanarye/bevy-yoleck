use bevy::asset::{AssetLoader, LoadedAsset};
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use serde::{Deserialize, Serialize};

use crate::api::PopulateReason;
use crate::{YoleckEditorState, YoleckManaged, YoleckPopulateContext, YoleckTypeHandlers};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YoleckEntryHeader {
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Component, Debug, Clone)]
pub struct YoleckRawEntry {
    pub header: YoleckEntryHeader,
    pub data: serde_json::Value,
}

impl Serialize for YoleckRawEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (&self.header, &self.data).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for YoleckRawEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (header, data): (YoleckEntryHeader, serde_json::Value) =
            Deserialize::deserialize(deserializer)?;
        Ok(Self { header, data })
    }
}

pub(crate) fn yoleck_process_raw_entries(
    raw_entries_query: Query<(Entity, &YoleckRawEntry)>,
    mut commands: Commands,
    yoleck_type_handlers: Res<YoleckTypeHandlers>,
    editor_state: Res<State<YoleckEditorState>>,
    asset_server: Res<AssetServer>,
) {
    let populate_reason = match editor_state.current() {
        YoleckEditorState::EditorActive => PopulateReason::EditorInit,
        YoleckEditorState::GameActive => PopulateReason::RealGame,
    };
    for (entity, raw_entry) in raw_entries_query.iter() {
        let mut cmd = commands.entity(entity);
        cmd.remove::<YoleckRawEntry>();
        let handler = yoleck_type_handlers
            .type_handlers
            .get(&raw_entry.header.type_name)
            .unwrap();
        let concrete = handler.make_concrete(raw_entry.data.clone()).unwrap();
        let populate_ctx = YoleckPopulateContext {
            asset_server: &asset_server,
            reason: populate_reason,
            _phantom_data: Default::default(),
        };
        handler.populate(&concrete, &populate_ctx, &mut cmd);
        cmd.insert(YoleckManaged {
            name: raw_entry.header.name.to_owned(),
            type_name: raw_entry.header.type_name.to_owned(),
            data: concrete,
        });
    }
}

pub(crate) fn yoleck_process_loading_command(
    mut commands: Commands,
    mut yoleck_loading_command: ResMut<YoleckLoadingCommand>,
    raw_levels_assets: Res<Assets<YoleckRawLevel>>,
) {
    if let YoleckLoadingCommand::FromAsset(handle) = &*yoleck_loading_command {
        if let Some(asset) = raw_levels_assets.get(handle) {
            *yoleck_loading_command = YoleckLoadingCommand::NoCommand;
            for entry in asset.entries.iter() {
                commands.spawn().insert(entry.clone());
            }
        }
    }
}

pub enum YoleckLoadingCommand {
    NoCommand,
    FromAsset(Handle<YoleckRawLevel>),
}

pub(crate) struct YoleckLevelAssetLoader;

#[derive(TypeUuid)]
#[uuid = "4b37433a-1cff-4693-b943-3fb46eaaeabc"]
pub struct YoleckRawLevel {
    pub entries: Vec<YoleckRawEntry>,
}

impl AssetLoader for YoleckLevelAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::asset::BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let json = std::str::from_utf8(bytes)?;
            let entries: Vec<YoleckRawEntry> = serde_json::from_str(json)?;
            load_context.set_default_asset(LoadedAsset::new(YoleckRawLevel { entries }));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["yol"]
    }
}
