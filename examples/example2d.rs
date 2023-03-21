use std::path::Path;

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

use bevy_yoleck::vpeol::{VpeolCameraState, VpeolWillContainClickableChildren, YoleckKnobClick};
use bevy_yoleck::vpeol_2d::{
    vpeol_position_edit_adapter, Vpeol2dCameraControl, VpeolTransform2dProjection,
};
use bevy_yoleck::{
    YoleckComponent, YoleckDirective, YoleckEdit, YoleckEditNewStyle,
    YoleckEditorLevelsDirectoryPath, YoleckEditorState, YoleckEntityType,
    YoleckEntityUpgradingPlugin, YoleckExtForApp, YoleckLoadingCommand, YoleckPluginForEditor,
    YoleckPluginForGame, YoleckPopulate, YoleckPopulateNewStyle, YoleckTypeHandler, YoleckUi,
};
use serde::{Deserialize, Serialize};

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
                    asset_server.load(Path::new("levels2d").join(&level)),
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
        app.add_plugin(bevy_yoleck::vpeol_2d::Vpeol2dPlugin);
        app.add_plugin(bevy_yoleck::vpeol::VpeolSelectionCuePlugin::default());
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

    app.add_plugin(YoleckEntityUpgradingPlugin {
        app_format_version: 1,
    });

    app.add_startup_system(setup_camera);

    app.add_yoleck_handler({
        YoleckTypeHandler::<Player>::new("Player")
            .populate_with(populate_player)
            .with(vpeol_position_edit_adapter(|data: &mut Player| {
                VpeolTransform2dProjection {
                    translation: &mut data.position,
                }
            }))
            .edit_with(edit_player)
    });

    app.add_yoleck_handler({
        YoleckTypeHandler::<Fruit>::new("Fruit")
            .populate_with(populate_fruit)
            .edit_with(duplicate_fruit)
            .with(vpeol_position_edit_adapter(|data: &mut Fruit| {
                VpeolTransform2dProjection {
                    translation: &mut data.position,
                }
            }))
            .edit_with(edit_fruit)
    });
    app.add_yoleck_entity_type(YoleckEntityType::new("Fruit").with::<Fruit>());
    app.add_yoleck_edit_system(edit_fruit_type);
    app.yoleck_populate_schedule_mut()
        .add_system(populate_fruit_type);
    app.add_yoleck_entity_upgrade(1, |_, data| {
        // TODO: something better than that
        if let Some(fruit_index) = data.pointer_mut("/Fruit/fruit_index") {
            *fruit_index = (fruit_index.as_u64().unwrap() - 1).into();
        }
    });

    app.add_yoleck_handler({
        YoleckTypeHandler::<FloatingText>::new("FloatingText")
            .populate_with(populate_text)
            .with(vpeol_position_edit_adapter(
                |floating_text: &mut FloatingText| {
                    bevy_yoleck::vpeol_2d::VpeolTransform2dProjection {
                        translation: &mut floating_text.position,
                    }
                },
            ))
            .edit_with(edit_text)
    });

    app.add_systems((control_player, eat_fruits).in_set(OnUpdate(YoleckEditorState::GameActive)));
    app.run();
}

fn setup_camera(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.transform.translation.z = 100.0;
    commands
        .spawn(camera)
        .insert(VpeolCameraState::default())
        .insert(Vpeol2dCameraControl::default());
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
        let mut system_state =
            SystemState::<(Res<AssetServer>, ResMut<Assets<TextureAtlas>>, EguiContexts)>::new(
                world,
            );
        let (asset_server, mut texture_atlas_assets, mut egui_context) =
            system_state.get_mut(world);
        let fruits_atlas = TextureAtlas::from_grid(
            asset_server.load("sprites/fruits.png"),
            Vec2::new(64.0, 64.0),
            3,
            1,
            None,
            None,
        );
        let fruits_egui = {
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

#[derive(Clone, PartialEq, Serialize, Deserialize, Component)]
struct Fruit {
    #[serde(default)]
    position: Vec2,
    #[serde(default)]
    fruit_index: usize,
}

impl YoleckComponent for Fruit {
    const KEY: &'static str = "Fruit";
}

fn populate_fruit(mut populate: YoleckPopulate<Fruit>, assets: Res<GameAssets>) {
    populate.populate(|_ctx, data, mut cmd| {
        cmd.despawn_descendants();
        cmd.insert((
            SpatialBundle::from_transform(Transform::from_translation(data.position.extend(0.0))),
            VpeolWillContainClickableChildren,
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

fn duplicate_fruit(mut edit: YoleckEdit<Fruit>, mut writer: EventWriter<YoleckDirective>) {
    edit.edit(|_ctx, data, ui| {
        if ui.button("Duplicate").clicked() {
            writer.send(YoleckDirective::spawn_entity(
                "Fruit",
                Fruit {
                    position: data.position - 100.0 * Vec2::Y,
                    fruit_index: data.fruit_index,
                },
                true, // select_created_entity
            ));
        }
    });
}

fn edit_fruit(mut edit: YoleckEdit<Fruit>, assets: Res<GameAssets>, mut commands: Commands) {
    edit.edit(|ctx, data, ui| {
        ui.horizontal(|ui| {
            ui.label(format!("Old Style:\n#{} chosen", data.fruit_index));
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

fn edit_fruit_type(
    mut ui: ResMut<YoleckUi>,
    mut query: Query<&mut Fruit, With<YoleckEditNewStyle>>,
    assets: Res<GameAssets>,
) {
    let Ok(mut fruit) = query.get_single_mut() else { return };
    ui.horizontal(|ui| {
        ui.label(format!("New Style:\n#{} chosen", fruit.fruit_index));
        let (texture_id, rects) = &assets.fruits_sprite_sheet_egui;
        for (index, rect) in rects.iter().enumerate() {
            if ui
                .add_enabled(
                    index != fruit.fruit_index,
                    egui::ImageButton::new(*texture_id, [100.0, 100.0]).uv(*rect),
                )
                .clicked()
            {
                fruit.fruit_index = index;
            }

            if index != fruit.fruit_index {
                // TODO: knobs
                /*
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
                */
            }
        }
    });
}

fn populate_fruit_type(mut populate: YoleckPopulateNewStyle<&mut Fruit>, assets: Res<GameAssets>) {
    info!("Running populate_fruit_type");
    populate.populate(|_ctx, mut cmd, fruit| {
        info!("Fruit index {}", fruit.fruit_index);
        cmd.despawn_descendants();
        cmd.insert((
            SpatialBundle::from_transform(Transform::from_translation(fruit.position.extend(0.0))),
            VpeolWillContainClickableChildren,
            IsFruit,
        ));
        // Could have placed them on the main entity, but with this the children picking feature
        // can be tested and demonstrated.
        cmd.with_children(|commands| {
            commands.spawn(SpriteSheetBundle {
                sprite: TextureAtlasSprite {
                    index: fruit.fruit_index,
                    custom_size: Some(Vec2::new(100.0, 100.0)),
                    ..Default::default()
                },
                texture_atlas: assets.fruits_sprite_sheet.clone(),
                ..Default::default()
            });
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
