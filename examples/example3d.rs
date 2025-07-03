use std::path::Path;

use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use uuid::Uuid;

use bevy_yoleck::exclusive_systems::{YoleckExclusiveSystemDirective, YoleckExclusiveSystemsQueue};
use bevy_yoleck::vpeol::{prelude::*, vpeol_read_click_on_entity};
use bevy_yoleck::{prelude::*, yoleck_exclusive_system_cancellable, yoleck_map_entity_to_uuid};
use serde::{Deserialize, Serialize};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    let level = std::env::args().nth(1);
    if let Some(level) = level {
        // The egui plugin is not needed for the game itself, but GameAssets won't load without it
        // because it needs `EguiContexts` which cannot be `Option` because it's a custom
        // `SystemParam`.
        app.add_plugins(EguiPlugin::default());

        app.add_plugins(YoleckPluginForGame);
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                commands.spawn(YoleckLoadLevel(
                    asset_server.load(Path::new("levels3d").join(&level)),
                ));
            },
        );
        app.add_plugins(Vpeol3dPluginForGame);
    } else {
        app.add_plugins(EguiPlugin::default());

        app.add_plugins(YoleckPluginForEditor);
        // Adding `YoleckEditorLevelsDirectoryPath` is not usually required -
        // `YoleckPluginForEditor` will add one with "assets/levels". Here we want to support
        // example2d and example3d in the same repository so we use different directories.
        app.insert_resource(bevy_yoleck::YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels3d"),
        ));
        app.add_plugins(Vpeol3dPluginForEditor::topdown());
        app.add_plugins(VpeolSelectionCuePlugin::default());
        #[cfg(target_arch = "wasm32")]
        app.add_systems(
            Startup,
            |asset_server: Res<AssetServer>, mut commands: Commands| {
                commands.spawn(YoleckLoadLevel(asset_server.load("levels3d/example.yol")));
            },
        );
    }
    app.add_systems(Startup, setup_camera);
    app.add_systems(Startup, setup_arena);

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Spaceship")
            .with::<Vpeol3dPosition>()
            .insert_on_init(|| IsSpaceship)
            .insert_on_init_during_editor(|| Vpeol3dThirdAxisWithKnob {
                knob_distance: 2.0,
                knob_scale: 0.5,
            })
    });
    app.add_systems(YoleckSchedule::Populate, populate_spaceship);

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Planet")
            .with_uuid()
            .with::<Vpeol3dPosition>()
            .insert_on_init(|| IsPlanet)
            .insert_on_init_during_editor(|| VpeolDragPlane::XY)
            .insert_on_init_during_editor(|| Vpeol3dThirdAxisWithKnob {
                knob_distance: 2.0,
                knob_scale: 0.5,
            })
    });
    app.add_systems(YoleckSchedule::Populate, populate_planet);

    app.add_yoleck_entity_type({
        YoleckEntityType::new("PlanetPointer")
            .with::<Vpeol3dPosition>()
            .with::<LaserPointer>()
            .insert_on_init(|| SimpleSphere)
            .insert_on_init_during_editor(|| Vpeol3dThirdAxisWithKnob {
                knob_distance: 2.0,
                knob_scale: 0.5,
            })
    });

    app.add_systems(YoleckSchedule::Populate, populate_simple_sphere);

    app.add_yoleck_edit_system(edit_laser_pointer);
    app.add_systems(Update, draw_laser_pointers);

    app.add_systems(
        Update,
        (control_spaceship, hit_planets).run_if(in_state(YoleckEditorState::GameActive)),
    );

    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 16.0, 40.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ))
        .insert(VpeolCameraState::default())
        .insert(Vpeol3dCameraControl::topdown());

    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 50_000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::from_xyz(0.0, 100.0, 0.0).looking_to(-Vec3::Y, Vec3::Z),
    ));
}

fn setup_arena(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = mesh_assets.add(Mesh::from(
        Plane3d {
            normal: Dir3::Y,
            half_size: Vec2::new(100.0, 100.0),
        }
        .mesh(),
    ));
    let material = material_assets.add(Color::from(css::GRAY));
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, -10.0, 0.0),
    ));
}

#[derive(Component)]
struct IsSpaceship;

fn populate_spaceship(
    mut populate: YoleckPopulate<(), With<IsSpaceship>>,
    asset_server: Res<AssetServer>,
) {
    populate.populate(|ctx, mut cmd, ()| {
        cmd.insert(VpeolWillContainClickableChildren);
        // Spaceship model doesn't change, so there is no need to despawn and recreated it.
        if ctx.is_first_time() {
            cmd.insert(SceneRoot(asset_server.load("models/spaceship.glb#Scene0")));
        }
    });
}

#[derive(Component)]
struct IsPlanet;

