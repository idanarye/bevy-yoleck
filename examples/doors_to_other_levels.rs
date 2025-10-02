use std::path::Path;

use bevy::color::palettes::css;
use bevy::math::Affine3A;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy_egui::{egui, EguiPlugin};
use bevy_yoleck::vpeol::prelude::*;
use bevy_yoleck::{prelude::*, YoleckEditableLevels};
use serde::{Deserialize, Serialize};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    let level = if cfg!(target_arch = "wasm32") {
        Some("entry.yol".to_owned())
    } else {
        std::env::args().nth(1)
    };

    app.add_plugins(EguiPlugin::default());

    if let Some(level) = level {
        app.add_plugins(YoleckPluginForGame);
        app.add_plugins(Vpeol2dPluginForGame);
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                commands.spawn(YoleckLoadLevel(
                    asset_server.load(Path::new("levels_doors").join(&level)),
                ));
            },
        );
    } else {
        app.add_plugins(YoleckPluginForEditor);
        app.add_plugins(Vpeol2dPluginForEditor);
        app.add_plugins(VpeolSelectionCuePlugin::default());

        app.insert_resource(bevy_yoleck::YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels_doors"),
        ));
    }

    app.add_systems(Startup, setup_camera);

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Player")
            .with::<Vpeol2dPosition>()
            .insert_on_init(|| IsPlayer)
    });
    app.add_systems(YoleckSchedule::Populate, populate_player);

    app.add_yoleck_entity_type({
        YoleckEntityType::new("FloatingText")
            .with::<Vpeol2dPosition>()
            .with::<Vpeol2dScale>()
            .with::<TextContent>()
    });
    app.add_yoleck_edit_system(edit_text);
    app.add_systems(YoleckSchedule::Populate, populate_text);

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Doorway")
            .with::<Vpeol2dPosition>()
            .with::<Vpeol2dRotatation>()
            .with::<Doorway>()
    });
    app.add_yoleck_edit_system(edit_doorway_rotation);
    app.add_yoleck_edit_system(edit_doorway);
    app.add_systems(YoleckSchedule::Populate, populate_doorway);
    app.add_systems(Update, set_doorways_sprite_index);

    app.add_systems(
        YoleckSchedule::LevelLoaded,
        (
            handle_player_entity_when_level_loads,
            position_level_from_opened_door,
        ),
    );

    app.add_systems(
        Update,
        (
            control_camera,
            control_player,
            handle_door_opening,
            close_old_doors,
        )
            .run_if(in_state(YoleckEditorState::GameActive)),
    );

    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Transform::from_xyz(0.0, 0.0, 100.0).with_scale(2.0 * (Vec3::X + Vec3::Y) + Vec3::Z),
        VpeolCameraState::default(),
        Vpeol2dCameraControl::default(),
    ));
}

#[derive(Component)]
struct IsPlayer;

fn populate_player(
    mut populate: YoleckPopulate<(), With<IsPlayer>>,
    asset_server: Res<AssetServer>,
    mut texture_cache: Local<Option<Handle<Image>>>,
) {
    populate.populate(|_ctx, mut cmd, ()| {
        cmd.insert(Sprite {
            image: texture_cache
                .get_or_insert_with(|| asset_server.load("sprites/player.png"))
                .clone(),
            custom_size: Some(Vec2::new(100.0, 100.0)),
            ..Default::default()
        });
    });
}

fn control_camera(
    player_query: Query<&GlobalTransform, With<IsPlayer>>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
    time: Res<Time>,
) {
    if player_query.is_empty() {
        return;
    }
    let position_to_track = player_query.iter().map(|t| t.translation()).sum::<Vec3>()
        / player_query.iter().len() as f32;
    for mut camera_transform in camera_query.iter_mut() {
        let displacement = position_to_track - camera_transform.translation;
        if displacement.length_squared() < 10000.0 {
            camera_transform.translation +=
                displacement.clamp_length_max(100.0 * time.delta_secs());
        } else {
            camera_transform.translation +=
                displacement.clamp_length_max(800.0 * time.delta_secs());
        }
    }
}

fn control_player(
    mut player_query: Query<&mut Transform, With<IsPlayer>>,
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
) {
    let mut velocity = Vec3::ZERO;
    if input.pressed(KeyCode::ArrowUp) {
        velocity += Vec3::Y;
    }
    if input.pressed(KeyCode::ArrowDown) {
        velocity -= Vec3::Y;
    }
    if input.pressed(KeyCode::ArrowLeft) {
        velocity -= Vec3::X;
    }
    if input.pressed(KeyCode::ArrowRight) {
        velocity += Vec3::X;
    }
    velocity *= 800.0;
    for mut player_transform in player_query.iter_mut() {
        player_transform.translation += velocity * time.delta_secs();
    }
}

#[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
struct TextContent {
    text: String,
}

