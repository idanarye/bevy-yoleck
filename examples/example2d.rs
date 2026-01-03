use std::path::Path;

use bevy::{asset::RenderAssetUsages, log::LogPlugin};
use bevy::color::palettes::css;
use bevy::ecs::system::SystemState;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

use bevy_yoleck::prelude::*;
use bevy_yoleck::vpeol::prelude::*;
use bevy_yoleck::YoleckDirective;
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
        app.add_plugins(bevy_yoleck::vpeol_2d::Vpeol2dPluginForGame);
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                commands.spawn(YoleckLoadLevel(
                    asset_server.load(Path::new("levels2d").join(&level)),
                ));
            },
        );
    } else {
        app.add_plugins(EguiPlugin::default());
        app.add_plugins(YoleckPluginForEditor);
        app.insert_resource(bevy_yoleck::YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels2d"),
        ));
        app.add_plugins(Vpeol2dPluginForEditor);
        app.add_plugins(VpeolSelectionCuePlugin::default());
        #[cfg(target_arch = "wasm32")]
        app.add_systems(
            Startup,
            |asset_server: Res<AssetServer>, mut commands: Commands| {
                commands.spawn(YoleckLoadLevel(asset_server.load("levels2d/example.yol")));
            },
        );
    }

    app.add_plugins(YoleckEntityUpgradingPlugin {
        app_format_version: 1,
    });

    app.add_systems(Startup, (setup_camera, setup_assets));

    // ========================================================================
    // Player
    // ========================================================================

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Player")
            .with::<Vpeol2dPosition>()
            .with::<Vpeol2dRotatation>()
            .insert_on_init(|| IsPlayer)
    });
    app.add_yoleck_edit_system(edit_player);
    app.add_systems(YoleckSchedule::Populate, populate_player);
    app.add_yoleck_entity_upgrade_for(1, "Player", |data| {
        let mut old_data = data.as_object_mut().unwrap().remove("Player").unwrap();
        data["Vpeol2dPosition"] = old_data.get_mut("position").unwrap().take();
    });

    // ========================================================================
    // Fruit
    // ========================================================================

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Fruit")
            .with_uuid()
            .with::<Vpeol2dPosition>()
            .with::<FruitType>()
    });
    app.add_yoleck_edit_system(duplicate_fruit);
    app.add_yoleck_edit_system(edit_fruit_type);
    app.add_systems(YoleckSchedule::Populate, populate_fruit);
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

    // ========================================================================
    // FloatingText
    // ========================================================================

    app.add_yoleck_entity_type({
        YoleckEntityType::new("FloatingText")
            .with_uuid()
            .with::<Vpeol2dPosition>()
            .with::<Vpeol2dScale>()
            .with::<TextContent>()
            .with::<TextLaserPointer>()
    });
    app.add_yoleck_auto_edit::<TextContent>();
    app.add_yoleck_auto_edit::<TextLaserPointer>();
    app.add_systems(YoleckSchedule::Populate, populate_text);
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

    // ========================================================================
    // Triangle
    // ========================================================================

    app.add_yoleck_entity_type({
        YoleckEntityType::new("Triangle")
            .with::<Vpeol2dPosition>()
            .with::<TriangleVertices>()
    });
    app.add_yoleck_edit_system(edit_triangle);
    app.add_systems(YoleckSchedule::Populate, populate_triangle);

    // ========================================================================
    // Common systems
    // ========================================================================

    app.add_systems(Update, (resolve_laser_pointers, draw_laser_pointers));
    app.add_systems(
        Update,
        (control_player, eat_fruits).run_if(in_state(YoleckEditorState::GameActive)),
    );

    app.run();
}

// ============================================================================
// Setup
// ============================================================================

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Transform::from_xyz(0.0, 0.0, 100.0),
        VpeolCameraState::default(),
        Vpeol2dCameraControl::default(),
    ));
}

fn setup_assets(world: &mut World) {
    world.init_resource::<GameAssets>();
}

#[derive(Resource)]
struct GameAssets {
    fruits_sprite_sheet_texture: Handle<Image>,
    fruits_sprite_sheet_layout: Handle<TextureAtlasLayout>,
    fruits_sprite_sheet_egui: (egui::TextureId, Vec<egui::Rect>),
    font: Handle<Font>,
}

