use crate::{
    bevy_egui::egui,
    vpeol::{prelude::Vpeol3dPosition, VpeolCameraState},
    vpeol_3d::Editor3dResource,
    YoleckExtForApp,
};
use bevy::{input::mouse::MouseMotion, prelude::*};

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

#[derive(Event)]
struct VpeolWorldCursorPointer {
    position: Vec3,
    delta: Vec3,
}

pub struct Vpeol3dRotationEdit;

impl Plugin for Vpeol3dRotationEdit {
    fn build(&self, app: &mut App) {
        app.add_event::<VpeolWorldCursorPointer>();
        app.insert_resource(VpeolRot(Vec3::ZERO));
        app.insert_resource(Is3dRotationEditing(false));
        app.insert_resource(VpeolLastWorldCursorPosition(None));

        app.add_yoleck_edit_system(test);
        app.add_yoleck_edit_system(rotation_gizmos);
        app.add_yoleck_edit_system(edit_rotation_by_ui);
        app.add_yoleck_edit_system(edit_rotaion_by_editor);
    }
}

fn test(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut last_cursor_pos: ResMut<VpeolLastWorldCursorPosition>,
    mut cursor_moved_events: EventReader<MouseMotion>,
    mut query: Query<(&Camera, &VpeolCameraState)>,
    entity_position: YoleckEdit<&Vpeol3dPosition>,
    mut world_cursor_event: EventWriter<VpeolWorldCursorPointer>,
) {
    if !mouse_input.pressed(MouseButton::Left) {
        last_cursor_pos.0 = None;
        return;
    }
    let Ok((_, camera_state)) = query.get_single_mut() else {
        return;
    };
    let Some(cursor_ray) = camera_state.cursor_ray else {
        return;
    };
    let Ok(entity_position) = entity_position.get_single() else {
        return;
    };
    for _ in cursor_moved_events.read() {
        let entity_distance = cursor_ray.origin.distance(entity_position.0);
        let plane_point = cursor_ray.get_point(entity_distance);
        let distance = cursor_ray
            .intersect_plane(plane_point, Plane3d::new(cursor_ray.direction.normalize()))
            .unwrap_or(0.0);
        match last_cursor_pos.0 {
            Some(last_cursor_pos) => {
                world_cursor_event.send(VpeolWorldCursorPointer {
                    position: cursor_ray.get_point(distance),
                    delta: cursor_ray.get_point(distance) - last_cursor_pos,
                });
            }
            None => {
                world_cursor_event.send(VpeolWorldCursorPointer {
                    position: cursor_ray.get_point(distance),
                    delta: Vec3::ZERO,
                });
            }
        }
        last_cursor_pos.0 = Some(cursor_ray.get_point(distance));
        //println!("{:?} distance:{distance:?}", cursor_ray);
    }
}

fn rotation_gizmos(
    mut gizmos: Gizmos,
    edit: YoleckEdit<&Vpeol3dPosition>,
    editor_config: Res<Editor3dResource>,
) {
    if let Ok(position) = edit.get_single() {
        if editor_config.is_rotation_editing {
            gizmos.circle(position.0, Direction3d::Y, 2.1, Color::GREEN);
            gizmos.circle(position.0, Direction3d::X, 2.1, Color::RED);
            gizmos.circle(position.0, Direction3d::Z, 2.1, Color::BLUE);
        }
    }
}

fn edit_rotaion_by_editor(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut cursor_moved_events: EventReader<MouseMotion>,
    mut world_cursor_moved_events: EventReader<VpeolWorldCursorPointer>,
    mut rot: ResMut<VpeolRot>,
    camera_query: Query<&mut Transform, With<Camera>>,
    editor_config: Res<Editor3dResource>,
) {
    if mouse_input.pressed(MouseButton::Left) && editor_config.is_rotation_editing {
        //if let Ok(transform) = camera_query.get_single() {
        //    println!("{:?}", transform);
        //}

        //for event in cursor_moved_events.read() {
        //    rot.0.y += event.delta.x * 0.01;
        //    rot.0.x += event.delta.y * 0.01;
        //    rot.0.z += 0.0;
        //}
        for event in world_cursor_moved_events.read() {
            println!("{:?}", event.delta);
            //rot.0 = Quat::from_scaled_axis(event.delta.normalize()).xyz();
            rot.0.y += event.delta.y * 0.5;
            rot.0.x += event.delta.x * 0.5;
            rot.0.z += event.delta.z * 0.5;
        }
    }
}

fn edit_rotation_by_ui(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<&mut Vpeol3dRotation>,
    mut rot: ResMut<VpeolRot>,
    mut editor_config: ResMut<Editor3dResource>,
) {
    if let Ok(mut rotation) = edit.get_single_mut() {
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

        let quat_x = Quat::from_rotation_x(x);
        let quat_y = Quat::from_rotation_y(y);
        let quat_z = Quat::from_rotation_z(z);
        rotation.0 = (quat_z * quat_y * quat_x).normalize();
        //rotation.0 = Quat::from_euler(EulerRot::XYZ, x, y, z);
    }
}
