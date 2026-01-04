use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContext, PrimaryEguiContext};

use crate::util::EditSpecificResources;
use crate::YoleckEditorRightPanelSections;
use crate::YoleckEditorLeftPanelSections;
use crate::YoleckEditorTopPanelSections;
use crate::YoleckEditorBottomPanelSections;

#[derive(Resource, Default)]
pub struct YoleckEditorViewportRect {
    pub rect: Option<egui::Rect>,
}

pub(crate) fn yoleck_editor_window(
    world: &mut World,
    mut egui_query: Local<Option<QueryState<&mut EguiContext, With<PrimaryEguiContext>>>>,
) {
    let egui_query = egui_query.get_or_insert_with(|| world.query_filtered());
    let mut borrowed_egui = if let Ok(mut egui_context) = egui_query.single_mut(world) {
        core::mem::take(egui_context.as_mut())
    } else {
        return;
    };

    let ctx = borrowed_egui.get_mut();

    let left = egui::SidePanel::left("yoleck_left_panel")
        .resizable(true)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.heading("Level Hierarchy");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                world.resource_scope(
                    |world, mut yoleck_editor_sections: Mut<YoleckEditorLeftPanelSections>| {
                        world.resource_scope(
                            |world, mut edit_specific: Mut<EditSpecificResources>| {
                                edit_specific.inject_to_world(world);
                                for section in yoleck_editor_sections.0.iter_mut() {
                                    section.0.invoke(world, ui).unwrap();
                                }
                                edit_specific.take_from_world(world);
                            },
                        );
                    },
                );
            });
        })
        .response
        .rect
        .width();

    let right = egui::SidePanel::right("yoleck_right_panel")
        .resizable(true)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.heading("Properties");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                world.resource_scope(
                    |world, mut yoleck_editor_right_sections: Mut<YoleckEditorRightPanelSections>| {
                        world.resource_scope(|world, mut edit_specific: Mut<EditSpecificResources>| {
                            edit_specific.inject_to_world(world);
                            for section in yoleck_editor_right_sections.0.iter_mut() {
                                section.0.invoke(world, ui).unwrap();
                            }
                            edit_specific.take_from_world(world);
                        });
                    },
                );
            });
        })
        .response
        .rect
        .width();

    let top = egui::TopBottomPanel::top("yoleck_top_panel")
        .resizable(false)
        .show(ctx, |ui| {
            let inner_margin = 3.;

            ui.add_space(inner_margin);
            ui.horizontal(|ui| {
                ui.add_space(inner_margin);
                ui.label("Yoleck Editor");
                ui.separator();

                world.resource_scope(
                    |world, mut yoleck_editor_top_sections: Mut<YoleckEditorTopPanelSections>| {
                        world.resource_scope(
                            |world, mut edit_specific: Mut<EditSpecificResources>| {
                                edit_specific.inject_to_world(world);
                                for section in yoleck_editor_top_sections.0.iter_mut() {
                                    section.0.invoke(world, ui).unwrap();
                                }
                                edit_specific.take_from_world(world);
                            },
                        );
                    },
                );
                ui.add_space(inner_margin);
            });
            ui.add_space(inner_margin);
        })
        .response
        .rect
        .height();

    let bottom = egui::TopBottomPanel::bottom("yoleck_bottom_panel")
        .resizable(true)
        .default_height(200.0)
        .show(ctx, |ui| {
            world.resource_scope(
                |world, mut yoleck_editor_bottom_sections: Mut<YoleckEditorBottomPanelSections>| {
                    world.resource_scope(|world, mut edit_specific: Mut<EditSpecificResources>| {
                        edit_specific.inject_to_world(world);
                        
                        let inner_margin = 3.;
                        ui.add_space(inner_margin);
                        
                        let mut new_active_tab = yoleck_editor_bottom_sections.active_tab;
                        ui.horizontal(|ui| {
                            for (i, tab) in yoleck_editor_bottom_sections.tabs.iter().enumerate() {
                                if ui.selectable_label(
                                    new_active_tab == i,
                                    &tab.name
                                ).clicked() {
                                    new_active_tab = i;
                                }
                            }
                        });
                        yoleck_editor_bottom_sections.active_tab = new_active_tab;
                        
                        ui.separator();
                        
                        let active_tab = yoleck_editor_bottom_sections.active_tab;
                        if let Some(tab) = yoleck_editor_bottom_sections.tabs.get_mut(active_tab) {
                            tab.section.0.invoke(world, ui).unwrap();
                        }
                        
                        edit_specific.take_from_world(world);
                    });
                },
            );
        })
        .response
        .rect
        .height();

    let viewport_rect = egui::Rect::from_min_max(
        egui::Pos2::new(left, top),
        egui::Pos2::new(
            ctx.input(|i| i.viewport_rect().width()) - right,
            ctx.input(|i| i.viewport_rect().height()) - bottom,
        ),
    );

    if let Some(mut editor_viewport) = world.get_resource_mut::<YoleckEditorViewportRect>() {
        editor_viewport.rect = Some(viewport_rect);
    }

    if let Ok(window) = world
        .query_filtered::<&bevy::window::Window, With<PrimaryWindow>>()
        .single(world)
    {
        let scale = window.scale_factor();

        let left_px = (left * scale) as u32;
        let right_px = (right * scale) as u32;
        let top_px = (top * scale) as u32;
        let bottom_px = (bottom * scale) as u32;

        let pos = UVec2::new(left_px, top_px);
        let size = UVec2::new(window.physical_width(), window.physical_height())
            .saturating_sub(pos)
            .saturating_sub(UVec2::new(right_px, bottom_px));

        if size.x > 0 && size.y > 0 {
            let mut camera_query =
                world.query_filtered::<&mut Camera, Without<PrimaryEguiContext>>();
            for mut camera in camera_query.iter_mut(world) {
                camera.viewport = Some(bevy::camera::Viewport {
                    physical_position: pos,
                    physical_size: size,
                    ..default()
                });
            }
        }
    }

    if let Ok(mut egui_context) = egui_query.single_mut(world) {
        *egui_context = borrowed_egui;
    }
}

