use std::path::Path;

use bevy::{color::palettes::css, log::LogPlugin};
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_yoleck::prelude::*;
use bevy_yoleck::vpeol::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins.set(LogPlugin {
            custom_layer: bevy_yoleck::console_layer_factory,
            ..default()
        })
    );

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
    }

    app.add_systems(Startup, (setup_camera, setup_arena));

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Spaceship")
            .with::<Vpeol3dPosition>()
            .with::<SpaceshipSettings>()
            .insert_on_init(|| IsSpaceship)
    });
    app.add_yoleck_auto_edit::<SpaceshipSettings>();
    app.add_systems(YoleckSchedule::Populate, populate_spaceship);

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Planet")
            .with_uuid()
            .with::<Vpeol3dPosition>()
            .with::<Vpeol3dRotation>()
            .with::<Vpeol3dScale>()
            .insert_on_init(|| IsPlanet)
            .insert_on_init_during_editor(|| VpeolDragPlane::XY)
    });
    app.add_systems(YoleckSchedule::Populate, populate_planet);

    app.add_yoleck_entity_type({
        YoleckEntityType::new("PlanetPointer")
            .with::<Vpeol3dPosition>()
            .with::<LaserPointer>()
            .insert_on_init(|| SimpleSphere)
    });
    app.add_yoleck_auto_edit::<LaserPointer>();
    app.add_systems(YoleckSchedule::Populate, populate_simple_sphere);
    app.add_systems(Update, (resolve_laser_pointers, draw_laser_pointers));

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
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 16.0, 40.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        VpeolCameraState::default(),
        Vpeol3dCameraControl::topdown(),
    ));

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

// ============================================================================
// Spaceship
// ============================================================================

#[derive(Component)]
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

fn populate_spaceship(
    mut populate: YoleckPopulate<&SpaceshipSettings, With<IsSpaceship>>,
    asset_server: Res<AssetServer>,
) {
    populate.populate(|ctx, mut cmd, _settings| {
        cmd.insert(VpeolWillContainClickableChildren);
        if ctx.is_first_time() {
            cmd.insert(SceneRoot(asset_server.load("models/spaceship.glb#Scene0")));
        }
    });
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

#[derive(Component)]
struct IsPlanet;

fn populate_planet(
    mut populate: YoleckPopulate<(), With<IsPlanet>>,
    asset_server: Res<AssetServer>,
) {
    populate.populate(|ctx, mut cmd, ()| {
        cmd.insert(VpeolWillContainClickableChildren);
        if ctx.is_first_time() {
            cmd.insert(SceneRoot(asset_server.load("models/planet.glb#Scene0")));
        }
    });
}

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

#[derive(Component)]
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

fn resolve_laser_pointers(
    mut query: Query<&mut LaserPointer>,
    uuid_registry: Res<YoleckUuidRegistry>,
) {
    for mut laser_pointer in query.iter_mut() {
        laser_pointer.target.resolve(&uuid_registry);
    }
}

fn draw_laser_pointers(
    query: Query<(&LaserPointer, &GlobalTransform)>,
    targets_query: Query<&GlobalTransform>,
    mut gizmos: Gizmos,
) {
    for (laser_pointer, source_transform) in query.iter() {
        if let Some(target_entity) = laser_pointer.target.get() {
            if let Ok(target_transform) = targets_query.get(target_entity) {
                gizmos.line(
                    source_transform.translation(),
                    target_transform.translation(),
                    css::LIMEGREEN,
                );
            }
        }
    }
}
