use std::path::Path;

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin};

use bevy_yoleck::vpeol::{YoleckKnobClick, YoleckVpeolCameraState};
use bevy_yoleck::vpeol_2d::{
    yoleck_vpeol_position_edit_adapter, YoleckVpeolTransform2dProjection,
    YoleckWillContainClickableChildren,
};
use bevy_yoleck::{
    YoleckEdit, YoleckEditorLevelsDirectoryPath, YoleckEditorState, YoleckExtForApp,
    YoleckLoadingCommand, YoleckPluginForEditor, YoleckPluginForGame, YoleckPopulate,
    YoleckTypeHandler,
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
        app.add_plugin(bevy_yoleck::vpeol_2d::YoleckVpeol2dPlugin);
        app.add_plugin(bevy_yoleck::vpeol::YoleckVpeolSelectionCuePlugin::default());
        #[cfg(target_arch = "wasm32")]
        app.add_startup_system(
            |asset_server: Res<AssetServer>,
             mut yoleck_loading_command: ResMut<YoleckLoadingCommand>| {
                *yoleck_loading_command =
                    YoleckLoadingCommand::FromAsset(asset_server.load("levels2d/example.yol"));
            },
        );
    }
    app.init_resource::<GameAssets>();

    app.add_startup_system(setup_camera);

    app.add_yoleck_handler({
        YoleckTypeHandler::<Player>::new("Player")
            .populate_with(populate_player)
            .with(yoleck_vpeol_position_edit_adapter(|data: &mut Player| {
                YoleckVpeolTransform2dProjection {
                    translation: &mut data.position,
                }
            }))
            .edit_with(edit_player)
    });

    app.add_yoleck_handler({
        YoleckTypeHandler::<Fruit>::new("Fruit")
            .populate_with(populate_fruit)
            .with(yoleck_vpeol_position_edit_adapter(|data: &mut Fruit| {
                YoleckVpeolTransform2dProjection {
                    translation: &mut data.position,
                }
            }))
            .edit_with(edit_fruit)
    });

    app.add_yoleck_handler({
        YoleckTypeHandler::<FloatingText>::new("FloatingText")
            .populate_with(populate_text)
            .with(yoleck_vpeol_position_edit_adapter(
                |floating_text: &mut FloatingText| {
                    bevy_yoleck::vpeol_2d::YoleckVpeolTransform2dProjection {
                        translation: &mut floating_text.position,
                    }
                },
            ))
            .edit_with(edit_text)
    });

    app.add_system_set({
        SystemSet::on_update(YoleckEditorState::GameActive)
            .with_system(control_player)
            .with_system(eat_fruits)
    });
    app.run();
}

fn setup_camera(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.transform.translation.z = 100.0;
    commands
        .spawn(camera)
        .insert(YoleckVpeolCameraState::default());
}

#[derive(Resource)]
struct GameAssets {
    player_sprite: Handle<Image>,
    fruits_sprite_sheet: Handle<TextureAtlas>,
    fruits_sprite_sheet_egui: (egui::TextureId, Vec<egui::Rect>),
    font: Handle<Font>,
}

impl FromWorld for GameAssets {
    fn from_world(world: &mut World) -> Self {
        let (asset_server, mut texture_atlas_assets, egui_context) = SystemState::<(
            Res<AssetServer>,
            ResMut<Assets<TextureAtlas>>,
            Option<ResMut<EguiContext>>,
        )>::new(world)
        .get_mut(world);
        let fruits_atlas = TextureAtlas::from_grid(
            asset_server.load("sprites/fruits.png"),
            Vec2::new(64.0, 64.0),
            3,
            1,
            None,
            None,
        );
        let fruits_egui = if let Some(mut egui_context) = egui_context {
            (
                egui_context.add_image(fruits_atlas.texture.clone()),
                fruits_atlas
                    .textures
                    .iter()
                    .map(|rect| {
                        [
                            [
                                rect.min.x / fruits_atlas.size.x,
                                rect.min.y / fruits_atlas.size.y,
                            ]
                            .into(),
                            [
                                rect.max.x / fruits_atlas.size.x,
                                rect.max.y / fruits_atlas.size.y,
                            ]
                            .into(),
                        ]
                        .into()
                    })
                    .collect(),
            )
        } else {
            Default::default()
        };
        Self {
            player_sprite: asset_server.load("sprites/player.png"),
            fruits_sprite_sheet: texture_atlas_assets.add(fruits_atlas),
            fruits_sprite_sheet_egui: fruits_egui,
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        }
    }
}

#[derive(Component)]
struct IsPlayer;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Player {
    #[serde(default)]
    position: Vec2,
    #[serde(default)]
    rotation: f32,
}

