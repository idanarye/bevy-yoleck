//! # Viewport Editing Overlay for 2D games.
//!
//! Use this module to implement simple 2D editing for 2D games.
//!
//! To use add the egui and Yoleck plugins to the Bevy app, as well as the plugin of this module:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoleck::bevy_egui::EguiPlugin;
//! # use bevy_yoleck::prelude::*;
//! # use bevy_yoleck::vpeol::prelude::*;
//! # let mut app = App::new();
//! app.add_plugin(EguiPlugin);
//! app.add_plugin(YoleckPluginForEditor);
//! // Use `Vpeol2dPluginForGame` instead when setting up for game.
//! app.add_plugin(Vpeol2dPluginForEditor);
//! ```
//!
//! Add the following components to the camera entity:
//! * [`VpeolCameraState`] in order to select and drag entities.
//! * [`Vpeol2dCameraControl`] in order to pan and zoom the camera with the mouse. This one can be
//!   skipped if there are other means to control the camera inside the editor, or if no camera
//!   control inside the editor is desired.
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoleck::vpeol::VpeolCameraState;
//! # use bevy_yoleck::vpeol::prelude::*;
//! # let commands: Commands = panic!();
//! commands
//!     .spawn(Camera2dBundle::default())
//!     .insert(VpeolCameraState::default())
//!     .insert(Vpeol2dCameraControl::default());
//! ```
//!
//! Entity selection by clicking on it is supported by just adding the plugin. To implement
//! dragging, there are two options:
//!
//! 1. Add  the [`Vpeol2dPosition`] Yoleck component and use it as the source of position (there
//!    are also [`Vpeol2dRotatation`] and [`Vpeol2dScale`], but they don't currently get editing
//!    support from vpeol_2d)
//!     ```no_run
//!     # use bevy::prelude::*;
//!     # use bevy_yoleck::prelude::*;
//!     # use bevy_yoleck::vpeol::prelude::*;
//!     # use serde::{Deserialize, Serialize};
//!     # #[derive(Clone, PartialEq, Serialize, Deserialize, Component, Default, YoleckComponent)]
//!     # struct Example;
//!     # let mut app = App::new();
//!     app.add_yoleck_entity_type({
//!         YoleckEntityType::new("Example")
//!             .with::<Vpeol2dPosition>() // vpeol_2d dragging
//!             .with::<Example>() // entity's specific data and systems
//!     });
//!     ```
//! 2. Use data passing. vpeol_2d will pass a `Vec3` to the entity being dragged:
//!     ```no_run
//!     # use bevy::prelude::*;
//!     # use bevy_yoleck::prelude::*;
//!     # use serde::{Deserialize, Serialize};
//!     # #[derive(Clone, PartialEq, Serialize, Deserialize, Component, Default, YoleckComponent)]
//!     # struct Example {
//!     #     position: Vec2,
//!     # }
//!     # let mut app = App::new();
//!     fn edit_example(mut edit: YoleckEdit<(Entity, &mut Example)>, passed_data: Res<YoleckPassedData>) {
//!         let Ok((entity, mut example)) = edit.get_single_mut() else { return };
//!         if let Some(pos) = passed_data.get::<Vec3>(entity) {
//!             example.position = pos.truncate();
//!         }
//!     }
//!
//!     fn populate_example(mut populate: YoleckPopulate<&Example>) {
//!         populate.populate(|_ctx, mut cmd, example| {
//!             cmd.insert(SpriteBundle {
//!                 transform: Transform::from_translation(example.position.extend(0.0)),
//!                 // Actual sprite components
//!                 ..Default::default()
//!             });
//!         });
//!     }
//!     ```

use crate::bevy_egui::{egui, EguiContexts};
use crate::vpeol::{
    handle_clickable_children_system, ray_intersection_with_mesh, VpeolBasePlugin,
    VpeolCameraState, VpeolDragPlane, VpeolRootResolver, VpeolSystemSet, WindowGetter,
};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::view::VisibleEntities;
use bevy::sprite::{Anchor, Mesh2dHandle};
use bevy::text::TextLayoutInfo;
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};

use crate::{prelude::*, YoleckPopulateBaseSet};

/// Add the systems required for loading levels that use vpeol_2d components
pub struct Vpeol2dPluginForGame;

impl Plugin for Vpeol2dPluginForGame {
    fn build(&self, app: &mut App) {
        app.yoleck_populate_schedule_mut().add_system(
            vpeol_2d_populate_transform
                .in_base_set(YoleckPopulateBaseSet::OverrideCommonComponents),
        );
    }
}

/// Add the systems required for 2D editing.
///
/// * 2D camera control (for cameras with [`Vpeol2dCameraControl`])
/// * Entity selection.
/// * Entity dragging.
/// * Connecting nested entities.
pub struct Vpeol2dPluginForEditor;