impl FromWorld for GameAssets {
    fn from_world(world: &mut World) -> Self {
        let mut system_state = SystemState::<(
            Res<AssetServer>,
            ResMut<Assets<TextureAtlasLayout>>,
            EguiContexts,
        )>::new(world);
        let (asset_server, mut texture_atlas_layout_assets, mut egui_context) =
            system_state.get_mut(world);
        let fruits_atlas_texture = asset_server.load("sprites/fruits.png");
        let fruits_atlas_layout =
            TextureAtlasLayout::from_grid(UVec2::new(64, 64), 3, 1, None, None);
        let fruits_egui = {
            (
                egui_context.add_image(bevy_egui::EguiTextureHandle::Strong(
                    fruits_atlas_texture.clone(),
                )),
                fruits_atlas_layout
                    .textures
                    .iter()
                    .map(|rect| {
                        [
                            [
                                rect.min.x as f32 / fruits_atlas_layout.size.x as f32,
                                rect.min.y as f32 / fruits_atlas_layout.size.y as f32,
                            ]
                            .into(),
                            [
                                rect.max.x as f32 / fruits_atlas_layout.size.x as f32,
                                rect.max.y as f32 / fruits_atlas_layout.size.y as f32,
                            ]
                            .into(),
                        ]
                        .into()
                    })
                    .collect(),
            )
        };
        Self {
            fruits_sprite_sheet_texture: fruits_atlas_texture,
            fruits_sprite_sheet_layout: texture_atlas_layout_assets.add(fruits_atlas_layout),
            fruits_sprite_sheet_egui: fruits_egui,
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        }
    }
}

// ============================================================================
// Player
// ============================================================================

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

fn edit_player(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<(&IsPlayer, &Vpeol2dPosition, &mut Vpeol2dRotatation)>,
    mut knobs: YoleckKnobs,
) {
    let Ok((_, Vpeol2dPosition(position), mut rotation)) = edit.single_mut() else {
        return;
    };
    use std::f32::consts::PI;
    ui.add(egui::Slider::new(&mut rotation.0, PI..=-PI).prefix("Angle: "));
    let mut rotate_knob = knobs.knob("rotate");
    let knob_position = position.extend(1.0) + Quat::from_rotation_z(rotation.0) * (50.0 * Vec3::Y);
    rotate_knob.cmd.insert((
        Sprite::from_color(css::PURPLE, Vec2::new(30.0, 30.0)),
        Transform::from_translation(knob_position),
        GlobalTransform::from(Transform::from_translation(knob_position)),
    ));
    if let Some(rotate_to) = rotate_knob.get_passed_data::<Vec3>() {
        rotation.0 = Vec2::Y.angle_to(rotate_to.truncate() - *position);
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
    velocity *= 400.0;
    for mut player_transform in player_query.iter_mut() {
        player_transform.translation += velocity * time.delta_secs();
    }
}

// ============================================================================
// Fruit
// ============================================================================

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
    edit: YoleckEdit<(&YoleckBelongsToLevel, &FruitType, &Vpeol2dPosition)>,
    mut writer: MessageWriter<YoleckDirective>,
) {
    let Ok((belongs_to_level, fruit_type, Vpeol2dPosition(position))) = edit.single() else {
        return;
    };
    if ui.button("Duplicate").clicked() {
        writer.write(
            YoleckDirective::spawn_entity(
                belongs_to_level.level,
                "Fruit",
                true,
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
                knob.cmd.insert((
                    Sprite {
                        image: assets.fruits_sprite_sheet_texture.clone(),
                        texture_atlas: Some(TextureAtlas {
                            layout: assets.fruits_sprite_sheet_layout.clone(),
                            index,
                        }),
                        custom_size: Some(Vec2::new(20.0, 20.0)),
                        ..Default::default()
                    },
                    Transform::from_translation(knob_position),
                    GlobalTransform::from(Transform::from_translation(knob_position)),
                ));
                if knob.get_passed_data::<YoleckKnobClick>().is_some() {
                    fruit_type.index = index;
                }
            }
        }
    }
    if edit.has_nonmatching() {
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
                    egui::Button::image(
                        egui::Image::new(egui::load::SizedTexture {
                            id: *texture_id,
                            size: egui::Vec2::new(100.0, 100.0),
                        })
                        .uv(*rect),
                    )
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
            Visibility::default(),
            VpeolWillContainClickableChildren,
            IsFruit,
        ));
        cmd.with_children(|commands| {
            let mut child = commands.spawn(marking.marker());
            child.insert((Sprite {
                image: assets.fruits_sprite_sheet_texture.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: assets.fruits_sprite_sheet_layout.clone(),
                    index: fruit.index,
                }),
                custom_size: Some(Vec2::new(100.0, 100.0)),
                ..Default::default()
            },));
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
                commands.entity(fruit_entity).despawn();
            }
        }
    }
}

