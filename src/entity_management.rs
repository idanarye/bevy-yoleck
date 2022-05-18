use bevy::asset::{AssetLoader, LoadedAsset};
use bevy::ecs::system::CommandQueue;
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};

use crate::api::YoleckUserSystemContext;
use crate::{YoleckEditorState, YoleckManaged, YoleckTypeHandlers};

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

pub(crate) fn yoleck_process_raw_entries(world: &mut World) {
    let is_in_editor = match world.resource::<State<YoleckEditorState>>().current() {
        YoleckEditorState::EditorActive => true,
        YoleckEditorState::GameActive => false,
    };
    world.resource_scope(|world, mut yoleck_type_handlers: Mut<YoleckTypeHandlers>| {
        let mut entities_by_type = HashMap::<String, Vec<Entity>>::new();
        let mut commands_queue = CommandQueue::default();
        let mut raw_entries_query = world.query::<(Entity, &YoleckRawEntry)>();
        let mut commands = Commands::new(&mut commands_queue, world);
        for (entity, raw_entry) in raw_entries_query.iter(world) {
            entities_by_type
                .entry(raw_entry.header.type_name.clone())
                .or_default()
                .push(entity);
            let mut cmd = commands.entity(entity);
            cmd.remove::<YoleckRawEntry>();
            let handler = yoleck_type_handlers
                .type_handlers
                .get(&raw_entry.header.type_name)
                .unwrap();
            let concrete = handler.make_concrete(raw_entry.data.clone()).unwrap();
            cmd.insert(YoleckManaged {
                name: raw_entry.header.name.to_owned(),
                type_name: raw_entry.header.type_name.to_owned(),
                data: concrete,
            });
        }
        commands_queue.apply(world);
        for (type_name, entities) in entities_by_type {
            let handler = yoleck_type_handlers
                .type_handlers
                .get_mut(&type_name)
                .unwrap();
            *world.resource_mut::<YoleckUserSystemContext>() =
                YoleckUserSystemContext::PopulateInitiated {
                    is_in_editor,
                    entities,
                };
            handler.run_populate_systems(world);
        }
        *world.resource_mut::<YoleckUserSystemContext>() = YoleckUserSystemContext::Nope;
    });
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
