use std::collections::BTreeSet;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::utils::{BoxedFuture, HashMap, Uuid};
use serde::{Deserialize, Serialize};

use crate::editor::YoleckEditorState;
use crate::entity_upgrading::YoleckEntityUpgrading;
use crate::errors::YoleckAssetLoaderError;
use crate::level_files_upgrading::upgrade_level_file;
use crate::populating::PopulateReason;
use crate::prelude::{YoleckEntityUuid, YoleckUuidRegistry};
use crate::{
    YoleckBelongsToLevel, YoleckEntityConstructionSpecs, YoleckEntityLifecycleStatus,
    YoleckManaged, YoleckSchedule, YoleckState,
};

/// Used by Yoleck to determine how to handle the entity.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YoleckEntryHeader {
    #[serde(rename = "type")]
    pub type_name: String,
    /// A name to display near the entity in the entities list.
    ///
    /// This is for level editors' convenience only - it will not be used in the games.
    #[serde(default)]
    pub name: String,

    /// A persistable way to identify the specific entity.
    ///
    /// Will be set automatically if the entity type was defined with
    /// [`with_uuid`](crate::YoleckEntityType::with_uuid).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<Uuid>,
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
    mut raw_entries_query: Query<(Entity, &mut YoleckRawEntry), With<YoleckBelongsToLevel>>,
    construction_specs: Res<YoleckEntityConstructionSpecs>,
    mut uuid_registry: ResMut<YoleckUuidRegistry>,
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
            if entity_type_info.has_uuid {
                let uuid = raw_entry.header.uuid.unwrap_or_else(Uuid::new_v4);
                cmd.insert(YoleckEntityUuid(uuid));
                uuid_registry.0.insert(uuid, cmd.id());
            }
            for component_name in entity_type_info.components.iter() {
                let Some(handler) = construction_specs.component_handlers.get(component_name)
                else {
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
                dlg(*editor_state.get(), &mut cmd);
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
    mut yoleck_state: Option<ResMut<YoleckState>>,
    editor_state: Res<State<YoleckEditorState>>,
) {
    entities_to_populate.0.clear();
    let mut level_needs_saving = false;
    for (entity, mut yoleck_managed) in query.iter_mut() {
        match yoleck_managed.lifecycle_status {
            YoleckEntityLifecycleStatus::Synchronized => {}
            YoleckEntityLifecycleStatus::JustCreated => {
                let populate_reason = match editor_state.get() {
                    YoleckEditorState::EditorActive => PopulateReason::EditorInit,
                    YoleckEditorState::GameActive => PopulateReason::RealGame,
                };
                entities_to_populate.0.push((entity, populate_reason));
            }
            YoleckEntityLifecycleStatus::JustChanged => {
                entities_to_populate
                    .0
                    .push((entity, PopulateReason::EditorUpdate));
                level_needs_saving = true;
            }
        }
        yoleck_managed.lifecycle_status = YoleckEntityLifecycleStatus::Synchronized;
    }
    if level_needs_saving {
        if let Some(yoleck_state) = yoleck_state.as_mut() {
            yoleck_state.level_needs_saving = true;
        }
    }
}

pub(crate) fn yoleck_run_populate_schedule(world: &mut World) {
    world.run_schedule(YoleckSchedule::Populate);
    world.run_schedule(YoleckSchedule::OverrideCommonComponents);
}

#[derive(Resource)]
pub(crate) struct EntitiesToPopulate(pub Vec<(Entity, PopulateReason)>);

pub(crate) fn process_loading_command(
    query: Query<(Entity, &YoleckLoadLevel)>,
    mut raw_levels_assets: ResMut<Assets<YoleckRawLevel>>,
    entity_upgrading: Option<Res<YoleckEntityUpgrading>>,
    mut commands: Commands,
) {
    for (level_entity, load_level) in query.iter() {
        if let Some(raw_level) = raw_levels_assets.get_mut(&load_level.0) {
            if let Some(entity_upgrading) = &entity_upgrading {
                entity_upgrading.upgrade_raw_level_file(raw_level);
            }
            commands
                .entity(level_entity)
                .remove::<YoleckLoadLevel>()
                .insert(YoleckKeepLevel);
            for entry in raw_level.entries() {
                commands.spawn((
                    entry.clone(),
                    YoleckBelongsToLevel {
                        level: level_entity,
                    },
                ));
            }
        }
    }
}

pub(crate) fn process_unloading_command(
    mut removed_levels: RemovedComponents<YoleckKeepLevel>,
    level_owned_entities_query: Query<(Entity, &YoleckBelongsToLevel)>,
    mut commands: Commands,
) {
    if removed_levels.is_empty() {
        return;
    }
    let removed_levels: BTreeSet<Entity> = removed_levels.read().collect();
    for (entity, belongs_to_level) in level_owned_entities_query.iter() {
        if removed_levels.contains(&belongs_to_level.level) {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Command Yoleck to load a level.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::prelude::*;
/// fn level_loading_system(
///     asset_server: Res<AssetServer>,
///     mut commands: Commands,
/// ) {
///     commands.spawn(YoleckLoadLevel(asset_server.load("levels/level1.yol")));
/// }
/// ```
///
/// After the level is loaded, `YoleckLoadLevel` will be removed and [`YoleckKeepLevel`] will be
/// added instead. To unload the level, either remove `YoleckKeepLevel` or despawn the entire level
/// entity.
///
/// Note that the entities inside the level will _not_ be children of the level entity.
#[derive(Component)]
pub struct YoleckLoadLevel(pub Handle<YoleckRawLevel>);

#[derive(Component)]
pub struct YoleckKeepLevel;

pub(crate) struct YoleckLevelAssetLoader;

/// Represents a level file.
#[derive(Asset, TypeUuid, TypePath, Debug, Serialize, Deserialize, Clone)]
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
    type Asset = YoleckRawLevel;
    type Settings = ();
    type Error = YoleckAssetLoaderError;

    fn extensions(&self) -> &[&str] {
        &["yol"]
    }

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let json = std::str::from_utf8(&bytes)?;
            let level: serde_json::Value = serde_json::from_str(json)?;
            let level = upgrade_level_file(level)?;
            let level: YoleckRawLevel = serde_json::from_value(level)?;
            Ok(level)
        })
    }
}
