use std::path::PathBuf;
use std::{fs, io};

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::{YoleckEntryHeader, YoleckManaged, YoleckRawEntry, YoleckTypeHandlers};

const EXTENSION: &str = ".yol";
const EXTENSION_WITHOUT_DOT: &str = "yol";

pub struct YoleckEditorLevelsDirectoryPath(pub PathBuf);

pub fn level_files_manager_section(world: &mut World) -> impl FnMut(&mut World, &mut egui::Ui) {
    let mut system_state = SystemState::<(
        Commands,
        ResMut<YoleckEditorLevelsDirectoryPath>,
        Res<YoleckTypeHandlers>,
        Query<(Entity, &YoleckManaged)>,
    )>::new(world);

    let mut should_list_files = true;
    let mut loaded_files: io::Result<Vec<PathBuf>> = Ok(vec![]);

    #[derive(Debug)]
    enum SelectedLevelFile {
        Unsaved(String),
        Existing(String),
    }

    let mut selected_level_file = SelectedLevelFile::Unsaved(String::new());

    move |world, ui: &mut egui::Ui| {
        let (mut commands, mut levels_directory, yoleck_type_handlers, yoleck_managed_query) =
            system_state.get_mut(world);

        let gen_raw_entries = || {
            yoleck_managed_query
                .iter()
                .map(|(_entity, yoleck_managed)| {
                    let handler = yoleck_type_handlers
                        .type_handlers
                        .get(&yoleck_managed.type_name)
                        .unwrap();
                    YoleckRawEntry {
                        header: YoleckEntryHeader {
                            type_name: yoleck_managed.type_name.clone(),
                            name: yoleck_managed.name.clone(),
                        },
                        data: handler.make_raw(&yoleck_managed.data),
                    }
                })
                .collect::<Vec<_>>()
        };

        let clear_level = |commands: &mut Commands| {
            for (entity, _) in yoleck_managed_query.iter() {
                commands.entity(entity).despawn_recursive();
            }
        };

        if ui.button("Save Level").clicked() {
            for (entity, yoleck_managed) in yoleck_managed_query.iter() {
                let handler = yoleck_type_handlers
                    .type_handlers
                    .get(&yoleck_managed.type_name)
                    .unwrap();
                println!(
                    "{:?}: {}",
                    entity,
                    serde_json::to_string(&YoleckRawEntry {
                        header: YoleckEntryHeader {
                            type_name: yoleck_managed.type_name.clone(),
                            name: yoleck_managed.name.clone(),
                        },
                        data: handler.make_raw(&yoleck_managed.data),
                    })
                    .unwrap()
                );
            }
        }
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

                let save_existing = |filename: &str| -> io::Result<()> {
                    let file_path = levels_directory.0.join(filename);
                    info!("Saving current level to {:?}", file_path);
                    let fd = fs::OpenOptions::new()
                        .write(true)
                        .create(false)
                        .truncate(true)
                        .open(file_path)?;
                    serde_json::to_writer(fd, &gen_raw_entries())?;
                    Ok(())
                };

                if should_list_files {
                    should_list_files = false;
                    loaded_files = fs::read_dir(&levels_directory.0).and_then(|files| {
                        let mut result = Vec::new();
                        for file in files {
                            result.push(file?.path());
                        }
                        Ok(result)
                    });
                }
                match &loaded_files {
                    Ok(files) => {
                        egui::ScrollArea::vertical()
                            .max_height(30.0)
                            .show(ui, |ui| {
                                for file in files {
                                    if file.extension()
                                        != Some(std::ffi::OsStr::new(EXTENSION_WITHOUT_DOT))
                                    {
                                        continue;
                                    }
                                    if let Some(file_name) =
                                        file.file_name().and_then(|n| n.to_str())
                                    {
                                        let is_selected =
                                            if let SelectedLevelFile::Existing(selected_name) =
                                                &selected_level_file
                                            {
                                                selected_name == file_name
                                            } else {
                                                false
                                            };
                                        if ui.selectable_label(is_selected, file_name).clicked() {
                                            if is_selected {
                                                save_existing(file_name).unwrap();
                                            } else {
                                                match &selected_level_file {
                                                    SelectedLevelFile::Unsaved(_) => {
                                                        if yoleck_managed_query.is_empty() {
                                                            selected_level_file =
                                                                SelectedLevelFile::Existing(
                                                                    file_name.to_owned(),
                                                                );
                                                        } else {
                                                            warn!("You have some unsaved file");
                                                            continue;
                                                        }
                                                    }
                                                    SelectedLevelFile::Existing(current_file) => {
                                                        save_existing(current_file).unwrap();
                                                        clear_level(&mut commands);
                                                        selected_level_file =
                                                            SelectedLevelFile::Existing(
                                                                file_name.to_owned(),
                                                            );
                                                    }
                                                }
                                                let fd = fs::File::open(
                                                    levels_directory.0.join(file_name),
                                                )
                                                .unwrap();
                                                let data: Vec<YoleckRawEntry> =
                                                    serde_json::from_reader(fd).unwrap();
                                                for entry in data.into_iter() {
                                                    commands.spawn().insert(entry);
                                                }
                                            }
                                        }
                                    }
                                }
                            });
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
                                                serde_json::to_writer(fd, &gen_raw_entries())
                                                    .unwrap();
                                                selected_level_file = SelectedLevelFile::Existing(
                                                    file_name.to_owned(),
                                                );
                                                should_list_files = true;
                                            }
                                            Err(err) => {
                                                warn!("Cannot open {:?} - {}", file_path, err);
                                            }
                                        }
                                    }
                                }
                                SelectedLevelFile::Existing(current_file) => {
                                    if ui.button("New Level").clicked() {
                                        save_existing(current_file).unwrap();
                                        clear_level(&mut commands);
                                        selected_level_file =
                                            SelectedLevelFile::Unsaved(String::new());
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
        system_state.apply(world);
    }
}
