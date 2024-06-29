use crate::{
    bevy_egui::egui, vpeol::prelude::Vpeol3dScale, vpeol_3d::Editor3dResource, YoleckExtForApp,
};
use bevy::prelude::*;

use crate::prelude::{YoleckEdit, YoleckUi};

pub struct Vpeol3dScaleEdit;

impl Plugin for Vpeol3dScaleEdit {
    fn build(&self, app: &mut App) {
        app.add_yoleck_edit_system(vpeol_3d_edit_scale);
    }
}

fn vpeol_3d_edit_scale(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<&mut Vpeol3dScale>,
    mut editor_config: ResMut<Editor3dResource>,
) {
    let Ok(mut scale) = edit.get_single_mut() else {
        return;
    };
    ui.horizontal(|ui| {
        ui.add(egui::Label::new("Scale:"));
        ui.add(egui::Checkbox::new(
            &mut editor_config.is_sync_scale_axis,
            "Sync scale axis",
        ));
    });

    if editor_config.is_sync_scale_axis {
        ui.vertical(|ui| {
            ui.add(
                egui::DragValue::new(&mut scale.0.x)
                    .prefix("Scale value:")
                    .speed(0.1)
                    .clamp_range(0..=i32::MAX),
            );
        });
        scale.0 = Vec3::splat(scale.0.x);
        return;
    }

    ui.vertical(|ui| {
        ui.add(
            egui::DragValue::new(&mut scale.0.x)
                .prefix("X:")
                .speed(0.1)
                .clamp_range(0..=i32::MAX),
        );
        ui.add(
            egui::DragValue::new(&mut scale.0.y)
                .prefix("Y:")
                .speed(0.1)
                .clamp_range(0..=i32::MAX),
        );
        ui.add(
            egui::DragValue::new(&mut scale.0.z)
                .prefix("Z:")
                .speed(0.1)
                .clamp_range(0..=i32::MAX),
        );
    });
}