fn populate_planet(
    mut populate: YoleckPopulate<(), With<IsPlanet>>,
    asset_server: Res<AssetServer>,
) {
    populate.populate(|ctx, mut cmd, ()| {
        cmd.insert(VpeolWillContainClickableChildren);
        // Planet model doesn't change, so there is no need to despawn and recreated it.
        if ctx.is_first_time() {
            cmd.insert(SceneRoot(asset_server.load("models/planet.glb#Scene0")));
        }
    });
}

fn control_spaceship(
    mut spaceship_query: Query<&mut Transform, With<IsSpaceship>>,
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
) {
    let calc_axis = |neg: KeyCode, pos: KeyCode| match (input.pressed(neg), input.pressed(pos)) {
        (true, true) | (false, false) => 0.0,
        (true, false) => -1.0,
        (false, true) => 1.0,
    };
    let pitch = calc_axis(KeyCode::ArrowUp, KeyCode::ArrowDown);
    let roll = calc_axis(KeyCode::ArrowLeft, KeyCode::ArrowRight);
    for mut spaceship_transform in spaceship_query.iter_mut() {
        let forward_direction = spaceship_transform.rotation.mul_vec3(-Vec3::Z);
        let roll_quat = Quat::from_scaled_axis(2.0 * forward_direction * time.delta_secs() * roll);
        let pitch_axis = spaceship_transform.rotation.mul_vec3(Vec3::X);
        let pitch_quat = Quat::from_scaled_axis(2.0 * pitch_axis * time.delta_secs() * pitch);
        spaceship_transform.rotation = roll_quat * pitch_quat * spaceship_transform.rotation;
        spaceship_transform.translation += 2.0 * forward_direction * time.delta_secs();
    }
}

fn hit_planets(
    spaceship_query: Query<&Transform, With<IsSpaceship>>,
    planets_query: Query<(Entity, &Transform), With<IsPlanet>>,
    mut commands: Commands,
) {
    for spaceship_transform in spaceship_query.iter() {
        for (planet_entity, planet_transform) in planets_query.iter() {
            if spaceship_transform
                .translation
                .distance_squared(planet_transform.translation)
                < 2.0f32.powi(2)
            {
                commands.entity(planet_entity).despawn();
            }
        }
    }
}

#[derive(Component)]
struct SimpleSphere;

fn populate_simple_sphere(
    mut populate: YoleckPopulate<(), With<SimpleSphere>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut mesh: Local<Option<Handle<Mesh>>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut material: Local<Option<Handle<StandardMaterial>>>,
) {
    populate.populate(|ctx, mut cmd, ()| {
        if ctx.is_first_time() {
            let mesh = mesh
                .get_or_insert_with(|| mesh_assets.add(Mesh::from(Sphere { radius: 1.0 })))
                .clone();
            let material = material
                .get_or_insert_with(|| material_assets.add(Color::from(css::YELLOW)))
                .clone();
            cmd.insert((Mesh3d(mesh), MeshMaterial3d(material)));
        }
    });
}

#[derive(
    Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Component, YoleckComponent, Debug,
)]
struct LaserPointer {
    target: Option<Uuid>,
}

fn edit_laser_pointer(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<&mut LaserPointer>,
    mut exclusive_queue: ResMut<YoleckExclusiveSystemsQueue>,
) {
    let Ok(mut laser_pointer) = edit.single_mut() else {
        return;
    };

    ui.horizontal(|ui| {
        let button = if let Some(target) = laser_pointer.target {
            ui.button(format!("Target: {:?}", target))
        } else {
            ui.button("No Target")
        };
        if button.clicked() {
            exclusive_queue.push_back(
                vpeol_read_click_on_entity::<With<YoleckEntityUuid>>
                    .pipe(yoleck_map_entity_to_uuid)
                    .pipe(
                        |In(target): In<Option<Uuid>>, mut edit: YoleckEdit<&mut LaserPointer>| {
                            let Ok(mut laser_pointer) = edit.single_mut() else {
                                return YoleckExclusiveSystemDirective::Finished;
                            };

                            if let Some(target) = target {
                                laser_pointer.target = Some(target);
                                YoleckExclusiveSystemDirective::Finished
                            } else {
                                YoleckExclusiveSystemDirective::Listening
                            }
                        },
                    )
                    .pipe(yoleck_exclusive_system_cancellable),
            );
        }
        if laser_pointer.target.is_some() {
            if ui.button("Clear").clicked() {
                laser_pointer.target = None;
            }
        }
    });
}

fn draw_laser_pointers(
    lasers_query: Query<(&LaserPointer, &GlobalTransform)>,
    uuid_registry: Res<YoleckUuidRegistry>,
    targets_query: Query<&GlobalTransform>,
    mut gizmos: Gizmos,
) {
    for (laser_pointer, laser_transform) in lasers_query.iter() {
        let Some(target_entity) = laser_pointer
            .target
            .and_then(|uuid| uuid_registry.get(uuid))
        else {
            continue;
        };
        let Ok(target_transform) = targets_query.get(target_entity) else {
            continue;
        };
        gizmos.line(
            laser_transform.translation(),
            target_transform.translation(),
            css::GREEN,
        );
    }
}