impl Plugin for Vpeol2dPluginForEditor {
    fn build(&self, app: &mut App) {
        app.add_plugin(VpeolBasePlugin);
        app.add_plugin(Vpeol2dPluginForGame);
        app.insert_resource(VpeolDragPlane { normal: Vec3::Z });

        app.add_systems(
            (
                update_camera_status_for_sprites,
                update_camera_status_for_atlas_sprites,
                update_camera_status_for_2d_meshes,
                update_camera_status_for_text_2d,
            )
                .in_set(VpeolSystemSet::UpdateCameraState),
        );
        app.add_systems(
            (camera_2d_pan, camera_2d_zoom).in_set(OnUpdate(YoleckEditorState::EditorActive)),
        );
        app.add_systems(
            (
                apply_system_buffers,
                handle_clickable_children_system::<
                    Or<(
                        (With<Sprite>, With<Handle<Image>>),
                        (With<TextureAtlasSprite>, With<Handle<TextureAtlas>>),
                        (With<TextLayoutInfo>, With<Anchor>),
                    )>,
                    (),
                >,
                apply_system_buffers,
            )
                .chain()
                .in_set(OnUpdate(YoleckEditorState::EditorActive)),
        );
        app.add_yoleck_edit_system(vpeol_2d_edit_position);
    }
}

struct CursorInWorldPos {
    cursor_in_world_pos: Vec2,
}

impl CursorInWorldPos {
    fn from_camera_state(camera_state: &VpeolCameraState) -> Option<Self> {
        Some(Self {
            cursor_in_world_pos: camera_state.cursor_ray?.origin.truncate(),
        })
    }

    fn cursor_in_entity_space(&self, transform: &GlobalTransform) -> Vec2 {
        transform
            .compute_matrix()
            .inverse()
            .project_point3(self.cursor_in_world_pos.extend(0.0))
            .truncate()
    }

    fn check_square(
        &self,
        entity_transform: &GlobalTransform,
        anchor: &Anchor,
        size: Vec2,
    ) -> bool {
        let cursor = self.cursor_in_entity_space(entity_transform);
        let anchor = anchor.as_vec();
        let mut min_corner = Vec2::new(-0.5, -0.5) - anchor;
        let mut max_corner = Vec2::new(0.5, 0.5) - anchor;
        for corner in [&mut min_corner, &mut max_corner] {
            corner.x *= size.x;
            corner.y *= size.y;
        }
        min_corner.x <= cursor.x
            && cursor.x <= max_corner.x
            && min_corner.y <= cursor.y
            && cursor.y <= max_corner.y
    }
}

fn update_camera_status_for_sprites(
    mut cameras_query: Query<(&mut VpeolCameraState, &VisibleEntities)>,
    entities_query: Query<(Entity, &GlobalTransform, &Sprite, &Handle<Image>)>,
    image_assets: Res<Assets<Image>>,
    root_resolver: VpeolRootResolver,
) {
    for (mut camera_state, visible_entities) in cameras_query.iter_mut() {
        let Some(cursor) = CursorInWorldPos::from_camera_state(&camera_state) else { continue };

        for (entity, entity_transform, sprite, texture) in
            entities_query.iter_many(&visible_entities.entities)
        {
            let size = if let Some(custom_size) = sprite.custom_size {
                custom_size
            } else if let Some(texture) = image_assets.get(texture) {
                texture.size()
            } else {
                continue;
            };
            if cursor.check_square(entity_transform, &sprite.anchor, size) {
                let z_depth = entity_transform.translation().z;
                let Some(root_entity) = root_resolver.resolve_root(entity) else { continue };
                camera_state.consider(root_entity, z_depth, || {
                    cursor.cursor_in_world_pos.extend(z_depth)
                });
            }
        }
    }
}

fn update_camera_status_for_atlas_sprites(
    mut cameras_query: Query<(&mut VpeolCameraState, &VisibleEntities)>,
    entities_query: Query<(
        Entity,
        &GlobalTransform,
        &TextureAtlasSprite,
        &Handle<TextureAtlas>,
    )>,
    texture_atlas_assets: Res<Assets<TextureAtlas>>,
    root_resolver: VpeolRootResolver,
) {
    for (mut camera_state, visible_entities) in cameras_query.iter_mut() {
        let Some(cursor) = CursorInWorldPos::from_camera_state(&camera_state) else { continue };

        for (entity, entity_transform, sprite, texture) in
            entities_query.iter_many(&visible_entities.entities)
        {
            let size = if let Some(custom_size) = sprite.custom_size {
                custom_size
            } else if let Some(texture_atlas) = texture_atlas_assets.get(texture) {
                texture_atlas.textures[sprite.index].size()
            } else {
                continue;
            };
            if cursor.check_square(entity_transform, &sprite.anchor, size) {
                let z_depth = entity_transform.translation().z;
                let Some(root_entity) = root_resolver.resolve_root(entity) else { continue };
                camera_state.consider(root_entity, z_depth, || {
                    cursor.cursor_in_world_pos.extend(z_depth)
                });
            }
        }
    }
}