fn edit_text(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<(&mut TextContent, &mut Vpeol2dScale)>,
) {
    let Ok((mut content, mut scale)) = edit.single_mut() else {
        return;
    };
    ui.text_edit_multiline(&mut content.text);
    // TODO: do this in vpeol_2d?
    ui.add(egui::Slider::new(&mut scale.0.x, 0.5..=5.0).logarithmic(true));
    scale.0.y = scale.0.x;
}

fn populate_text(mut populate: YoleckPopulate<&TextContent>, asset_server: Res<AssetServer>) {
    populate.populate(|ctx, mut cmd, content| {
        let text;
        let color;
        if ctx.is_in_editor() && content.text.chars().all(|c| c.is_whitespace()) {
            text = "<TEXT>".to_owned();
            color = css::WHITE.with_alpha(0.25);
        } else {
            text = content.text.clone();
            color = css::WHITE;
        };
        cmd.insert((
            Text2d(text),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 72.0,
                ..Default::default()
            },
            TextColor(color.into()),
        ));
    });
}

#[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent, Debug)]
struct Doorway {
    target_level: String,
    marker: String,
}

fn edit_doorway(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<&mut Doorway>,
    levels: Res<YoleckEditableLevels>,
) {
    let Ok(mut doorway) = edit.single_mut() else {
        return;
    };

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("doorway-level")
            .selected_text(
                Some(doorway.target_level.as_str())
                    .filter(|l| !l.is_empty())
                    .unwrap_or("<target level>"),
            )
            .show_ui(ui, |ui| {
                for level in levels.names() {
                    ui.selectable_value(&mut doorway.target_level, level.to_owned(), level);
                }
            });
        egui::TextEdit::singleline(&mut doorway.marker)
            .hint_text("<marker>")
            .show(ui);
    });
}

fn edit_doorway_rotation(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<(&Vpeol2dPosition, &mut Vpeol2dRotatation), With<Doorway>>,
    mut knobs: YoleckKnobs,
) {
    let Ok((Vpeol2dPosition(position), mut rotation)) = edit.single_mut() else {
        return;
    };
    use std::f32::consts::PI;
    ui.add(egui::Slider::new(&mut rotation.0, PI..=-PI).prefix("Angle: "));
    // TODO: do this in vpeol_2d?
    let mut rotate_knob = knobs.knob("rotate");
    let knob_position = position.extend(1.0) + Quat::from_rotation_z(rotation.0) * (75.0 * Vec3::X);
    rotate_knob.cmd.insert((
        Sprite::from_color(css::PURPLE, Vec2::new(30.0, 30.0)),
        Transform::from_translation(knob_position),
        GlobalTransform::from(Transform::from_translation(knob_position)),
    ));
    if let Some(rotate_to) = rotate_knob.get_passed_data::<Vec3>() {
        rotation.0 = Vec2::X.angle_to(rotate_to.truncate() - *position);
    }
}

fn populate_doorway(
    mut populate: YoleckPopulate<(), With<Doorway>>,
    asset_server: Res<AssetServer>,
    mut asset_handle_cache: Local<Option<(Handle<Image>, Handle<TextureAtlasLayout>)>>,
    mut texture_atlas_layout_assets: ResMut<Assets<TextureAtlasLayout>>,
) {
    populate.populate(|_ctx, mut cmd, ()| {
        let (image, texture_atlas_layout) = asset_handle_cache
            .get_or_insert_with(|| {
                (
                    asset_server.load("sprites/doorway.png"),
                    texture_atlas_layout_assets.add(TextureAtlasLayout::from_grid(
                        UVec2::new(64, 64),
                        1,
                        2,
                        None,
                        None,
                    )),
                )
            })
            .clone();
        cmd.insert(Sprite {
            image,
            custom_size: Some(Vec2::new(100.0, 100.0)),
            texture_atlas: Some(TextureAtlas {
                layout: texture_atlas_layout,
                index: 0,
            }),
            ..Default::default()
        });
    });
}

fn set_doorways_sprite_index(mut query: Query<(&mut Sprite, Has<DoorIsOpen>), With<Doorway>>) {
    for (mut sprite, door_is_open) in query.iter_mut() {
        if let Some(texture_atlas) = sprite.texture_atlas.as_mut() {
            texture_atlas.index = if door_is_open { 1 } else { 0 };
        }
    }
}

#[derive(Component)]
struct LevelFromOpenedDoor {
    exit_door: Entity,
}

#[derive(Component)]
struct PlayerHoldingLevel;

