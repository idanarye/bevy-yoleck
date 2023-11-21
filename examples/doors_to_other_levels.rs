use std::path::Path;

use bevy::math::Affine3A;
use bevy::prelude::*;
use bevy_egui::{egui, EguiPlugin};
use bevy_yoleck::vpeol::prelude::*;
use bevy_yoleck::{prelude::*, YoleckEditableLevels};
use serde::{Deserialize, Serialize};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    let level = std::env::args().nth(1);

    app.add_plugins(EguiPlugin);

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
    app.yoleck_populate_schedule_mut()
        .add_systems(populate_player);

    app.add_yoleck_entity_type({
        YoleckEntityType::new("FloatingText")
            .with::<Vpeol2dPosition>()
            .with::<Vpeol2dScale>()
            .with::<TextContent>()
    });
    app.add_yoleck_edit_system(edit_text);
    app.yoleck_populate_schedule_mut()
        .add_systems(populate_text);

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Doorway")
            .with::<Vpeol2dPosition>()
            .with::<Vpeol2dRotatation>()
            .with::<Doorway>()
    });
    app.add_yoleck_edit_system(edit_doorway_rotation);
    app.add_yoleck_edit_system(edit_doorway);
    app.yoleck_populate_schedule_mut()
        .add_systems(populate_doorway);

    app.add_systems(
        YoleckSchedule::LevelLoaded,
        (
            remove_players_from_opened_door_levels,
            position_level_from_opened_door,
        ),
    );

    app.add_systems(
        Update,
        (control_player, handle_door_opening).run_if(in_state(YoleckEditorState::GameActive)),
    );

    app.run();
}

fn setup_camera(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.transform.translation.z = 100.0;
    camera.transform.scale *= 2.0 * (Vec3::X + Vec3::Y) + Vec3::Z;
    commands
        .spawn(camera)
        .insert(VpeolCameraState::default())
        .insert(Vpeol2dCameraControl::default());
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

fn populate_player(
    mut populate: YoleckPopulate<(), With<IsPlayer>>,
    asset_server: Res<AssetServer>,
) {
    populate.populate(|_ctx, mut cmd, ()| {
        cmd.insert((SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(100.0, 100.0)),
                ..Default::default()
            },
            texture: asset_server.load("sprites/player.png"),
            ..Default::default()
        },));
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

#[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
struct TextContent {
    text: String,
}

fn edit_text(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<(&mut TextContent, &mut Vpeol2dScale)>,
) {
    let Ok((mut content, mut scale)) = edit.get_single_mut() else {
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
            color = Color::WHITE.with_a(0.25);
        } else {
            text = content.text.clone();
            color = Color::WHITE;
        };
        cmd.insert(Text2dBundle {
            text: {
                Text::from_section(
                    text,
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 72.0,
                        color,
                    },
                )
            },
            ..Default::default()
        });
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
    let Ok(mut doorway) = edit.get_single_mut() else {
        return;
    };

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_source("doorway-level")
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
    let Ok((Vpeol2dPosition(position), mut rotation)) = edit.get_single_mut() else {
        return;
    };
    use std::f32::consts::PI;
    ui.add(egui::Slider::new(&mut rotation.0, PI..=-PI).prefix("Angle: "));
    // TODO: do this in vpeol_2d?
    let mut rotate_knob = knobs.knob("rotate");
    let knob_position = position.extend(1.0) + Quat::from_rotation_z(rotation.0) * (75.0 * Vec3::X);
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
        rotation.0 = Vec2::X.angle_between(rotate_to.truncate() - *position);
    }
}

fn populate_doorway(
    mut populate: YoleckPopulate<(), With<Doorway>>,
    asset_server: Res<AssetServer>,
) {
    populate.populate(|_ctx, mut cmd, ()| {
        cmd.insert(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(100.0, 100.0)),
                ..Default::default()
            },
            texture: asset_server.load("sprites/doorway.png"),
            ..Default::default()
        });
    });
}

#[derive(Component)]
struct LevelFromOpenedDoor {
    exit_door: Entity,
}

fn remove_players_from_opened_door_levels(
    levels_query: Query<Entity, With<LevelFromOpenedDoor>>,
    players_query: Query<(Entity, &YoleckBelongsToLevel), With<IsPlayer>>,
    mut commands: Commands,
) {
    for (player_entity, belongs_to_level) in players_query.iter() {
        if !levels_query.contains(belongs_to_level.level) {
            continue;
        }
        commands.entity(player_entity).despawn_recursive();
    }
}

#[derive(Component)]
struct DoorIsOpen;

fn handle_door_opening(
    players_query: Query<&GlobalTransform, With<IsPlayer>>,
    doors_query: Query<(Entity, &GlobalTransform, &Doorway), Without<DoorIsOpen>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for player_transform in players_query.iter() {
        for (door_entity, door_transform, doorway) in doors_query.iter() {
            let distance_sq = player_transform
                .translation()
                .distance_squared(door_transform.translation());
            if distance_sq < 10000.0 {
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

        commands.entity(entry_door_entity).insert(DoorIsOpen);
        commands
            .entity(level_entity)
            .insert(VpeolRepositionLevel(level_transformation));
    }
}