fn populate_player(mut populate: YoleckPopulate<Player>, assets: Res<GameAssets>) {
    populate.populate(|_ctx, data, mut cmd| {
        cmd.insert((
            SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(100.0, 100.0)),
                    ..Default::default()
                },
                transform: Transform::from_translation(data.position.extend(0.0))
                    .with_rotation(Quat::from_rotation_z(data.rotation)),
                texture: assets.player_sprite.clone(),
                ..Default::default()
            },
            IsPlayer,
        ));
    });
}

fn edit_player(mut edit: YoleckEdit<Player>, mut commands: Commands) {
    edit.edit(|ctx, data, ui| {
        use std::f32::consts::PI;
        ui.add(egui::Slider::new(&mut data.rotation, PI..=-PI).prefix("Angle: "));

        let mut rotate_knob = ctx.knob(&mut commands, "rotate");
        let knob_position =
            data.position.extend(1.0) + Quat::from_rotation_z(data.rotation) * (50.0 * Vec3::Y);
        rotate_knob.cmd.insert(SpriteBundle {
            sprite: Sprite {
                color: Color::PURPLE,
                custom_size: Some(Vec2::new(30.0, 30.0)),
                ..Default::default()
            },
            transform: Transform::from_translation(knob_position),
            global_transform: Transform::from_translation(knob_position).into(),
            ..Default::default()
        });
        if let Some(rotate_to) = rotate_knob.get_passed_data::<Vec3>() {
            data.rotation = Vec2::Y.angle_between(rotate_to.truncate() - data.position);
        }
    });
}

fn control_player(
    mut player_query: Query<&mut Transform, With<IsPlayer>>,
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
    velocity *= 400.0;
    for mut player_transform in player_query.iter_mut() {
        player_transform.translation += velocity * time.delta_seconds();
    }
}

#[derive(Component)]
struct IsFruit;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Fruit {
    #[serde(default)]
    position: Vec2,
    #[serde(default)]
    fruit_index: usize,
}

fn populate_fruit(mut populate: YoleckPopulate<Fruit>, assets: Res<GameAssets>) {
    populate.populate(|_ctx, data, mut cmd| {
        cmd.despawn_descendants();
        cmd.insert((
            SpatialBundle::from_transform(Transform::from_translation(data.position.extend(0.0))),
            YoleckWillContainClickableChildren,
            IsFruit,
        ));
        // Could have placed them on the main entity, but with this the children picking feature
        // can be tested and demonstrated.
        cmd.with_children(|commands| {
            commands.spawn(SpriteSheetBundle {
                sprite: TextureAtlasSprite {
                    index: data.fruit_index,
                    custom_size: Some(Vec2::new(100.0, 100.0)),
                    ..Default::default()
                },
                texture_atlas: assets.fruits_sprite_sheet.clone(),
                ..Default::default()
            });
        });
    });
}

fn edit_fruit(mut edit: YoleckEdit<Fruit>, assets: Res<GameAssets>, mut commands: Commands) {
    edit.edit(|ctx, data, ui| {
        ui.horizontal(|ui| {
            let (texture_id, rects) = &assets.fruits_sprite_sheet_egui;
            for (index, rect) in rects.iter().enumerate() {
                if ui
                    .add_enabled(
                        index != data.fruit_index,
                        egui::ImageButton::new(*texture_id, [100.0, 100.0]).uv(*rect),
                    )
                    .clicked()
                {
                    data.fruit_index = index;
                }

                if index != data.fruit_index {
                    let mut knob = ctx.knob(&mut commands, ("select", index));
                    let knob_position =
                        (data.position + Vec2::new(-30.0 + index as f32 * 30.0, 50.0)).extend(1.0);
                    knob.cmd.insert(SpriteSheetBundle {
                        sprite: TextureAtlasSprite {
                            index,
                            custom_size: Some(Vec2::new(20.0, 20.0)),
                            ..Default::default()
                        },
                        texture_atlas: assets.fruits_sprite_sheet.clone(),
                        transform: Transform::from_translation(knob_position),
                        global_transform: Transform::from_translation(knob_position).into(),
                        ..Default::default()
                    });
                    if knob.get_passed_data::<YoleckKnobClick>().is_some() {
                        data.fruit_index = index;
                    }
                }
            }
        });
    });
}

fn eat_fruits(
    player_query: Query<&Transform, With<IsPlayer>>,
    fruits_query: Query<(Entity, &Transform), With<IsFruit>>,
    mut commands: Commands,
) {
    for player_transform in player_query.iter() {
        for (fruit_entity, fruit_transform) in fruits_query.iter() {
            if player_transform
                .translation
                .distance_squared(fruit_transform.translation)
                < 100.0f32.powi(2)
            {
                commands.entity(fruit_entity).despawn_recursive();
            }
        }
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
        cmd.insert(Text2dBundle {
            text: {
                Text::from_section(
                    data.text.clone(),
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 72.0,
                        color: Color::WHITE,
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