#[allow(clippy::type_complexity)]
pub(crate) enum YoleckEditorSectionInner {
    Uninitialized(
        Box<
            dyn 'static
                + Send
                + Sync
                + FnOnce(
                    &mut World,
                ) -> Box<
                    dyn 'static + Send + Sync + FnMut(&mut World, &mut egui::Ui) -> Result,
                >,
        >,
    ),
    Middle,
    Initialized(Box<dyn 'static + Send + Sync + FnMut(&mut World, &mut egui::Ui) -> Result>),
}

impl YoleckEditorSectionInner {
    pub(crate) fn invoke(&mut self, world: &mut World, ui: &mut egui::Ui) -> Result {
        match self {
            Self::Uninitialized(_) => {
                if let Self::Uninitialized(system_constructor) =
                    core::mem::replace(self, Self::Middle)
                {
                    let mut system = system_constructor(world);
                    system(world, ui)?;
                    *self = Self::Initialized(system);
                } else {
                    panic!("It was just Uninitialized...");
                }
            }
            Self::Middle => panic!("Cannot start in the middle state when being invoked"),
            Self::Initialized(system) => {
                system(world, ui)?;
            }
        }
        Ok(())
    }
}

/// A single section of the UI. See [`YoleckEditorLeftPanelSections`](crate::YoleckEditorLeftPanelSections).
pub struct YoleckEditorSection(pub(crate) YoleckEditorSectionInner);

impl<C, S> From<C> for YoleckEditorSection
where
    C: 'static + Send + Sync + FnOnce(&mut World) -> S,
    S: 'static + Send + Sync + FnMut(&mut World, &mut egui::Ui) -> Result,
{
    fn from(system_constructor: C) -> Self {
        Self(YoleckEditorSectionInner::Uninitialized(Box::new(
            move |world| Box::new(system_constructor(world)),
        )))
    }
}
