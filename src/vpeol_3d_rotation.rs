use bevy::{input::mouse::MouseMotion, prelude::*};

use crate::{bevy_egui::egui, vpeol_3d::Editor3dResource, YoleckExtForApp};
use crate::{
    prelude::{YoleckEdit, YoleckUi},
    vpeol::prelude::Vpeol3dRotation,
};

#[derive(Resource)]
pub struct VpeolRot(pub Vec3);

#[derive(Resource)]
pub struct Is3dRotationEditing(pub bool);

#[derive(Resource)]
pub struct VpeolLastWorldCursorPosition(pub Option<Vec3>);

pub struct Vpeol3dRotationEdit;

impl Plugin for Vpeol3dRotationEdit {
    fn build(&self, app: &mut App) {
        app.insert_resource(VpeolRot(Vec3::ZERO));
        app.insert_resource(Is3dRotationEditing(false));
        app.insert_resource(VpeolLastWorldCursorPosition(None));

        app.add_yoleck_edit_system(edit_rotation_by_ui);
        app.add_yoleck_edit_system(edit_rotaion_by_editor);
    }
}

fn edit_rotaion_by_editor(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut cursor_moved_events: EventReader<MouseMotion>,
    camera_query: Query<&mut Transform, With<Camera>>,
    editor_config: Res<Editor3dResource>,
    edit: YoleckEdit<&Vpeol3dRotation>,
    mut rot: ResMut<VpeolRot>,
) {
    let Ok(rotation) = edit.get_single() else {
        return;
    };
    if mouse_input.pressed(MouseButton::Left) && editor_config.is_rotation_editing {
        let Ok(camera_transform) = camera_query.get_single() else {
            return;
        };

        for event in cursor_moved_events.read() {
            let normal = camera_transform.rotation * Vec3::Y;
            let yaw = Quat::from_axis_angle(normal.normalize(), event.delta.x * 0.01);
            let normal = camera_transform.rotation * Vec3::X;
            let pitch = Quat::from_axis_angle(normal.normalize(), event.delta.y * 0.01);
            let (x, y, z) = (yaw * pitch * rotation.0).to_euler(EulerRot::XYZ);
            rot.0 = Vec3::new(x, y, z);
        }
    }
}

fn edit_rotation_by_ui(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<&mut Vpeol3dRotation>,
    mut rot: ResMut<VpeolRot>,
    mut editor_config: ResMut<Editor3dResource>,
) {
    let Ok(mut rotation) = edit.get_single_mut() else {
        return;
    };
    let (mut x, mut y, mut z) = (rot.0.x, rot.0.y, rot.0.z);

    ui.horizontal(|ui| {
        ui.add(egui::Label::new("Rotation:"));
        ui.add(egui::Checkbox::new(
            &mut editor_config.is_rotation_editing,
            "Enable khobs for rotation edit",
        ));
    });
    ui.vertical(|ui| {
        ui.add(egui::DragValue::new(&mut x).prefix("X:").speed(0.01));
        ui.add(egui::DragValue::new(&mut y).prefix("Y:").speed(0.01));
        ui.add(egui::DragValue::new(&mut z).prefix("Z:").speed(0.01));
    });

    rot.0.x = x;
    rot.0.y = y;
    rot.0.z = z;

    rotation.0 = Quat::from_euler(EulerRot::XYZ, x, y, z);
}
