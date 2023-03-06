//! # Viewport Editing Overlay for 2D games.
//!
//! Use this module to implement simple 2D editing for 2D games.
//!
//! To use add the egui and Yoleck plugins to the Bevy app, as well as the plugin of this module:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoleck::bevy_egui::EguiPlugin;
//! # use bevy_yoleck::YoleckPluginForEditor;
//! # use bevy_yoleck::vpeol_2d::Vpeol2dPlugin;
//! # let mut app = App::new();
//! app.add_plugin(EguiPlugin);
//! app.add_plugin(YoleckPluginForEditor);
//! app.add_plugin(Vpeol2dPlugin);
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
//! # use bevy_yoleck::vpeol_2d::Vpeol2dCameraControl;
//! # let commands: Commands = panic!();
//! commands
//!     .spawn(Camera2dBundle::default())
//!     .insert(VpeolCameraState::default())
//!     .insert(Vpeol2dCameraControl::default());
//! ```
//!
//! Entity selection by clicking on it is supported by just adding the plugin. To implement
//! dragging, there are two options. Either use the passed data:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoleck::{YoleckTypeHandler, YoleckExtForApp, YoleckEdit, YoleckPopulate};
//! # use serde::{Deserialize, Serialize};
//! # #[derive(Clone, PartialEq, Serialize, Deserialize)]
//! # struct Example {
//! #     position: Vec2,
//! # }
//! # let mut app = App::new();
//! app.add_yoleck_handler({
//!     YoleckTypeHandler::<Example>::new("Example")
//!         .edit_with(edit_example)
//!         .populate_with(populate_example)
//! });
//!
//! fn edit_example(mut edit: YoleckEdit<Example>) {
//!     edit.edit(|ctx, data, _ui| {
//!         if let Some(pos) = ctx.get_passed_data::<Vec3>() {
//!             data.position = pos.truncate();
//!         }
//!     });
//! }
//!
//! fn populate_example(mut populate: YoleckPopulate<Example>) {
//!     populate.populate(|_ctx, data, mut cmd| {
//!         cmd.insert_bundle(SpriteBundle {
//!             transform: Transform::from_translation(data.position.extend(0.0)),
//!             // Actual sprite components
//!             ..Default::default()
//!         });
//!     });
//! }
//! ```
//!
//! Alternatively, use [`vpeol_position_edit_adapter`].

use crate::bevy_egui::{egui, EguiContext};
use crate::vpeol::{
    handle_clickable_children_system, VpeolBasePlugin, VpeolCameraState, VpeolRootResolver,
    VpeolSystemLabel,
};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::sprite::Anchor;
use bevy::text::Text2dSize;
use bevy::utils::HashMap;

use crate::{YoleckEdit, YoleckEditorState, YoleckTypeHandler};

/// Add the systems required for 2D editing.
///
/// * 2D camera zoom/pan
/// * Entity selection.
/// * Entity dragging.
/// * Connecting nested entities.
pub struct Vpeol2dPlugin;

impl Plugin for Vpeol2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(VpeolBasePlugin);
        app.add_system_set({
            SystemSet::on_update(YoleckEditorState::EditorActive)
                .label(VpeolSystemLabel::PrepareCameraState)
                .before(VpeolSystemLabel::UpdateCameraState)
                .before(VpeolSystemLabel::HandleCameraState)
                .with_system(update_camera_world_position)
        });

        app.add_system_set({
            SystemSet::on_update(YoleckEditorState::EditorActive)
                .label(VpeolSystemLabel::UpdateCameraState)
                .with_system(update_camera_status_for_sprites)
                .with_system(update_camera_status_for_atlas_sprites)
                .with_system(update_camera_status_for_text_2d)
        });
        app.add_system_set({
            SystemSet::on_update(YoleckEditorState::EditorActive)
                .with_system(camera_2d_pan)
                .with_system(camera_2d_zoom)
                .with_system(
                    handle_clickable_children_system::<
                        Or<(
                            (With<Sprite>, With<Handle<Image>>),
                            (With<TextureAtlasSprite>, With<Handle<TextureAtlas>>),
                            With<Text2dSize>,
                        )>,
                        (),
                    >,
                )
        });
    }
}

struct CursorInWorldPos {
    cursor_in_world_pos: Vec2,
}