// ============================================================================
// FloatingText
// ============================================================================

#[derive(
    Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent, YoleckAutoEdit,
)]
pub struct TextContent {
    #[yoleck(multiline)]
    text: String,
}

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
struct TextLaserPointer {
    #[yoleck(entity_ref = "Fruit")]
    target: YoleckEntityRef,
}

fn populate_text(mut populate: YoleckPopulate<&TextContent>, assets: Res<GameAssets>) {
    populate.populate(|_ctx, mut cmd, content| {
        cmd.insert((
            Text2d(content.text.clone()),
            TextFont {
                font: assets.font.clone(),
                font_size: 72.0,
                ..Default::default()
            },
        ));
    });
}

// ============================================================================
// Triangle
// ============================================================================

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
    let Ok((mut triangle, triangle_transform)) = edit.single_mut() else {
        return;
    };
    for (index, vertex) in triangle.vertices.iter_mut().enumerate() {
        let mut knob = knobs.knob(("move-vertex", index));
        if let Some(move_to) = knob.get_passed_data::<Vec3>() {
            *vertex = triangle_transform
                .to_matrix()
                .inverse()
                .transform_point3(*move_to)
                .truncate();
        }
        let knob_position = triangle_transform.transform_point(vertex.extend(1.0));
        knob.cmd.insert((
            Sprite::from_color(css::RED, Vec2::new(15.0, 15.0)),
            Transform::from_translation(knob_position),
            GlobalTransform::from(Transform::from_translation(knob_position)),
        ));
    }
}

fn populate_triangle(
    mut populate: YoleckPopulate<(&TriangleVertices, Option<&Mesh2d>)>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<ColorMaterial>>,
) {
    populate.populate(|_ctx, mut cmd, (triangle, mesh2d)| {
        let mesh = if let Some(Mesh2d(mesh_handle)) = mesh2d {
            mesh_assets
                .get_mut(mesh_handle)
                .expect("mesh inserted by previous invocation of this system")
        } else {
            let mesh_handle = mesh_assets.add(Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            ));
            let mesh = mesh_assets.get_mut(&mesh_handle);
            cmd.insert((
                Mesh2d(mesh_handle),
                MeshMaterial2d(material_assets.add(Color::from(css::GREEN))),
            ));
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
        mesh.insert_indices(Indices::U32(indices));
    });
}

// ============================================================================
// LaserPointer (shared)
// ============================================================================

fn resolve_laser_pointers(
    mut query: Query<&mut TextLaserPointer>,
    uuid_registry: Res<YoleckUuidRegistry>,
) {
    for mut laser_pointer in query.iter_mut() {
        laser_pointer.target.resolve(&uuid_registry);
    }
}

fn draw_laser_pointers(
    query: Query<(&TextLaserPointer, &GlobalTransform)>,
    targets_query: Query<&GlobalTransform>,
    mut gizmos: Gizmos,
) {
    for (laser_pointer, source_transform) in query.iter() {
        if let Some(target_entity) = laser_pointer.target.get_entity() {
            if let Ok(target_transform) = targets_query.get(target_entity) {
                gizmos.line(
                    source_transform.translation(),
                    target_transform.translation(),
                    css::GREEN,
                );
            }
        }
    }
}
