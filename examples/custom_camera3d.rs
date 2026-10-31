use std::path::Path;

use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin};
use bevy_yoleck::YoleckEditMarker;
use bevy_yoleck::prelude::*;
use bevy_yoleck::vpeol::prelude::*;

/// Custom camera mode for isometric view with diagonal movement.
const CAMERA_MODE_ISOMETRIC: Vpeol3dCameraMode = Vpeol3dCameraMode::Custom(0);
/// Custom camera mode for orbital rotation around selected entities.
const CAMERA_MODE_ORBITAL: Vpeol3dCameraMode = Vpeol3dCameraMode::Custom(1);

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    let level = std::env::args().nth(1);
    if let Some(level) = level {
        app.add_plugins(YoleckPluginForGame);
        app.add_plugins(Vpeol3dPluginForGame);
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                commands.spawn(YoleckLoadLevel(
                    asset_server.load(Path::new("levels3d").join(&level)),
                ));
            },
        );
    } else {
        app.add_plugins(EguiPlugin::default());
        app.add_plugins(YoleckPluginForEditor);
        app.add_plugins(Vpeol3dPluginForEditor::topdown());
        app.add_plugins(VpeolSelectionCuePlugin::default());
        app.insert_resource(bevy_yoleck::YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels3d"),
        ));
        #[cfg(target_arch = "wasm32")]
        app.add_systems(
            Startup,
            |asset_server: Res<AssetServer>, mut commands: Commands| {
                commands.spawn(YoleckLoadLevel(asset_server.load("levels3d/example.yol")));
            },
        );

        app.insert_resource(
            YoleckCameraChoices::default()
                .choice_with_transform(
                    "Isometric",
                    {
                        let mut control = Vpeol3dCameraControl::fps();
                        control.mode = CAMERA_MODE_ISOMETRIC;
                        control.allow_rotation_while_maintaining_up = None;
                        control.wasd_movement_speed = 15.0;
                        control
                    },
                    Vec3::new(10.0, 10.0, 10.0),
                    Vec3::ZERO,
                    Vec3::Y,
                )
                .choice_with_transform(
                    "Orbital",
                    {
                        let mut control = Vpeol3dCameraControl::fps();
                        control.mode = CAMERA_MODE_ORBITAL;
                        control.allow_rotation_while_maintaining_up = None;
                        control.wasd_movement_speed = 0.0;
                        control.mouse_sensitivity = 0.005;
                        control
                    },
                    Vec3::new(0.0, 5.0, 15.0),
                    Vec3::ZERO,
                    Vec3::Y,
                ),
        );

        app.add_systems(
            PostUpdate,
            isometric_camera_movement.run_if(in_state(YoleckEditorState::EditorActive)),
        );
        app.add_systems(
            PostUpdate,
            orbital_camera_movement.run_if(in_state(YoleckEditorState::EditorActive)),
        );
    }

    app.add_systems(Startup, (setup_camera, setup_scene));

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Cube")
            .with::<Vpeol3dPosition>()
            .with::<Vpeol3dScale>()
            .apply_scene_on_init(|| {
                bsn! {
                    VpeolWillContainClickableChildren
                    Mesh3d(asset_value(Cuboid::from_size(Vec3::splat(2.0))))
                    MeshMaterial3d<StandardMaterial>(asset_value(Color::from(css::BLUE)))
                }
            })
    });

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Sphere")
            .with::<Vpeol3dPosition>()
            .apply_scene_on_init(|| {
                bsn! {
                    VpeolWillContainClickableChildren
                    Mesh3d(asset_value(Sphere { radius: 1.0 }))
                    MeshMaterial3d<StandardMaterial>(asset_value(Color::from(css::RED)))
                }
            })
    });

    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_scene_list(bsn_list! {
        Camera3d
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y)
        VpeolCameraState
        Vpeol3dCameraControl::topdown()
        --
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
        }
        Transform::from_xyz(5.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y)
    });
}

fn setup_scene(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        Mesh3d(asset_value(Plane3d {
            normal: Dir3::Y,
            half_size: Vec2::new(50.0, 50.0),
        }))
        MeshMaterial3d<StandardMaterial>(asset_value(Color::from(css::DARK_GRAY)))
        Transform::from_xyz(0.0, -1.0, 0.0)
    });
}

fn isometric_camera_movement(
    mut egui_context: EguiContexts,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cameras_query: Query<(&mut Transform, &Vpeol3dCameraControl)>,
) -> Result {
    if egui_context.ctx_mut()?.egui_wants_keyboard_input() {
        return Ok(());
    }

    for (mut camera_transform, camera_control) in cameras_query.iter_mut() {
        if camera_control.mode != CAMERA_MODE_ISOMETRIC {
            continue;
        }

        let mut direction = Vec3::ZERO;

        if keyboard_input.pressed(KeyCode::KeyW) {
            direction += Vec3::new(-1.0, 0.0, -1.0);
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            direction += Vec3::new(1.0, 0.0, 1.0);
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            direction += Vec3::new(-1.0, 0.0, 1.0);
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            direction += Vec3::new(1.0, 0.0, -1.0);
        }
        if keyboard_input.pressed(KeyCode::KeyQ) {
            direction += Vec3::Y;
        }
        if keyboard_input.pressed(KeyCode::KeyE) {
            direction += Vec3::NEG_Y;
        }

        if direction != Vec3::ZERO {
            direction = direction.normalize();
            let speed_multiplier = if keyboard_input.pressed(KeyCode::ShiftLeft) {
                2.0
            } else {
                1.0
            };

            camera_transform.translation += direction
                * camera_control.wasd_movement_speed
                * speed_multiplier
                * time.delta_secs();
        }
    }
    Ok(())
}

fn orbital_camera_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cameras_query: Query<(&mut Transform, &Vpeol3dCameraControl)>,
    selected_entities: Query<&GlobalTransform, With<YoleckEditMarker>>,
) {
    for (mut camera_transform, camera_control) in cameras_query.iter_mut() {
        if camera_control.mode != CAMERA_MODE_ORBITAL {
            continue;
        }

        let look_at = if let Some(selected_transform) = selected_entities.iter().next() {
            selected_transform.translation()
        } else {
            Vec3::ZERO
        };

        let mut orbit_speed = 0.0;
        if keyboard_input.pressed(KeyCode::KeyA) {
            orbit_speed += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            orbit_speed -= 1.0;
        }

        if orbit_speed != 0.0 {
            let rotation = Quat::from_axis_angle(Vec3::Y, orbit_speed * time.delta_secs());
            let offset = camera_transform.translation - look_at;
            camera_transform.translation = look_at + rotation * offset;
            camera_transform.look_at(look_at, Vec3::Y);
        }

        let mut zoom = 0.0;
        if keyboard_input.pressed(KeyCode::KeyW) {
            zoom -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            zoom += 1.0;
        }

        if zoom != 0.0 {
            let direction = (camera_transform.translation - look_at).normalize();
            camera_transform.translation += direction * zoom * 10.0 * time.delta_secs();
        }

        camera_transform.look_at(look_at, Vec3::Y);
    }
}