fn update_camera_status_for_2d_meshes(
    mut cameras_query: Query<(&mut VpeolCameraState, &VisibleEntities)>,
    entities_query: Query<(Entity, &GlobalTransform, &Mesh2dHandle)>,
    mesh_assets: Res<Assets<Mesh>>,
    root_resolver: VpeolRootResolver,
) {
    for (mut camera_state, visible_entities) in cameras_query.iter_mut() {
        let Some(cursor_ray) = camera_state.cursor_ray else { continue };
        for (entity, global_transform, mesh) in entities_query.iter_many(&visible_entities.entities)
        {
            let Some(mesh) = mesh_assets.get(&mesh.0) else { continue };

            let inverse_transform = global_transform.compute_matrix().inverse();

            let ray_in_object_coords = Ray {
                origin: inverse_transform.transform_point3(cursor_ray.origin),
                direction: inverse_transform.transform_vector3(cursor_ray.direction),
            };

            let Some(distance) = ray_intersection_with_mesh(ray_in_object_coords, &mesh) else { continue };

            let Some(root_entity) = root_resolver.resolve_root(entity) else { continue };
            camera_state.consider(root_entity, -distance, || cursor_ray.get_point(distance));
        }
    }
}

fn update_camera_status_for_text_2d(
    mut cameras_query: Query<(&mut VpeolCameraState, &VisibleEntities)>,
    entities_query: Query<(Entity, &GlobalTransform, &TextLayoutInfo, &Anchor)>,
    root_resolver: VpeolRootResolver,
) {
    for (mut camera_state, visible_entities) in cameras_query.iter_mut() {
        let Some(cursor) = CursorInWorldPos::from_camera_state(&camera_state) else { continue };

        for (entity, entity_transform, text_layout_info, anchor) in
            entities_query.iter_many(&visible_entities.entities)
        {
            if cursor.check_square(entity_transform, anchor, text_layout_info.size) {
                let z_depth = entity_transform.translation().z;
                let Some(root_entity) = root_resolver.resolve_root(entity) else { continue };
                camera_state.consider(root_entity, z_depth, || {
                    cursor.cursor_in_world_pos.extend(z_depth)
                });
            }
        }
    }
}

/// Pan and zoom a camera entity with the mouse while inisde the editor.
#[derive(Component)]
pub struct Vpeol2dCameraControl {
    /// How much to zoom when receiving scroll event in `MouseScrollUnit::Line` units.
    pub zoom_per_scroll_line: f32,
    /// How much to zoom when receiving scroll event in `MouseScrollUnit::Pixel` units.
    pub zoom_per_scroll_pixel: f32,
}

impl Default for Vpeol2dCameraControl {
    fn default() -> Self {
        Self {
            zoom_per_scroll_line: 0.2,
            zoom_per_scroll_pixel: 0.001,
        }
    }
}

fn camera_2d_pan(
    mut egui_context: EguiContexts,
    window_getter: WindowGetter,
    buttons: Res<Input<MouseButton>>,
    mut cameras_query: Query<
        (Entity, &mut Transform, &GlobalTransform, &Camera),
        With<Vpeol2dCameraControl>,
    >,
    mut last_cursor_world_pos_by_camera: Local<HashMap<Entity, Vec2>>,
) {
    enum MouseButtonOp {
        JustPressed,
        BeingPressed,
    }

    let mouse_button_op = if buttons.just_pressed(MouseButton::Right) {
        if egui_context.ctx_mut().is_pointer_over_area() {
            return;
        }
        MouseButtonOp::JustPressed
    } else if buttons.pressed(MouseButton::Right) {
        MouseButtonOp::BeingPressed
    } else {
        last_cursor_world_pos_by_camera.clear();
        return;
    };

    for (camera_entity, mut camera_transform, camera_global_transform, camera) in
        cameras_query.iter_mut()
    {
        let window = if let RenderTarget::Window(window_ref) = camera.target {
            window_getter.get_window(window_ref).unwrap()
        } else {
            continue;
        };
        if let Some(screen_pos) = window.cursor_position() {
            let world_pos = screen_pos_to_world_pos(
                screen_pos,
                window,
                &camera_global_transform.compute_matrix(),
                camera,
            );

            match mouse_button_op {
                MouseButtonOp::JustPressed => {
                    last_cursor_world_pos_by_camera.insert(camera_entity, world_pos);
                }
                MouseButtonOp::BeingPressed => {
                    if let Some(prev_pos) = last_cursor_world_pos_by_camera.get_mut(&camera_entity)
                    {
                        let movement = *prev_pos - world_pos;
                        camera_transform.translation += movement.extend(0.0);
                    }
                }
            }
        }
    }
}