fn handle_player_entity_when_level_loads(
    levels_query: Query<Has<LevelFromOpenedDoor>, With<YoleckLevelJustLoaded>>,
    mut players_query: Query<(Entity, &mut YoleckBelongsToLevel, &Vpeol2dPosition), With<IsPlayer>>,
    mut commands: Commands,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    for (player_entity, mut belongs_to_level, player_position) in players_query.iter_mut() {
        let Ok(is_level_from_opened_door) = levels_query.get(belongs_to_level.level) else {
            continue;
        };
        if is_level_from_opened_door {
            commands.entity(player_entity).despawn();
        } else {
            belongs_to_level.level = commands
                .spawn((
                    // So that the player entity will be removed when finishing a playtest in the
                    // editor:
                    YoleckKeepLevel,
                    // So that we won't remove that level when unloading old rooms:
                    PlayerHoldingLevel,
                ))
                .id();
            let translation_for_camera = player_position.0.extend(100.0);
            for mut camera_transform in camera_query.iter_mut() {
                *camera_transform = Transform {
                    translation: translation_for_camera,
                    rotation: Quat::IDENTITY,
                    scale: 2.0 * (Vec3::X + Vec3::Y) + Vec3::Z,
                };
            }
        }
    }
}

#[derive(Component)]
struct DoorIsOpen;

#[derive(Component)]
struct DoorConnectsTo(Entity);

fn handle_door_opening(
    players_query: Query<&GlobalTransform, With<IsPlayer>>,
    doors_query: Query<
        (Entity, &YoleckBelongsToLevel, &GlobalTransform, &Doorway),
        Without<DoorIsOpen>,
    >,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    levels_query: Query<Entity, (With<YoleckKeepLevel>, Without<PlayerHoldingLevel>)>,
) {
    for player_transform in players_query.iter() {
        for (door_entity, belongs_to_level, door_transform, doorway) in doors_query.iter() {
            let distance_sq = player_transform
                .translation()
                .distance_squared(door_transform.translation());
            if distance_sq < 10000.0 {
                for level_entity in levels_query.iter() {
                    if level_entity != belongs_to_level.level {
                        commands.entity(level_entity).despawn();
                    }
                }

                commands.entity(door_entity).insert(DoorIsOpen);
                commands.spawn((
                    YoleckLoadLevel(
                        asset_server.load(format!("levels_doors/{}", doorway.target_level)),
                    ),
                    LevelFromOpenedDoor {
                        exit_door: door_entity,
                    },
                ));
            }
        }
    }
}

fn position_level_from_opened_door(
    levels_query: Query<(Entity, &LevelFromOpenedDoor), With<YoleckLevelJustLoaded>>,
    doors_query: Query<(
        Entity,
        &YoleckBelongsToLevel,
        Option<&GlobalTransform>,
        &Doorway,
        &Vpeol2dPosition,
        &Vpeol2dRotatation,
    )>,
    mut commands: Commands,
) {
    for (level_entity, level_from_opened_door) in levels_query.iter() {
        let Ok((_, _, Some(exit_door_transform), exit_doorway, _, _)) =
            doors_query.get(level_from_opened_door.exit_door)
        else {
            continue;
        };
        let exit_door_affine = exit_door_transform.affine();
        let (entry_door_entity, _, _, _, entry_door_position, entry_door_rotation) = doors_query
            .iter()
            .find(|(_, belongs_to_level, _, entry_doorway, _, _)| {
                belongs_to_level.level == level_entity
                    && entry_doorway.marker == exit_doorway.marker
            })
            .expect(&format!(
                "Cannot find a door marked as {:?} in {:?}",
                exit_doorway.marker, exit_doorway.target_level
            ));
        let entry_door_affine = Affine3A::from_rotation_translation(
            Quat::from_rotation_z(entry_door_rotation.0),
            entry_door_position.0.extend(0.0),
        );
        let rotate_door_around = Affine3A::from_rotation_translation(
            Quat::from_rotation_z(std::f32::consts::PI),
            100.0 * Vec3::X,
        );
        let level_transformation =
            exit_door_affine * rotate_door_around * entry_door_affine.inverse();
        let level_transformation = Transform::from_matrix(level_transformation.into());

        commands
            .entity(level_from_opened_door.exit_door)
            .insert(DoorConnectsTo(entry_door_entity));
        commands
            .entity(entry_door_entity)
            .insert((DoorIsOpen, DoorConnectsTo(level_from_opened_door.exit_door)));
        commands
            .entity(level_entity)
            .insert(VpeolRepositionLevel(level_transformation));
    }
}

fn close_old_doors(
    mut removed_doors: RemovedComponents<DoorConnectsTo>,
    doors_query: Query<(Entity, &DoorConnectsTo)>,
    mut commands: Commands,
) {
    if removed_doors.is_empty() {
        return;
    }
    let removed_doors = removed_doors.read().collect::<HashSet<Entity>>();

    for (door_entity, door_connects_to) in doors_query.iter() {
        if removed_doors.contains(&door_connects_to.0) {
            commands
                .entity(door_entity)
                .remove::<(DoorIsOpen, DoorConnectsTo)>();
        }
    }
}
