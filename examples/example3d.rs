use std::path::Path;

use bevy::prelude::*;
use bevy::{color::palettes::css, log::LogPlugin};
use bevy_egui::EguiPlugin;
use bevy_yoleck::prelude::*;
use bevy_yoleck::vpeol::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(LogPlugin {
        custom_layer: bevy_yoleck::console_layer_factory,
        ..default()
    }));

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
    }

    app.add_systems(Startup, (setup_camera, setup_arena));

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Spaceship")
            .with::<Vpeol3dPosition>()
            .with::<SpaceshipSettings>()
            .apply_scene_on_init(|| {
                bsn! {
                    IsSpaceship
                    VpeolWillContainClickableChildren
                    WorldAssetRoot("models/spaceship.glb#Scene0")
                }
            })
    });
    app.add_yoleck_auto_edit::<SpaceshipSettings>();

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Planet")
            .with_uuid()
            .with::<Vpeol3dPosition>()
            .with::<Vpeol3dRotation>()
            .with::<Vpeol3dScale>()
            .apply_scene_on_init(|| {
                bsn! {
                    IsPlanet
                    VpeolWillContainClickableChildren
                    Children [ // for some odd reason it fails otherwise
                        WorldAssetRoot("models/planet.glb#Scene0")
                    ]
                }
            })
            .insert_on_init_during_editor(|| VpeolOverrideDragPlane::XY)
    });

    app.add_yoleck_entity_type({
        YoleckEntityType::new("PlanetPointer")
            .with::<Vpeol3dPosition>()
            .with::<LaserPointer>()
            .apply_scene_on_init(|| {
                bsn! {
                    SimpleSphere
                    Mesh3d(asset_value(Sphere { radius: 1.0 }))
                    MeshMaterial3d<StandardMaterial>(asset_value(Color::from(css::YELLOW)))
                }
            })
            .insert_on_init_during_editor(|| Vpeol3dSnapToPlane {
                normal: Dir3::Y,
                offset: 2.0,
            })
    });
    app.add_yoleck_auto_edit::<LaserPointer>();
    app.add_systems(Update, draw_laser_pointers);

    app.add_systems(
        Update,
        (control_spaceship, hit_planets).run_if(in_state(YoleckEditorState::GameActive)),
    );

    app.run();
}

// ============================================================================
// Setup
// ============================================================================

fn setup_camera(mut commands: Commands) {
    commands.spawn_scene_list(bsn_list! {
        Camera3d
        Transform::from_xyz(0.0, 16.0, 40.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y)
        VpeolCameraState
        Vpeol3dCameraControl::topdown()
        --
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 50_000.0,
            shadow_maps_enabled: true,
        }
        Transform::from_xyz(0.0, 100.0, 0.0).looking_to(-Vec3::Y, Vec3::Z)
    });
}

fn setup_arena(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        Mesh3d(asset_value(Plane3d {
            normal: Dir3::Y,
            half_size: Vec2::new(100.0, 100.0),
        }))
        MeshMaterial3d<StandardMaterial>(asset_value(Color::from(css::GRAY)))
        Transform::from_xyz(0.0, -10.0, 0.0)
    });
}

// ============================================================================
// Spaceship
// ============================================================================

#[derive(Component, Default, Clone)]
struct IsSpaceship;

#[derive(Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent, YoleckAutoEdit)]
struct SpaceshipSettings {
    #[yoleck(label = "Speed", range(0.5..=10.0))]
    speed: f32,
    #[yoleck(label = "Rotation Speed", range(0.5..=5.0))]
    rotation_speed: f32,
    #[yoleck(label = "Enabled")]
    enabled: bool,
}

impl Default for SpaceshipSettings {
    fn default() -> Self {
        Self {
            speed: 2.0,
            rotation_speed: 2.0,
            enabled: true,
        }
    }
}

fn control_spaceship(
    mut query: Query<(&mut Transform, &SpaceshipSettings), With<IsSpaceship>>,
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

    for (mut transform, settings) in query.iter_mut() {
        if !settings.enabled {
            continue;
        }
        let forward = transform.rotation.mul_vec3(-Vec3::Z);
        let roll_quat =
            Quat::from_scaled_axis(settings.rotation_speed * forward * time.delta_secs() * roll);
        let pitch_axis = transform.rotation.mul_vec3(Vec3::X);
        let pitch_quat = Quat::from_scaled_axis(
            settings.rotation_speed * pitch_axis * time.delta_secs() * pitch,
        );
        transform.rotation = roll_quat * pitch_quat * transform.rotation;
        transform.translation += settings.speed * forward * time.delta_secs();
    }
}

// ============================================================================
// Planet
// ============================================================================

#[derive(Component, Default, Clone)]
struct IsPlanet;

fn hit_planets(
    spaceship_query: Query<&Transform, With<IsSpaceship>>,
    planets_query: Query<(Entity, &Transform), With<IsPlanet>>,
    mut commands: Commands,
) {
    for spaceship_transform in spaceship_query.iter() {
        for (planet_entity, planet_transform) in planets_query.iter() {
            let planet_radius = planet_transform.scale.max_element();
            let hit_distance = planet_radius + 1.0;
            if spaceship_transform
                .translation
                .distance_squared(planet_transform.translation)
                < hit_distance.powi(2)
            {
                commands.entity(planet_entity).despawn();
            }
        }
    }
}

// ============================================================================
// LaserPointer (PlanetPointer)
// ============================================================================

#[derive(Component, Default, Clone)]
struct SimpleSphere;

#[derive(
    Default,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Component,
    YoleckComponent,
    YoleckAutoEdit,
    Debug,
)]
struct LaserPointer {
    #[yoleck(entity_ref = "Planet")]
    target: YoleckEntityRef,
}

fn draw_laser_pointers(
    query: Query<(&LaserPointer, &GlobalTransform)>,
    targets_query: Query<&GlobalTransform>,
    mut gizmos: Gizmos,
) {
    for (laser_pointer, source_transform) in query.iter() {
        if let Some(target_entity) = laser_pointer.target.entity()
            && let Ok(target_transform) = targets_query.get(target_entity)
        {
            gizmos.line(
                source_transform.translation(),
                target_transform.translation(),
                css::LIMEGREEN,
            );
        }
    }
}
