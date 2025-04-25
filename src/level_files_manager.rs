use std::path::PathBuf;
use std::{fs, io};

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::platform::collections::HashSet;
use bevy_egui::egui;

use crate::entity_management::{
    YoleckEntryHeader, YoleckKeepLevel, YoleckLoadLevel, YoleckRawEntry,
};
use crate::entity_upgrading::YoleckEntityUpgrading;
use crate::exclusive_systems::YoleckActiveExclusiveSystem;
use crate::knobs::YoleckKnobsCache;
use crate::level_files_upgrading::upgrade_level_file;
use crate::level_index::YoleckLevelIndexEntry;
use crate::prelude::{YoleckEditorState, YoleckEntityUuid};
use crate::{
    YoleckEditableLevels, YoleckEntityConstructionSpecs, YoleckLevelInEditor,
    YoleckLevelInPlaytest, YoleckLevelIndex, YoleckManaged, YoleckRawLevel, YoleckState,
};

const EXTENSION: &str = ".yol";
const EXTENSION_WITHOUT_DOT: &str = "yol";

/// The path for the levels directory.
///
/// [The plugin](crate::YoleckPluginForEditor) sets it to `./assets/levels/`, but it can be set to
/// other values:
/// ```no_run
/// # use std::path::Path;
/// # use bevy::prelude::*;
/// # use bevy_yoleck::YoleckEditorLevelsDirectoryPath;
/// # let mut app = App::new();
/// app.insert_resource(YoleckEditorLevelsDirectoryPath(
///     Path::new(".").join("some").join("other").join("path"),
/// ));
/// ```
#[derive(Resource)]
pub struct YoleckEditorLevelsDirectoryPath(pub PathBuf);

