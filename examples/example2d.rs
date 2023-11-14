use std::path::Path;

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::sprite::Mesh2dHandle;
use bevy::utils::Uuid;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

use bevy_yoleck::exclusive_systems::{YoleckExclusiveSystemDirective, YoleckExclusiveSystemsQueue};
use bevy_yoleck::vpeol::prelude::*;
use bevy_yoleck::{prelude::*, YoleckDirective};
use serde::{Deserialize, Serialize};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    let level = std::env::args().nth(1);
    if let Some(level) = level {
        // The egui plugin is not needed for the game itself, but GameAssets won't load without it
        // because it needs `EguiContexts` which cannot be `Option` because it's a custom
        // `SystemParam`.
        app.add_plugins(EguiPlugin);

        app.add_plugins(YoleckPluginForGame);
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>,
                  mut yoleck_loading_command: ResMut<YoleckLoadingCommand>| {
                *yoleck_loading_command = YoleckLoadingCommand::FromAsset(
                    asset_server.load(Path::new("levels2d").join(&level)),
                );
            },
        );
        app.add_plugins(bevy_yoleck::vpeol_2d::Vpeol2dPluginForGame);
    } else {
        app.add_plugins(EguiPlugin);
        app.add_plugins(YoleckPluginForEditor);
        // Adding `YoleckEditorLevelsDirectoryPath` is not usually required -
        // `YoleckPluginForEditor` will add one with "assets/levels". Here we want to support
        // example2d and example3d in the same repository so we use different directories.
        app.insert_resource(bevy_yoleck::YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels2d"),
        ));
        app.add_plugins(Vpeol2dPluginForEditor);
        app.add_plugins(VpeolSelectionCuePlugin::default());
        #[cfg(target_arch = "wasm32")]
        app.add_systems(
            Startup,
            |asset_server: Res<AssetServer>,
             mut yoleck_loading_command: ResMut<YoleckLoadingCommand>| {
                *yoleck_loading_command =
                    YoleckLoadingCommand::FromAsset(asset_server.load("levels2d/example.yol"));
            },
        );
    }
    app.add_systems(Startup, |world: &mut World| {
        world.init_resource::<GameAssets>();
    });

    app.add_plugins(YoleckEntityUpgradingPlugin {
        app_format_version: 1,
    });

    app.add_systems(Startup, setup_camera);

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Player")
            .with::<Vpeol2dPosition>()
            .with::<Vpeol2dRotatation>()
            .insert_on_init(|| IsPlayer)
    });
    app.add_yoleck_edit_system(edit_player);
    app.yoleck_populate_schedule_mut()
        .add_systems(populate_player);
    app.add_yoleck_entity_upgrade_for(1, "Player", |data| {
        let mut old_data = data.as_object_mut().unwrap().remove("Player").unwrap();
        data["Vpeol2dPosition"] = old_data.get_mut("position").unwrap().take();
    });

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Fruit")
            .with_uuid()
            .with::<Vpeol2dPosition>()
            .with::<FruitType>()
    });
    app.add_yoleck_edit_system(duplicate_fruit);
    app.add_yoleck_edit_system(edit_fruit_type);
    app.yoleck_populate_schedule_mut()
        .add_systems(populate_fruit);
    app.add_yoleck_entity_upgrade(1, |type_name, data| {
        if type_name != "Fruit" {
            return;
        }

        let mut old_data = data.as_object_mut().unwrap().remove("Fruit").unwrap();
        data["Vpeol2dPosition"] = old_data.get_mut("position").unwrap().take();
        data["FruitType"] = serde_json::json!({
            "index": old_data.get_mut("fruit_index").unwrap().take(),
        });
    });

    app.add_yoleck_entity_type({
        YoleckEntityType::new("FloatingText")
            .with::<Vpeol2dPosition>()
            .with::<Vpeol2dScale>()
            .with::<TextContent>()
            .with::<LaserPointer>()
    });
    app.add_yoleck_edit_system(edit_text);
    app.yoleck_populate_schedule_mut()
        .add_systems(populate_text);
    app.add_yoleck_entity_upgrade(1, |type_name, data| {
        if type_name != "FloatingText" {
            return;
        }

        let mut old_data = data
            .as_object_mut()
            .unwrap()
            .remove("FloatingText")
            .unwrap();
        data["Vpeol2dPosition"] = old_data.get_mut("position").unwrap().take();
        data["TextContent"] = serde_json::json!({
            "text": old_data.get_mut("text").unwrap().take(),
        });
        data["Vpeol2dScale"] = serde_json::to_value(
            Vec2::ONE * old_data.get_mut("scale").unwrap().take().as_f64().unwrap() as f32,
        )
        .unwrap();
    });

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Triangle")
            .with::<Vpeol2dPosition>()
            .with::<TriangleVertices>()
    });
    app.add_yoleck_edit_system(edit_triangle);
    app.yoleck_populate_schedule_mut()
        .add_systems(populate_triangle);

    app.add_yoleck_edit_system(edit_laser_pointer);
    app.add_systems(Update, draw_laser_pointers);

    app.add_systems(
        Update,
        (control_player, eat_fruits).run_if(in_state(YoleckEditorState::GameActive)),
    );
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

