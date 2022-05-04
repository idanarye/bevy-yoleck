use std::path::PathBuf;
use std::{fs, io};

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::{YoleckEntryHeader, YoleckManaged, YoleckRawEntry, YoleckTypeHandlers};

pub struct YoleckEditorLevelsDirectoryPath(pub PathBuf);

pub fn level_files_manager_section(world: &mut World) -> impl FnMut(&mut World, &mut egui::Ui) {
    let mut system_state = SystemState::<(
        ResMut<YoleckEditorLevelsDirectoryPath>,
        Res<YoleckTypeHandlers>,
        Query<(Entity, &YoleckManaged)>,
    )>::new(world);

    let mut should_list_files = true;
    let mut loaded_files: io::Result<Vec<PathBuf>> = Ok(vec![]);

    let mut create_new_level_file: Option<String> = None;

    move |world, ui: &mut egui::Ui| {
        let (mut levels_directory, yoleck_type_handlers, yoleck_managed_query) =
            system_state.get_mut(world);
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
                                    if let Some(file) = file.to_str() {
                                        ui.label(file);
                                    }
                                }
                            });
                        ui.horizontal(|ui| {
                            #[allow(clippy::collapsible_else_if)]
                            if let Some(file_name) = &mut create_new_level_file {
                                ui.text_edit_singleline(file_name);
                                if !file_name.is_empty() {
                                    #[allow(clippy::collapsible_else_if)]
                                    if ui.button("Create").clicked() {
                                        if !file_name.ends_with(".yoleck") {
                                            file_name.push_str(".yoleck");
                                        }
                                        info!("Creating {}", file_name);
                                        create_new_level_file = None
                                    }
                                }
                            } else {
                                if ui.button("New File").clicked() {
                                    create_new_level_file = Some(String::new());
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
}
