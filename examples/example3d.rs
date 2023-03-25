use std::path::Path;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

use bevy_yoleck::prelude::*;
use bevy_yoleck::vpeol::{VpeolCameraState, VpeolWillContainClickableChildren};
use bevy_yoleck::vpeol_3d::Vpeol3dPosition;
// use serde::{Deserialize, Serialize};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    let level = std::env::args().nth(1);
    if let Some(level) = level {
        // The egui plugin is not needed for the game itself, but GameAssets won't load without it
        // because it needs `EguiContexts` which cannot be `Option` because it's a custom
        // `SystemParam`.
        app.add_plugin(EguiPlugin);

        app.add_plugin(YoleckPluginForGame);
        app.add_startup_system(
            move |asset_server: Res<AssetServer>,
                  mut yoleck_loading_command: ResMut<YoleckLoadingCommand>| {
                *yoleck_loading_command = YoleckLoadingCommand::FromAsset(
                    asset_server.load(Path::new("levels3d").join(&level)),
                );
            },
        );
        app.add_plugin(bevy_yoleck::vpeol_3d::Vpeol3dPluginForGame);
    } else {
        app.add_plugin(EguiPlugin);
        app.add_plugin(YoleckPluginForEditor);
        // Adding `YoleckEditorLevelsDirectoryPath` is not usually required -
        // `YoleckPluginForEditor` will add one with "assets/levels". Here we want to support
        // example2d and example3d in the same repository so we use different directories.
        app.insert_resource(bevy_yoleck::YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels3d"),
        ));
        app.add_plugin(bevy_yoleck::vpeol_3d::Vpeol3dPluginForEditor);
        app.add_plugin(bevy_yoleck::vpeol::VpeolSelectionCuePlugin::default());
        #[cfg(target_arch = "wasm32")]
        app.add_startup_system(
            |asset_server: Res<AssetServer>,
             mut yoleck_loading_command: ResMut<YoleckLoadingCommand>| {
                *yoleck_loading_command =
                    YoleckLoadingCommand::FromAsset(asset_server.load("levels3d/example.yol"));
            },
        );
    }
    app.add_startup_system(setup_camera);

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Spaceship")
            .with::<Vpeol3dPosition>()
            .insert_on_init(|| IsSpaceship)
    });
    app.yoleck_populate_schedule_mut()
        .add_system(populate_spaceship);

    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 16.0, 40.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            ..Default::default()
        })
        .insert(VpeolCameraState::default());

    commands.spawn(DirectionalLightBundle {
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

fn populate_spaceship(
    mut populate: YoleckPopulate<(), With<IsSpaceship>>,
    asset_server: Res<AssetServer>,
) {
    populate.populate(|_ctx, mut cmd, ()| {
        cmd.insert(VpeolWillContainClickableChildren);
        cmd.insert(SceneBundle {
            scene: asset_server.load("models/spaceship.glb#Scene0"),
            ..Default::default()
        });
    });
}