fn edit_player(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<(&IsPlayer, &Vpeol2dPosition, &mut Vpeol2dRotatation)>,
    mut knobs: YoleckKnobs,
) {
    let Ok((_, Vpeol2dPosition(position), mut rotation)) = edit.get_single_mut() else {
        return;
    };
    use std::f32::consts::PI;
    ui.add(egui::Slider::new(&mut rotation.0, PI..=-PI).prefix("Angle: "));
    // TODO: do this in vpeol_2d?
    let mut rotate_knob = knobs.knob("rotate");
    let knob_position = position.extend(1.0) + Quat::from_rotation_z(rotation.0) * (50.0 * Vec3::Y);
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
        rotation.0 = Vec2::Y.angle_between(rotate_to.truncate() - *position);
    }
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

#[derive(
    Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Component, YoleckComponent, Debug,
)]
struct FruitType {
    index: usize,
}

fn duplicate_fruit(
    mut ui: ResMut<YoleckUi>,
    edit: YoleckEdit<(&FruitType, &Vpeol2dPosition)>,
    mut writer: EventWriter<YoleckDirective>,
) {
    let Ok((fruit_type, Vpeol2dPosition(position))) = edit.get_single() else {
        return;
    };
    if ui.button("Duplicate").clicked() {
        writer.send(
            YoleckDirective::spawn_entity(
                "Fruit", true, // select_created_entity
            )
            .with(Vpeol2dPosition(*position - 100.0 * Vec2::Y))
            .with(FruitType {
                index: fruit_type.index,
            })
            .modify_exclusive_systems(|queue| queue.clear())
            .into(),
        );
    }
}

fn edit_fruit_type(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<(Entity, &mut FruitType, &Vpeol2dPosition)>,
    assets: Res<GameAssets>,
    mut knobs: YoleckKnobs,
) {
    if edit.is_empty() {
        return;
    }

    let (texture_id, rects) = &assets.fruits_sprite_sheet_egui;
    let mut selected_fruit_types = vec![false; rects.len()];
    for (entity, mut fruit_type, Vpeol2dPosition(position)) in edit.iter_matching_mut() {
        selected_fruit_types[fruit_type.index] = true;
        for index in 0..rects.len() {
            if index != fruit_type.index {
                let mut knob = knobs.knob((entity, "select", index));
                let knob_position =
                    (*position + Vec2::new(-30.0 + index as f32 * 30.0, 50.0)).extend(1.0);
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
                    fruit_type.index = index;
                }
            }
        }
    }
    if edit.has_nonmatching() {
        // Only show the UI if _all_ the selected entities are fruits
        return;
    }
    let selected_fruit_types = selected_fruit_types;
    let are_multile_types_selected = 1 < selected_fruit_types
        .iter()
        .filter(|is_selected| **is_selected)
        .count();

    ui.horizontal(|ui| {
        for (index, rect) in rects.iter().enumerate() {
            if ui
                .add_enabled(
                    are_multile_types_selected || !selected_fruit_types[index],
                    egui::ImageButton::new(egui::load::SizedTexture {
                        id: *texture_id,
                        size: egui::Vec2::new(100.0, 100.0),
                    })
                    .uv(*rect)
                    .selected(selected_fruit_types[index]),
                )
                .clicked()
            {
                for (_, mut fruit_type, _) in edit.iter_matching_mut() {
                    fruit_type.index = index;
                }
            }
        }
    });
}

