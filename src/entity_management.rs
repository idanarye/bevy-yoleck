use bevy::asset::{AssetLoader, LoadedAsset};
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};

use crate::entity_upgrading::YoleckEntityUpgrading;
use crate::level_files_upgrading::upgrade_level_file;
use crate::{
    YoleckEditorState, YoleckEntityConstructionSpecs, YoleckEntityLifecycleStatus, YoleckManaged,
    YoleckSchedule, YoleckState,
};

/// Used by Yoleck to determine how to handle the entity.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YoleckEntryHeader {
    /// This is the name passed to [`YoleckTypeHandler`](crate::YoleckTypeHandler::new).
    #[serde(rename = "type")]
    pub type_name: String,
    /// A name to display near the entity in the entities list.
    ///
    /// This is for level editors' convenience only - it will not be used in the games.
    #[serde(default)]
    pub name: String,
}

/// An entry for a Yoleck entity, as it appears in level files.
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
    editor_state: Res<State<YoleckEditorState>>,
    mut commands: Commands,
    mut raw_entries_query: Query<(Entity, &mut YoleckRawEntry)>,
    construction_specs: Res<YoleckEntityConstructionSpecs>,
) {
    let mut entities_by_type = HashMap::<String, Vec<Entity>>::new();
    for (entity, mut raw_entry) in raw_entries_query.iter_mut() {
        entities_by_type
            .entry(raw_entry.header.type_name.clone())
            .or_default()
            .push(entity);
        let mut cmd = commands.entity(entity);
        cmd.remove::<YoleckRawEntry>();

        let mut components_data = HashMap::new();

        if let Some(entity_type_info) =
            construction_specs.get_entity_type_info(&raw_entry.header.type_name)
        {
            for component_name in entity_type_info.components.iter() {
                let Some(handler) = construction_specs.component_handlers.get(component_name) else {
                        error!("Component type {:?} is not registered", component_name);
                        continue;
                    };
                let raw_component_data = raw_entry
                    .data
                    .get_mut(handler.key())
                    .map(|component_data| component_data.take());
                handler.init_in_entity(raw_component_data, &mut cmd, &mut components_data);
            }
            for dlg in entity_type_info.on_init.iter() {
                dlg(editor_state.0, &mut cmd);
            }
        } else {
            error!("Entity type {:?} is not registered", raw_entry.header.name);
        }

        cmd.insert(YoleckManaged {
            name: raw_entry.header.name.to_owned(),
            type_name: raw_entry.header.type_name.to_owned(),
            lifecycle_status: YoleckEntityLifecycleStatus::JustCreated,
            components_data,
        });
    }
}

pub(crate) fn yoleck_prepare_populate_schedule(
    mut query: Query<(Entity, &mut YoleckManaged)>,
    mut entities_to_populate: ResMut<EntitiesToPopulate>,
    mut yoleck_state: ResMut<YoleckState>,
) {
    entities_to_populate.0.clear();
    for (entity, mut yoleck_managed) in query.iter_mut() {
        match yoleck_managed.lifecycle_status {
            YoleckEntityLifecycleStatus::Synchronized => {}
            YoleckEntityLifecycleStatus::JustCreated => {
                entities_to_populate.0.push(entity);
            }
            YoleckEntityLifecycleStatus::JustChanged => {
                entities_to_populate.0.push(entity);
                yoleck_state.level_needs_saving = true;
            }
        }
        yoleck_managed.lifecycle_status = YoleckEntityLifecycleStatus::Synchronized;
    }
}

pub(crate) fn yoleck_run_populate_schedule(world: &mut World) {
    world.run_schedule(YoleckSchedule::Populate);
}

#[derive(Resource)]
pub(crate) struct EntitiesToPopulate(pub Vec<Entity>);

pub(crate) fn yoleck_process_loading_command(
    mut commands: Commands,
    mut yoleck_loading_command: ResMut<YoleckLoadingCommand>,
    mut raw_levels_assets: ResMut<Assets<YoleckRawLevel>>,
    entity_upgrading: Option<Res<YoleckEntityUpgrading>>,
) {
    match core::mem::replace(
        yoleck_loading_command.as_mut(),
        YoleckLoadingCommand::NoCommand,
    ) {
        YoleckLoadingCommand::NoCommand => {}
        YoleckLoadingCommand::FromAsset(handle) => {
            if let Some(level) = raw_levels_assets.get_mut(&handle) {
                if let Some(entity_upgrading) = entity_upgrading {
                    entity_upgrading.upgrade_raw_level_file(level);
                }
                for entry in level.entries() {
                    commands.spawn(entry.clone());
                }
            }
        }
        YoleckLoadingCommand::FromData(mut level) => {
            if let Some(entity_upgrading) = entity_upgrading {
                entity_upgrading.upgrade_raw_level_file(&mut level);
            }
            for entry in level.into_entries() {
                commands.spawn(entry);
            }
        }
    }
}

/// Command Yoleck to load a level, represented as an asset to an handle.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::{YoleckLoadingCommand};
/// fn level_loading_system(
///     asset_server: Res<AssetServer>,
///     mut yoleck_loading_command: ResMut<YoleckLoadingCommand>,
/// ) {
///     *yoleck_loading_command = YoleckLoadingCommand::FromAsset(asset_server.load("levels/level1.yol"));
/// }
#[derive(Resource)]
pub enum YoleckLoadingCommand {
    NoCommand,
    FromAsset(Handle<YoleckRawLevel>),
    FromData(YoleckRawLevel),
}

pub(crate) struct YoleckLevelAssetLoader;

/// Represents a level file.
#[derive(TypeUuid, Debug, Serialize, Deserialize, Clone)]
#[uuid = "4b37433a-1cff-4693-b943-3fb46eaaeabc"]
pub struct YoleckRawLevel(
    pub(crate) YoleckRawLevelHeader,
    serde_json::Value, // level data
    pub(crate) Vec<YoleckRawEntry>,
);

/// Internal Yoleck metadata for a level file.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct YoleckRawLevelHeader {
    format_version: usize,
    pub app_format_version: usize,
}

impl YoleckRawLevel {
    pub fn new(
        app_format_version: usize,
        entries: impl IntoIterator<Item = YoleckRawEntry>,
    ) -> Self {
        Self(
            YoleckRawLevelHeader {
                format_version: 2,
                app_format_version,
            },
            serde_json::Value::Object(Default::default()),
            entries.into_iter().collect(),
        )
    }

    pub fn entries(&self) -> &[YoleckRawEntry] {
        &self.2
    }

    pub fn into_entries(self) -> impl Iterator<Item = YoleckRawEntry> {
        self.2.into_iter()
    }
}

impl AssetLoader for YoleckLevelAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::asset::BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let json = std::str::from_utf8(bytes)?;
            let level: serde_json::Value = serde_json::from_str(json)?;
            let level = upgrade_level_file(level)?;
            let level: YoleckRawLevel = serde_json::from_value(level)?;
            load_context.set_default_asset(LoadedAsset::new(level));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["yol"]
    }
}