impl CursorInWorldPos {
    fn from_camera_state(camera_state: &VpeolCameraState) -> Option<Self> {
        Some(Self {
            cursor_in_world_pos: camera_state.cursor_in_world_position?.truncate(),
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

fn update_camera_world_position(
    mut cameras_query: Query<
        (&mut VpeolCameraState, &GlobalTransform, &Camera),
        With<OrthographicProjection>,
    >,
    windows: Res<Windows>,
) {
    for (mut camera_state, camera_transform, camera) in cameras_query.iter_mut() {
        camera_state.cursor_in_world_position = (|| {
            let RenderTarget::Window(window_id) = camera.target else { return None };
            let window = windows.get(window_id)?;
            let cursor_in_screen_pos = window.cursor_position()?;
            Some(
                screen_pos_to_world_pos(
                    cursor_in_screen_pos,
                    window,
                    &camera_transform.compute_matrix(),
                    camera,
                )
                .extend(0.0),
            )
        })();
    }
}

fn update_camera_status_for_sprites(
    mut cameras_query: Query<&mut VpeolCameraState>,
    entities_query: Query<(Entity, &GlobalTransform, &Sprite, &Handle<Image>)>,
    image_assets: Res<Assets<Image>>,
    root_resolver: VpeolRootResolver,
) {
    for mut camera_state in cameras_query.iter_mut() {
        let Some(cursor) = CursorInWorldPos::from_camera_state(&camera_state) else { continue };

        for (entity, entity_transform, sprite, texture) in entities_query.iter() {
            let size = if let Some(custom_size) = sprite.custom_size {
                custom_size
            } else if let Some(texture) = image_assets.get(texture) {
                texture.size()
            } else {
                continue;
            };
            if cursor.check_square(entity_transform, &sprite.anchor, size) {
                let z_depth = entity_transform.translation().z;
                camera_state.consider(root_resolver.resolve_root(entity), z_depth, || {
                    cursor.cursor_in_world_pos.extend(z_depth)
                });
            }
        }
    }
}

fn update_camera_status_for_atlas_sprites(
    mut cameras_query: Query<&mut VpeolCameraState>,
    entities_query: Query<(
        Entity,
        &GlobalTransform,
        &TextureAtlasSprite,
        &Handle<TextureAtlas>,
    )>,
    texture_atlas_assets: Res<Assets<TextureAtlas>>,
    root_resolver: VpeolRootResolver,
) {
    for mut camera_state in cameras_query.iter_mut() {
        let Some(cursor) = CursorInWorldPos::from_camera_state(&camera_state) else { continue };

        for (entity, entity_transform, sprite, texture) in entities_query.iter() {
            let size = if let Some(custom_size) = sprite.custom_size {
                custom_size
            } else if let Some(texture_atlas) = texture_atlas_assets.get(texture) {
                texture_atlas.textures[sprite.index].size()
            } else {
                continue;
            };
            if cursor.check_square(entity_transform, &sprite.anchor, size) {
                let z_depth = entity_transform.translation().z;
                camera_state.consider(root_resolver.resolve_root(entity), z_depth, || {
                    cursor.cursor_in_world_pos.extend(z_depth)
                });
            }
        }
    }
}

fn update_camera_status_for_text_2d(
    mut cameras_query: Query<&mut VpeolCameraState>,
    entities_query: Query<(Entity, &GlobalTransform, &Text2dSize)>,
    root_resolver: VpeolRootResolver,
) {
    for mut camera_state in cameras_query.iter_mut() {
        let Some(cursor) = CursorInWorldPos::from_camera_state(&camera_state) else { continue };

        for (entity, entity_transform, text_2d_size) in entities_query.iter() {
            if cursor.check_square(entity_transform, &Anchor::TopLeft, text_2d_size.size) {
                let z_depth = entity_transform.translation().z;
                camera_state.consider(root_resolver.resolve_root(entity), z_depth, || {
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
    mut egui_context: ResMut<EguiContext>,
    windows: Res<Windows>,
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
        let window = if let RenderTarget::Window(window_id) = camera.target {
            windows.get(window_id).unwrap()
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
    mut egui_context: ResMut<EguiContext>,
    windows: Res<Windows>,
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

        let window = if let RenderTarget::Window(window_id) = camera.target {
            windows.get(window_id).unwrap()
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

/// See [`vpeol_position_edit_adapter`].
pub struct VpeolTransform2dProjection<'a> {
    pub translation: &'a mut Vec2,
}

/// Edit a `Vec2` position field of an entity with drag&drop.
///
/// Note that this does not populate the `Transform` component - this needs be done with a manually
/// written populate system.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::{YoleckTypeHandler, YoleckExtForApp, YoleckPopulate};
/// # use bevy_yoleck::vpeol_2d::{vpeol_position_edit_adapter, VpeolTransform2dProjection};
/// # use serde::{Deserialize, Serialize};
/// # #[derive(Clone, PartialEq, Serialize, Deserialize)]
/// # struct Example {
/// #     position: Vec2,
/// # }
/// # let mut app = App::new();
/// app.add_yoleck_handler({
///     YoleckTypeHandler::<Example>::new("Example")
///         .with(vpeol_position_edit_adapter(
///             |data: &mut Example| {
///                 VpeolTransform2dProjection {
///                     translation: &mut data.position,
///                 }
///             }
///         ))
///         .populate_with(populate_example)
/// });
///
/// fn populate_example(mut populate: YoleckPopulate<Example>) {
///     populate.populate(|_ctx, data, mut cmd| {
///         cmd.insert_bundle(SpriteBundle {
///             transform: Transform::from_translation(data.position.extend(0.0)),
///             // Actual sprite components
///             ..Default::default()
///         });
///     });
/// }
/// ```
pub fn vpeol_position_edit_adapter<T: 'static>(
    projection: impl 'static
        + Clone
        + Send
        + Sync
        + for<'a> Fn(&'a mut T) -> VpeolTransform2dProjection<'a>,
) -> impl FnOnce(YoleckTypeHandler<T>) -> YoleckTypeHandler<T> {
    move |handler| {
        handler.edit_with(move |mut edit: YoleckEdit<T>| {
            edit.edit(|ctx, data, ui| {
                let VpeolTransform2dProjection { translation } = projection(data);
                if let Some(pos) = ctx.get_passed_data::<Vec3>() {
                    *translation = pos.truncate();
                }
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut translation.x).prefix("X:"));
                    ui.add(egui::DragValue::new(&mut translation.y).prefix("Y:"));
                });
            });
        })
    }
}
