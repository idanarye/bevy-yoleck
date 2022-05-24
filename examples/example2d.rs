use std::path::Path;

use bevy::prelude::*;
use bevy_egui::{egui, EguiPlugin};

use bevy_yoleck::tools_2d::{position_edit_adapter, Transform2dProjection};
use bevy_yoleck::{
    YoleckEdit, YoleckEditorLevelsDirectoryPath, YoleckEditorState, YoleckExtForApp,
    YoleckLoadingCommand, YoleckPluginForEditor, YoleckPluginForGame, YoleckPopulate,
    YoleckTypeHandlerFor,
};
use serde::{Deserialize, Serialize};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.init_resource::<GameAssets>();
    let level = std::env::args().nth(1);
    if let Some(level) = level {
        app.add_plugin(YoleckPluginForGame);
        app.add_startup_system(
            move |asset_server: Res<AssetServer>,
                  mut yoleck_loading_command: ResMut<YoleckLoadingCommand>| {
                *yoleck_loading_command = YoleckLoadingCommand::FromAsset(
                    asset_server.load(Path::new("levels").join(&level)),
                );
            },
        );
    } else {
        app.add_plugin(EguiPlugin);
        app.add_plugin(YoleckPluginForEditor);
        // Adding `YoleckEditorLevelsDirectoryPath` is not usually required -
        // `YoleckPluginForEditor` will add one with "assets/levels". Here we want to support
        // example3d in the same repository so we use different directories.
        app.insert_resource(YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels2d"),
        ));
        app.add_plugin(bevy_yoleck::tools_2d::YoleckTools2dPlugin);
    }

    app.add_startup_system(setup_camera);

    app.add_yoleck_handler({
        YoleckTypeHandlerFor::<Player>::new("Player")
            .populate_with(populate_player)
            .with(position_edit_adapter(|data: &mut Player| {
                Transform2dProjection {
                    translation: &mut data.position,
                }
            }))
    });

    app.add_yoleck_handler({
        YoleckTypeHandlerFor::<FloatingText>::new("FloatingText")
            .populate_with(populate_text)
            .with(position_edit_adapter(|floating_text: &mut FloatingText| {
                bevy_yoleck::tools_2d::Transform2dProjection {
                    translation: &mut floating_text.position,
                }
            }))
            .edit_with(edit_text)
    });

    app.add_system_set(
        SystemSet::on_update(YoleckEditorState::GameActive).with_system(control_player),
    );
    app.run();
}

fn setup_camera(mut commands: Commands) {
    let mut camera = OrthographicCameraBundle::new_2d();
    camera.transform.translation.z = 100.0;
    commands.spawn_bundle(camera);
}

struct GameAssets {
    player_sprite: Handle<Image>,
    #[allow(dead_code)]
    font: Handle<Font>,
}

impl FromWorld for GameAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self {
            player_sprite: asset_server.load("sprites/player.png"),
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        }
    }
}

#[derive(Component)]
struct PlayerControl {
    speed: f32,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Player {
    #[serde(default)]
    position: Vec2,
}

fn populate_player(mut populate: YoleckPopulate<Player>, assets: Res<GameAssets>) {
    populate.populate(|_ctx, data, mut cmd| {
        cmd.insert_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(100.0, 100.0)),
                ..Default::default()
            },
            transform: Transform::from_translation(data.position.extend(0.0)),
            texture: assets.player_sprite.clone(),
            ..Default::default()
        });
        cmd.insert(PlayerControl { speed: 400.0 });
    });
}

fn control_player(
    mut player_query: Query<(&PlayerControl, &mut Transform)>,
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
) {
    let mut velocity = Vec3::ZERO;
    if input.pressed(KeyCode::Up) {
        velocity += Vec3::Y;
    }
    if input.pressed(KeyCode::Down) {
        velocity -= Vec3::Y;
    }
    if input.pressed(KeyCode::Left) {
        velocity -= Vec3::X;
    }
    if input.pressed(KeyCode::Right) {
        velocity += Vec3::X;
    }
    for (player_control, mut player_transform) in player_query.iter_mut() {
        player_transform.translation += velocity * player_control.speed * time.delta_seconds();
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct FloatingText {
    #[serde(default)]
    position: Vec2,
    #[serde(default)]
    text: String,
    #[serde(default = "default_scale")]
    scale: f32,
}

fn default_scale() -> f32 {
    1.0
}

fn populate_text(mut populate: YoleckPopulate<FloatingText>, assets: Res<GameAssets>) {
    populate.populate(|_ctx, data, mut cmd| {
        cmd.insert_bundle(Text2dBundle {
            text: {
                Text::with_section(
                    data.text.clone(),
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 72.0,
                        color: Color::WHITE,
                    },
                    TextAlignment {
                        ..Default::default()
                    },
                )
            },
            transform: Transform {
                translation: data.position.extend(10.0),
                rotation: Default::default(),
                scale: Vec3::new(data.scale, data.scale, 1.0),
            },
            ..Default::default()
        });
    });
}

fn edit_text(mut edit: YoleckEdit<FloatingText>) {
    edit.edit(|_ctx, data, ui| {
        ui.text_edit_multiline(&mut data.text);
        ui.add(egui::Slider::new(&mut data.scale, 0.5..=5.0).logarithmic(true));
    });
}