/// The UI part for managing level files. See [`YoleckEditorSections`](crate::YoleckEditorSections).
pub fn level_files_manager_section(world: &mut World) -> impl FnMut(&mut World, &mut egui::Ui) {
    let mut system_state = SystemState::<(
        Commands,
        ResMut<YoleckState>,
        ResMut<YoleckEditorLevelsDirectoryPath>,
        ResMut<YoleckEditableLevels>,
        Res<YoleckEntityConstructionSpecs>,
        Query<(&YoleckManaged, Option<&YoleckEntityUuid>)>,
        Query<Entity, With<YoleckKeepLevel>>,
        Res<State<YoleckEditorState>>,
        ResMut<NextState<YoleckEditorState>>,
        ResMut<YoleckKnobsCache>,
        ResMut<Assets<YoleckRawLevel>>,
        Option<Res<YoleckEntityUpgrading>>,
        Option<Res<YoleckActiveExclusiveSystem>>,
    )>::new(world);

    let mut should_list_files = true;
    let mut loaded_files_index: io::Result<Vec<YoleckLevelIndexEntry>> = Ok(vec![]);

    #[derive(Debug)]
    enum SelectedLevelFile {
        Unsaved(String),
        Existing(String),
    }

    let mut selected_level_file = SelectedLevelFile::Unsaved(String::new());

    let mut level_being_playtested: Option<YoleckRawLevel> = None;

    move |world, ui: &mut egui::Ui| {
        let (
            mut commands,
            mut yoleck,
            mut levels_directory,
            mut editable_levels,
            construction_specs,
            yoleck_managed_query,
            keep_levels_query,
            editor_state,
            mut next_editor_state,
            mut knobs_cache,
            mut level_assets,
            entity_upgrading,
            active_exclusive_system,
        ) = system_state.get_mut(world);

        if active_exclusive_system.is_some() {
            return;
        }

        let gen_raw_level_file = || {
            let app_format_version = if let Some(entity_upgrading) = &entity_upgrading {
                entity_upgrading.app_format_version
            } else {
                0
            };
            YoleckRawLevel::new(app_format_version, {
                yoleck_managed_query
                    .iter()
                    .map(|(yoleck_managed, entity_uuid)| YoleckRawEntry {
                        header: YoleckEntryHeader {
                            type_name: yoleck_managed.type_name.clone(),
                            name: yoleck_managed.name.clone(),
                            uuid: entity_uuid.map(|entity_uuid| entity_uuid.get()),
                        },
                        data: {
                            if let Some(entity_type_info) =
                                construction_specs.get_entity_type_info(&yoleck_managed.type_name)
                            {
                                entity_type_info
                                    .components
                                    .iter()
                                    .filter_map(|component| {
                                        let component_data =
                                            yoleck_managed.components_data.get(component)?;
                                        let handler =
                                            &construction_specs.component_handlers[component];
                                        Some((
                                            handler.key(),
                                            handler.serialize(component_data.as_ref()),
                                        ))
                                    })
                                    .collect()
                            } else {
                                error!(
                                    "Entity type {:?} is not registered",
                                    yoleck_managed.type_name
                                );
                                Default::default()
                            }
                        },
                    })
            })
        };

        let mut clear_level = |commands: &mut Commands| {
            for level_entity in keep_levels_query.iter() {
                commands.entity(level_entity).despawn();
            }
            for knob_entity in knobs_cache.drain() {
                commands.entity(knob_entity).despawn();
            }
        };

        ui.horizontal(|ui| {
            if let Some(level) = &level_being_playtested {
                let finish_playtest_response = ui.button("Finish Playtest");
                if ui.button("Restart Playtest").clicked() {
                    clear_level(&mut commands);
                    let level_asset_handle = level_assets.add(level.clone());
                    yoleck.level_being_edited = commands
                        .spawn((YoleckLevelInPlaytest, YoleckLoadLevel(level_asset_handle)))
                        .id();
                }
                if finish_playtest_response.clicked() {
                    clear_level(&mut commands);
                    next_editor_state.set(YoleckEditorState::EditorActive);
                    let level_asset_handle = level_assets.add(level.clone());
                    yoleck.level_being_edited = commands
                        .spawn((YoleckLevelInEditor, YoleckLoadLevel(level_asset_handle)))
                        .id();
                    level_being_playtested = None;
                }
            } else {
                #[allow(clippy::collapsible_else_if)]
                if ui.button("Playtest").clicked() {
                    let level = gen_raw_level_file();
                    clear_level(&mut commands);
                    next_editor_state.set(YoleckEditorState::GameActive);
                    let level_asset_handle = level_assets.add(level.clone());
                    yoleck.level_being_edited = commands
                        .spawn((YoleckLevelInPlaytest, YoleckLoadLevel(level_asset_handle)))
                        .id();
                    level_being_playtested = Some(level);
                }
            }
        });

        if matches!(editor_state.get(), YoleckEditorState::EditorActive) {
            egui::CollapsingHeader::new("Files")
                .default_open(true)
                .show(ui, |ui| {
                    let mut path_str = levels_directory.0.to_string_lossy().to_string();
                    ui.horizontal(|ui| {
                        ui.label("Levels Directory:");
                        if ui.text_edit_singleline(&mut path_str).lost_focus() {
                            should_list_files = true;
                        }
                    });
                    levels_directory.0 = path_str.into();

                    let mk_files_index = || levels_directory.0.join("index.yoli");

                    let save_index = |loaded_files_index: &[YoleckLevelIndexEntry]| {
                        let index_file = mk_files_index();
                        match fs::File::create(&index_file) {
                            Ok(fd) => {
                                let index =
                                    YoleckLevelIndex::new(loaded_files_index.iter().cloned());
                                serde_json::to_writer(fd, &index).unwrap();
                            }
                            Err(err) => {
                                warn!("Cannot open {:?} - {}", index_file, err);
                            }
                        }
                    };

                    let save_existing = |filename: &str| -> io::Result<()> {
                        let file_path = levels_directory.0.join(filename);
                        info!("Saving current level to {:?}", file_path);
                        let fd = fs::OpenOptions::new()
                            .write(true)
                            .create(false)
                            .truncate(true)
                            .open(file_path)?;
                        serde_json::to_writer(fd, &gen_raw_level_file())?;
                        Ok(())
                    };

                    if should_list_files {
                        should_list_files = false;

                        let editable_levels_update_result = fs::read_dir(&levels_directory.0)
                            .and_then(|files| {
                                editable_levels.levels = files
                                    .filter_map(|file| {
                                        let file = match file {
                                            Ok(file) => file,
                                            Err(err) => return Some(Err(err)),
                                        };
                                        if file.path().extension()
                                            != Some(std::ffi::OsStr::new(EXTENSION_WITHOUT_DOT))
                                        {
                                            return None;
                                        }
                                        Some(Ok(file.file_name().to_string_lossy().into()))
                                    })
                                    .collect::<Result<_, _>>()?;
                                Ok(())
                            });

                        loaded_files_index = editable_levels_update_result.and_then(|()| {
                            let index_file = mk_files_index();
                            let mut files_index: Vec<YoleckLevelIndexEntry> =
                                match fs::File::open(&index_file) {
                                    Ok(fd) => {
                                        let index: YoleckLevelIndex = serde_json::from_reader(fd)?;
                                        index.iter().cloned().collect()
                                    }
                                    Err(err) => {
                                        warn!("Cannot open {:?} - {}", index_file, err);
                                        Vec::new()
                                    }
                                };
                            let mut existing_files: HashSet<String> = files_index
                                .iter()
                                .map(|file| file.filename.clone())
                                .collect();
                            for filename in editable_levels.names() {
                                if !existing_files.remove(filename) {
                                    files_index.push(YoleckLevelIndexEntry {
                                        filename: filename.to_owned(),
                                    });
                                }
                            }
                            files_index.retain(|file| !existing_files.contains(&file.filename));
                            save_index(&files_index);
                            Ok(files_index)
                        });
                    }
                    match &mut loaded_files_index {
                        Ok(files) => {
                            let mut swap_with_previous = None;
                            egui::ScrollArea::vertical()
                                .max_height(30.0)
                                .show(ui, |ui| {
                                    for (index, file) in files.iter().enumerate() {
                                        let is_selected =
                                            if let SelectedLevelFile::Existing(selected_name) =
                                                &selected_level_file
                                            {
                                                *selected_name == file.filename
                                            } else {
                                                false
                                            };
                                        ui.horizontal(|ui| {
                                            if ui
                                                .add_enabled(0 < index, egui::Button::new("^"))
                                                .clicked()
                                            {
                                                swap_with_previous = Some(index);
                                            }
                                            if ui
                                                .add_enabled(
                                                    index < files.len() - 1,
                                                    egui::Button::new("v"),
                                                )
                                                .clicked()
                                            {
                                                swap_with_previous = Some(index + 1);
                                            }
                                            let yoleck = yoleck.as_mut();
                                            let mut load_level = || {
                                                clear_level(&mut commands);
                                                let fd = fs::File::open(
                                                    levels_directory.0.join(&file.filename),
                                                )
                                                .unwrap();
                                                let level: serde_json::Value =
                                                    serde_json::from_reader(fd).unwrap();
                                                match upgrade_level_file(level) {
                                                    Ok(level) => {
                                                        let level: YoleckRawLevel =
                                                            serde_json::from_value(level).unwrap();
                                                        let level_asset_handle =
                                                            level_assets.add(level);
                                                        yoleck.level_being_edited = commands
                                                            .spawn((
                                                                YoleckLevelInEditor,
                                                                YoleckLoadLevel(level_asset_handle),
                                                            ))
                                                            .id();
                                                    }
                                                    Err(err) => {
                                                        warn!(
                                                            "Cannot upgrade {:?} - {}",
                                                            file.filename, err
                                                        );
                                                    }
                                                }
                                            };
                                            if ui
                                                .selectable_label(is_selected, &file.filename)
                                                .clicked()
                                            {
                                                #[allow(clippy::collapsible_else_if)]
                                                if !is_selected && !yoleck.level_needs_saving {
                                                    selected_level_file =
                                                        SelectedLevelFile::Existing(
                                                            file.filename.clone(),
                                                        );
                                                    load_level();
                                                }
                                            }
                                            if is_selected && yoleck.level_needs_saving {
                                                if ui.button("SAVE").clicked() {
                                                    save_existing(&file.filename).unwrap();
                                                    yoleck.level_needs_saving = false;
                                                }
                                                if ui.button("REVERT").clicked() {
                                                    load_level();
                                                    yoleck.level_needs_saving = false;
                                                }
                                            }
                                        });
                                    }
                                });
                            if let Some(swap_with_previous) = swap_with_previous {
                                files.swap(swap_with_previous, swap_with_previous - 1);
                                save_index(files);
                            }
                            ui.horizontal(|ui| {
                                #[allow(clippy::collapsible_else_if)]
                                match &mut selected_level_file {
                                    SelectedLevelFile::Unsaved(file_name) => {
                                        ui.text_edit_singleline(file_name);
                                        let button = ui.add_enabled(
                                            !file_name.is_empty(),
                                            egui::Button::new("Create"),
                                        );
                                        if button.clicked() {
                                            if !file_name.ends_with(EXTENSION) {
                                                file_name.push_str(EXTENSION);
                                            }
                                            let mut file_path = levels_directory.0.clone();
                                            file_path.push(&file_name);
                                            match fs::OpenOptions::new()
                                                .write(true)
                                                .create_new(true)
                                                .open(&file_path)
                                            {
                                                Ok(fd) => {
                                                    info!(
                                                        "Saving current new level to {:?}",
                                                        file_path
                                                    );
                                                    serde_json::to_writer(
                                                        fd,
                                                        &gen_raw_level_file(),
                                                    )
                                                    .unwrap();
                                                    selected_level_file =
                                                        SelectedLevelFile::Existing(
                                                            file_name.to_owned(),
                                                        );
                                                    should_list_files = true;
                                                    yoleck.level_needs_saving = false;
                                                }
                                                Err(err) => {
                                                    warn!("Cannot open {:?} - {}", file_path, err);
                                                }
                                            }
                                        }
                                        if yoleck.level_needs_saving
                                            && ui.button("Wipe Level").clicked()
                                        {
                                            clear_level(&mut commands);
                                            yoleck.level_needs_saving = false;
                                        }
                                    }
                                    SelectedLevelFile::Existing(_) => {
                                        let button = ui.add_enabled(
                                            !yoleck.level_needs_saving,
                                            egui::Button::new("New Level"),
                                        );
                                        if button.clicked() {
                                            clear_level(&mut commands);
                                            selected_level_file =
                                                SelectedLevelFile::Unsaved(String::new());
                                            yoleck.level_being_edited = commands
                                                .spawn((YoleckLevelInEditor, YoleckKeepLevel))
                                                .id();
                                        }
                                    }
                                }
                            });
                        }
                        Err(err) => {
                            ui.label(format!("Cannot read: {}", err));
                        }
                    }
                });
        }
        system_state.apply(world);
    }
}
