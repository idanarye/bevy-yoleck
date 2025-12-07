use std::path::Path;

use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin};
use bevy_yoleck::prelude::*;
use bevy_yoleck::vpeol::prelude::*;
use bevy_yoleck::YoleckEditMarker;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    let level = std::env::args().nth(1);
    if let Some(level) = level {
        app.add_plugins(EguiPlugin::default());
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
                    || {
                        let mut control = Vpeol3dCameraControl::fps();
                        control.mode_name = "Isometric".to_string();
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
                    || {
                        let mut control = Vpeol3dCameraControl::fps();
                        control.mode_name = "Orbital".to_string();
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
            .insert_on_init(|| IsCube)
    });
    app.add_systems(YoleckSchedule::Populate, populate_cube);

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Sphere")
            .with::<Vpeol3dPosition>()
            .insert_on_init(|| IsSphere)
    });
    app.add_systems(YoleckSchedule::Populate, populate_sphere);

    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        VpeolCameraState::default(),
        Vpeol3dCameraControl::topdown(),
    ));

    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 10_000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::from_xyz(5.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn setup_scene(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = mesh_assets.add(Mesh::from(
        Plane3d {
            normal: Dir3::Y,
            half_size: Vec2::new(50.0, 50.0),
        }
        .mesh(),
    ));
    let material = material_assets.add(Color::from(css::DARK_GRAY));
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, -1.0, 0.0),
    ));
}

fn isometric_camera_movement(
    mut egui_context: EguiContexts,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cameras_query: Query<(&mut Transform, &Vpeol3dCameraControl)>,
) -> Result {
    if egui_context.ctx_mut()?.wants_keyboard_input() {
        return Ok(());
    }

    for (mut camera_transform, camera_control) in cameras_query.iter_mut() {
        if camera_control.mode_name != "Isometric" {
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
        if camera_control.mode_name != "Orbital" {
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

#[derive(Component)]
struct IsCube;

fn populate_cube(
    mut populate: YoleckPopulate<(), With<IsCube>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut mesh: Local<Option<Handle<Mesh>>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut material: Local<Option<Handle<StandardMaterial>>>,
) {
    populate.populate(|ctx, mut cmd, ()| {
        cmd.insert(VpeolWillContainClickableChildren);
        if ctx.is_first_time() {
            let mesh = mesh
                .get_or_insert_with(|| mesh_assets.add(Mesh::from(Cuboid::from_size(Vec3::splat(2.0)))))
                .clone();
            let material = material
                .get_or_insert_with(|| material_assets.add(Color::from(css::BLUE)))
                .clone();
            cmd.insert((Mesh3d(mesh), MeshMaterial3d(material)));
        }
    });
}

#[derive(Component)]
struct IsSphere;

fn populate_sphere(
    mut populate: YoleckPopulate<(), With<IsSphere>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut mesh: Local<Option<Handle<Mesh>>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut material: Local<Option<Handle<StandardMaterial>>>,
) {
    populate.populate(|ctx, mut cmd, ()| {
        cmd.insert(VpeolWillContainClickableChildren);
        if ctx.is_first_time() {
            let mesh = mesh
                .get_or_insert_with(|| mesh_assets.add(Mesh::from(Sphere { radius: 1.0 })))
                .clone();
            let material = material
                .get_or_insert_with(|| material_assets.add(Color::from(css::RED)))
                .clone();
            cmd.insert((Mesh3d(mesh), MeshMaterial3d(material)));
        }
    });
}

