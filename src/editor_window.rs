use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContext, PrimaryEguiContext, egui};

use crate::YoleckEditorBottomPanelSections;
use crate::YoleckEditorLeftPanelSections;
use crate::YoleckEditorRightPanelSections;
use crate::YoleckEditorTopPanelSections;
use crate::editor_panels::EditorPanel;

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

    let left = YoleckEditorLeftPanelSections::show_panel(world, ctx)
        .rect
        .width();

    let right = YoleckEditorRightPanelSections::show_panel(world, ctx)
        .rect
        .width();

    let top = YoleckEditorTopPanelSections::show_panel(world, ctx)
        .rect
        .height();

    let bottom = YoleckEditorBottomPanelSections::show_panel(world, ctx)
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
