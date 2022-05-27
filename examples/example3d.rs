use std::path::Path;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_yoleck::editools_3d::{
    transform_edit_adapter, OrbitCameraBundle, OrbitCameraController, Tools3DCameraBundle,
    Transform3dProjection, WillContainClickableChildren,
};
use bevy_yoleck::{
    YoleckEditorLevelsDirectoryPath, YoleckEditorState, YoleckExtForApp, YoleckLoadingCommand,
    YoleckPluginForEditor, YoleckPluginForGame, YoleckPopulate, YoleckTypeHandlerFor,
};
use serde::{Deserialize, Serialize};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    let level = std::env::args().nth(1);
    if let Some(level) = level {
        app.add_plugin(YoleckPluginForGame);
        app.add_startup_system(
            move |asset_server: Res<AssetServer>,
                  mut yoleck_loading_command: ResMut<YoleckLoadingCommand>| {
                *yoleck_loading_command = YoleckLoadingCommand::FromAsset(
                    asset_server.load(Path::new("levels3d").join(&level)),
                );
            },
        );
    } else {
        app.add_plugin(EguiPlugin);
        app.add_plugin(YoleckPluginForEditor);
        app.insert_resource(YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels3d"),
        ));
        app.add_plugin(bevy_yoleck::editools_3d::YoleckEditools3dPlugin);
    }
    app.add_yoleck_handler({
        YoleckTypeHandlerFor::<Spaceship>::new("Spaceship")
            .populate_with(populate_spaceship)
            .with(transform_edit_adapter(|data: &mut Spaceship| {
                Transform3dProjection {
                    translation: &mut data.position,
                    rotation: Some(&mut data.rotation),
                }
            }))
    });
    app.add_yoleck_handler({
        YoleckTypeHandlerFor::<Planet>::new("Planet")
            .populate_with(populate_planet)
            .with(transform_edit_adapter(|data: &mut Planet| {
                Transform3dProjection {
                    translation: &mut data.position,
                    rotation: None,
                }
            }))
    });
    app.init_resource::<GameAssets>();
    app.add_startup_system(setup_camera);
    app.add_system_set({
        SystemSet::on_update(YoleckEditorState::GameActive)
            .with_system(control_spaceship)
            .with_system(hit_planets)
    });
    app.run();
}

struct GameAssets {
    spaceship_model: Handle<Scene>,
    planet_model: Handle<Scene>,
}

impl FromWorld for GameAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self {
            spaceship_model: asset_server.load("models/spaceship.glb#Scene0"),
            planet_model: asset_server.load("models/planet.glb#Scene0"),
        }
    }
}

fn setup_camera(mut commands: Commands) {
    let camera = Tools3DCameraBundle::new(OrbitCameraBundle::new(
        {
            let mut controller = OrbitCameraController::default();
            controller.mouse_translate_sensitivity *= 10.0;
            controller
        },
        PerspectiveCameraBundle::new_3d(),
        Vec3::new(0.0, 10.0, 10.0),
        Vec3::ZERO,
    ));
    commands.spawn_bundle(camera);

    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::WHITE,
            illuminance: 50_000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::from_xyz(10.0, 10.0, 20.0)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..Default::default()
    });
}

#[derive(Component)]
struct IsSpaceship;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Spaceship {
    #[serde(default)]
    position: Vec3,
    #[serde(default)]
    rotation: Quat,
}

fn populate_spaceship(mut populate: YoleckPopulate<Spaceship>, assets: Res<GameAssets>) {
    populate.populate(|_ctx, data, mut cmd| {
        cmd.despawn_descendants();
        cmd.insert_bundle(TransformBundle::from_transform(
            Transform::from_translation(data.position).with_rotation(data.rotation),
        ));
        cmd.with_children(|commands| {
            commands.spawn_scene(assets.spaceship_model.clone());
        });
        cmd.insert(WillContainClickableChildren);
        cmd.insert(IsSpaceship);
    });
}

fn control_spaceship(
    mut spaceship_query: Query<&mut Transform, With<IsSpaceship>>,
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
) {
    let calc_axis = |neg: KeyCode, pos: KeyCode| match (input.pressed(neg), input.pressed(pos)) {
        (true, true) | (false, false) => 0.0,
        (true, false) => -1.0,
        (false, true) => 1.0,
    };
    let pitch = calc_axis(KeyCode::Up, KeyCode::Down);
    let roll = calc_axis(KeyCode::Left, KeyCode::Right);
    for mut spaceship_transform in spaceship_query.iter_mut() {
        let forward_direction = spaceship_transform.rotation.mul_vec3(-Vec3::Z);
        let roll_quat =
            Quat::from_scaled_axis(2.0 * forward_direction * time.delta_seconds() * roll);
        let pitch_axis = spaceship_transform.rotation.mul_vec3(Vec3::X);
        let pitch_quat = Quat::from_scaled_axis(2.0 * pitch_axis * time.delta_seconds() * pitch);
        spaceship_transform.rotation = roll_quat * pitch_quat * spaceship_transform.rotation;
        spaceship_transform.translation += 2.0 * forward_direction * time.delta_seconds();
    }
}

#[derive(Component)]
struct IsPlanet;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Planet {
    #[serde(default)]
    position: Vec3,
}

fn populate_planet(mut populate: YoleckPopulate<Planet>, assets: Res<GameAssets>) {
    populate.populate(|_ctx, data, mut cmd| {
        cmd.insert_bundle(TransformBundle::from_transform(
            Transform::from_translation(data.position),
        ));
        cmd.with_children(|commands| {
            commands.spawn_scene(assets.planet_model.clone());
        });
        cmd.insert(WillContainClickableChildren);
        cmd.insert(IsPlanet);
    });
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
                commands.entity(planet_entity).despawn_recursive();
            }
        }
    }
}