fn populate_fruit(
    mut populate: YoleckPopulate<&FruitType>,
    assets: Res<GameAssets>,
    marking: YoleckMarking,
) {
    populate.populate(|_ctx, mut cmd, fruit| {
        marking.despawn_marked(&mut cmd);
        cmd.insert((
            VisibilityBundle::default(),
            VpeolWillContainClickableChildren,
            IsFruit,
        ));
        // Could have placed them on the main entity, but with this the children picking feature
        // can be tested and demonstrated.
        cmd.with_children(|commands| {
            let mut child = commands.spawn(marking.marker());
            child.insert(SpriteSheetBundle {
                sprite: TextureAtlasSprite {
                    index: fruit.index,
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

#[derive(Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
pub struct TextContent {
    text: String,
}

impl Default for TextContent {
    fn default() -> Self {
        Self {
            text: "<TEXT>".to_owned(),
        }
    }
}

fn populate_text(mut populate: YoleckPopulate<&TextContent>, assets: Res<GameAssets>) {
    populate.populate(|_ctx, mut cmd, content| {
        cmd.insert(Text2dBundle {
            text: {
                Text::from_section(
                    content.text.clone(),
                    TextStyle {
                        font: assets.font.clone(),
                        font_size: 72.0,
                        color: Color::WHITE,
                    },
                )
            },
            ..Default::default()
        });
    });
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

#[derive(Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
pub struct TriangleVertices {
    vertices: [Vec2; 3],
}

impl Default for TriangleVertices {
    fn default() -> Self {
        Self {
            vertices: [
                Vec2::new(-50.0, -50.0),
                Vec2::new(50.0, -50.0),
                Vec2::new(50.0, 50.0),
            ],
        }
    }
}

fn edit_triangle(
    mut edit: YoleckEdit<(&mut TriangleVertices, &GlobalTransform)>,
    mut knobs: YoleckKnobs,
) {
    let Ok((mut triangle, triangle_transform)) = edit.get_single_mut() else {
        return;
    };
    for (index, vertex) in triangle.vertices.iter_mut().enumerate() {
        let mut knob = knobs.knob(("move-vertex", index));
        if let Some(move_to) = knob.get_passed_data::<Vec3>() {
            *vertex = triangle_transform
                .compute_matrix()
                .inverse()
                .transform_point3(*move_to)
                .truncate();
        }
        let knob_position = triangle_transform.transform_point(vertex.extend(1.0));
        knob.cmd.insert(SpriteBundle {
            sprite: Sprite {
                color: Color::RED,
                custom_size: Some(Vec2::new(15.0, 15.0)),
                ..Default::default()
            },
            transform: Transform::from_translation(knob_position),
            global_transform: Transform::from_translation(knob_position).into(),
            ..Default::default()
        });
    }
}

fn populate_triangle(
    mut populate: YoleckPopulate<(&TriangleVertices, Option<&Mesh2dHandle>)>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<ColorMaterial>>,
) {
    populate.populate(|_ctx, mut cmd, (triangle, mesh2d)| {
        let mesh = if let Some(Mesh2dHandle(mesh_handle)) = mesh2d {
            mesh_assets
                .get_mut(mesh_handle)
                .expect("mesh inserted by previous invocation of this system")
        } else {
            let mesh_handle = mesh_assets.add(Mesh::new(PrimitiveTopology::TriangleList));
            let mesh = mesh_assets.get_mut(&mesh_handle);
            cmd.insert(ColorMesh2dBundle {
                mesh: Mesh2dHandle(mesh_handle),
                material: material_assets.add(Color::GREEN.into()),
                ..Default::default()
            });
            mesh.expect("mesh was just inserted")
        };
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            triangle
                .vertices
                .iter()
                .map(|point| point.extend(0.0).to_array())
                .collect::<Vec<_>>(),
        );
        let mut indices = Vec::new();
        for i in 1..(triangle.vertices.len() - 1) {
            let i = i as u32;
            indices.extend([0, i, i + 1]);
        }
        mesh.set_indices(Some(Indices::U32(indices)));
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
    let Ok(laser_pointer) = edit.get_single_mut() else {
        return;
    };

    let button = if let Some(target) = laser_pointer.target {
        ui.button(format!("{:?}", target))
    } else {
        ui.button("No Target")
    };
    if button.clicked() {
        exclusive_queue.push_back(pick_laser_pointer_target);
    }
}

fn pick_laser_pointer_target(
    mut edit: YoleckEdit<&mut LaserPointer>,
    cameras_query: Query<&VpeolCameraState>,
    ui: ResMut<YoleckUi>,
    buttons: Res<Input<MouseButton>>,
    uuid_query: Query<&YoleckEntityUuid>,
) -> YoleckExclusiveSystemDirective {
    let Ok(mut laser_pointer) = edit.get_single_mut() else {
        return YoleckExclusiveSystemDirective::Finished;
    };

    if ui.ctx().is_pointer_over_area() {
        return YoleckExclusiveSystemDirective::Listening;
    }

    let Some(target) = cameras_query
        .iter()
        .find_map(|camera_state| Some(camera_state.entity_under_cursor.as_ref()?.0))
    else {
        return YoleckExclusiveSystemDirective::Listening;
    };

    let Ok(uuid) = uuid_query.get(target) else {
        return YoleckExclusiveSystemDirective::Listening;
    };

    if buttons.just_released(MouseButton::Left) {
        laser_pointer.target = Some(**uuid);
        return YoleckExclusiveSystemDirective::Finished;
    }

    YoleckExclusiveSystemDirective::Listening
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
            Color::GREEN,
        );
    }
}