fn camera_2d_zoom(
    mut egui_context: EguiContexts,
    window_getter: WindowGetter,
    mut cameras_query: Query<(
        &mut Transform,
        &GlobalTransform,
        &Camera,
        &Vpeol2dCameraControl,
    )>,
    mut wheel_events_reader: EventReader<MouseWheel>,
) {
    if egui_context.ctx_mut().is_pointer_over_area() {
        return;
    }

    for (mut camera_transform, camera_global_transform, camera, camera_control) in
        cameras_query.iter_mut()
    {
        let zoom_amount: f32 = wheel_events_reader
            .iter()
            .map(|wheel_event| match wheel_event.unit {
                bevy::input::mouse::MouseScrollUnit::Line => {
                    wheel_event.y * camera_control.zoom_per_scroll_line
                }
                bevy::input::mouse::MouseScrollUnit::Pixel => {
                    wheel_event.y * camera_control.zoom_per_scroll_pixel
                }
            })
            .sum();

        if zoom_amount == 0.0 {
            continue;
        }

        let scale_by = (-zoom_amount).exp();

        let window = if let RenderTarget::Window(window_ref) = camera.target {
            window_getter.get_window(window_ref).unwrap()
        } else {
            continue;
        };
        if let Some(screen_pos) = window.cursor_position() {
            let camera_global_transform_matrix = camera_global_transform.compute_matrix();
            let world_pos = screen_pos_to_world_pos(
                screen_pos,
                window,
                &camera_global_transform_matrix,
                camera,
            );
            camera_transform.scale.x *= scale_by;
            camera_transform.scale.y *= scale_by;
            let new_global_transform_matrix = camera_global_transform_matrix
                .mul_mat4(&Mat4::from_scale(Vec3::new(scale_by, scale_by, 1.0)));
            let new_world_pos =
                screen_pos_to_world_pos(screen_pos, window, &new_global_transform_matrix, camera);
            camera_transform.translation += (world_pos - new_world_pos).extend(0.0);
        }
    }
}

fn screen_pos_to_world_pos(
    screen_pos: Vec2,
    wnd: &Window,
    camera_transform_matrix: &Mat4,
    camera: &Camera,
) -> Vec2 {
    // Code stolen from https://bevy-cheatbook.github.io/cookbook/cursor2world.html

    // get the size of the window
    let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

    // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
    let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

    // matrix for undoing the projection and camera transform
    let ndc_to_world = camera_transform_matrix.mul_mat4(&camera.projection_matrix().inverse());

    // use it to convert ndc to world-space coordinates
    let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

    // reduce it to a 2D value
    world_pos.truncate()
}

/// A position component that's edited and populated by vpeol_2d.
#[derive(Clone, PartialEq, Serialize, Deserialize, Component, Default, YoleckComponent)]
#[serde(transparent)]
pub struct Vpeol2dPosition(pub Vec2);

/// A rotation component that's populated (but not edited) by vpeol_2d.
///
/// The rotation is in radians around the Z axis.
#[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
#[serde(transparent)]
pub struct Vpeol2dRotatation(pub f32);

/// A scale component that's populated (but not edited) by vpeol_2d.
#[derive(Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
#[serde(transparent)]
pub struct Vpeol2dScale(pub Vec2);

impl Default for Vpeol2dScale {
    fn default() -> Self {
        Self(Vec2::ONE)
    }
}

fn vpeol_2d_edit_position(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<(Entity, &mut Vpeol2dPosition)>,
    passed_data: Res<YoleckPassedData>,
) {
    let Ok((entity, mut position)) = edit.get_single_mut() else { return };
    if let Some(pos) = passed_data.get::<Vec3>(entity) {
        position.0 = pos.truncate();
    }
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut position.0.x).prefix("X:"));
        ui.add(egui::DragValue::new(&mut position.0.y).prefix("Y:"));
    });
}

fn vpeol_2d_populate_transform(
    mut populate: YoleckPopulate<(
        &Vpeol2dPosition,
        Option<&Vpeol2dRotatation>,
        Option<&Vpeol2dScale>,
    )>,
) {
    populate.populate(|_ctx, mut cmd, (position, rotation, scale)| {
        let mut transform = Transform::from_translation(position.0.extend(0.0));
        if let Some(Vpeol2dRotatation(rotation)) = rotation {
            transform = transform.with_rotation(Quat::from_rotation_z(*rotation));
        }
        if let Some(Vpeol2dScale(scale)) = scale {
            transform = transform.with_scale(scale.extend(1.0));
        }
        cmd.insert(TransformBundle {
            local: transform,
            global: transform.into(),
        });
    })
}
