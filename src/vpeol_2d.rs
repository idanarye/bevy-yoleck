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
//! # use bevy_yoleck::vpeol_2d::YoleckVpeol2dPlugin;
//! # let mut app = App::new();
//! app.add_plugin(EguiPlugin);
//! app.add_plugin(YoleckPluginForEditor);
//! app.add_plugin(YoleckVpeol2dPlugin);
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
//!         if let Some(pos) = ctx.get_passed_data::<Vec2>() {
//!             data.position = *pos;
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
//! Alternatively, use [`yoleck_vpeol_position_edit_adapter`].

use crate::bevy_egui::{egui, EguiContext};
pub use crate::vpeol::YoleckWillContainClickableChildren;
use crate::vpeol::{handle_clickable_children_system, YoleckRouteClickTo};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::sprite::Anchor;
use bevy::text::Text2dSize;
use bevy::utils::HashMap;

use crate::{YoleckDirective, YoleckEdit, YoleckEditorState, YoleckState, YoleckTypeHandler};

/// Add the systems required for 2D editing.
///
/// * 2D camera zoom/pan
/// * Entity selection.
/// * Entity dragging.
/// * Connecting nested entities.
pub struct YoleckVpeol2dPlugin;

impl Plugin for YoleckVpeol2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set({
            SystemSet::on_update(YoleckEditorState::EditorActive)
                .with_system(yoleck_clicks_on_objects)
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

#[doc(hidden)]
enum YoleckClicksOnObjectsState {
    Empty,
    BeingDragged {
        entity: Entity,
        prev_screen_pos: Vec2,
        offset: Vec2,
    },
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn yoleck_clicks_on_objects(
    mut egui_context: ResMut<EguiContext>,
    windows: Res<Windows>,
    buttons: Res<Input<MouseButton>>,
    cameras_query: Query<(Entity, &GlobalTransform, &Camera), With<OrthographicProjection>>,
    yolek_targets_query: Query<(
        Entity,
        &GlobalTransform,
        AnyOf<(
            (&Sprite, &Handle<Image>),
            (&TextureAtlasSprite, &Handle<TextureAtlas>),
            &Text2dSize,
        )>,
    )>,
    root_resolver: Query<&YoleckRouteClickTo>,
    global_transform_query: Query<&GlobalTransform>,
    image_assets: Res<Assets<Image>>,
    texture_atlas_assets: Res<Assets<TextureAtlas>>,
    yoleck: ResMut<YoleckState>,
    mut state_by_camera: Local<HashMap<Entity, YoleckClicksOnObjectsState>>,
    mut directives_writer: EventWriter<YoleckDirective>,
) {
    enum MouseButtonOp {
        JustPressed,
        BeingPressed,
        JustReleased,
    }

    let mouse_button_op = if buttons.just_pressed(MouseButton::Left) {
        if egui_context.ctx_mut().is_pointer_over_area() {
            return;
        }
        MouseButtonOp::JustPressed
    } else if buttons.just_released(MouseButton::Left) {
        MouseButtonOp::JustReleased
    } else if buttons.pressed(MouseButton::Left) {
        MouseButtonOp::BeingPressed
    } else {
        state_by_camera.clear();
        return;
    };

    let is_world_pos_in = |transform: &GlobalTransform,
                           (regular_sprite, texture_atlas_sprite, text_2d): (
        Option<(&Sprite, &Handle<Image>)>,
        Option<(&TextureAtlasSprite, &Handle<TextureAtlas>)>,
        Option<&Text2dSize>,
    ),
                           cursor_in_world_pos: Vec2|
     -> bool {
        let [x, y, _] = transform
            .compute_matrix()
            .inverse()
            .project_point3(cursor_in_world_pos.extend(0.0))
            .to_array();

        let check = |anchor: &Anchor, size: Vec2| {
            let anchor = anchor.as_vec();
            let mut min_corner = Vec2::new(-0.5, -0.5) - anchor;
            let mut max_corner = Vec2::new(0.5, 0.5) - anchor;
            for corner in [&mut min_corner, &mut max_corner] {
                corner.x *= size.x;
                corner.y *= size.y;
            }
            min_corner.x <= x && x <= max_corner.x && min_corner.y <= y && y <= max_corner.y
        };

        if let Some((sprite, texture_handle)) = regular_sprite {
            let size = if let Some(custom_size) = sprite.custom_size {
                custom_size
            } else if let Some(texture) = image_assets.get(texture_handle) {
                texture.size()
            } else {
                return false;
            };
            if check(&sprite.anchor, size) {
                return true;
            }
        }
        if let Some((sprite, texture_atlas_handle)) = texture_atlas_sprite {
            let size = if let Some(custom_size) = sprite.custom_size {
                custom_size
            } else if let Some(texture_atlas) = texture_atlas_assets.get(texture_atlas_handle) {
                texture_atlas.textures[sprite.index].size()
            } else {
                return false;
            };
            if check(&sprite.anchor, size) {
                return true;
            }
        }
        if let Some(text_2d_size) = text_2d {
            if check(
                &Anchor::TopLeft,
                Vec2::new(text_2d_size.size.width, text_2d_size.size.height),
            ) {
                return true;
            }
        }
        false
    };

    for (camera_entity, camera_transform, camera) in cameras_query.iter() {
        let window = if let RenderTarget::Window(window_id) = camera.target {
            windows.get(window_id).unwrap()
        } else {
            continue;
        };
        if let Some(screen_pos) = window.cursor_position() {
            let world_pos = screen_pos_to_world_pos(screen_pos, window, camera_transform, camera);

            let state = state_by_camera
                .entry(camera_entity)
                .or_insert(YoleckClicksOnObjectsState::Empty);

            let is_entity_still_pointed_at = |entity: Entity| {
                if let Ok((_, entity_transform, sprite)) = yolek_targets_query.get(entity) {
                    if is_world_pos_in(entity_transform, sprite, world_pos) {
                        Some(entity_transform)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            match (&mouse_button_op, &state) {
                (MouseButtonOp::JustPressed, YoleckClicksOnObjectsState::Empty) => {
                    let entity_under_cursor = yoleck
                        .entity_being_edited()
                        .and_then(|entity| Some((entity, is_entity_still_pointed_at(entity)?)))
                        .or_else(|| {
                            let mut result = None;
                            for (entity, entity_transform, sprite) in yolek_targets_query.iter() {
                                if is_world_pos_in(entity_transform, sprite, world_pos) {
                                    if let Some((_, current_result_z)) = result {
                                        if entity_transform.translation.z < current_result_z {
                                            continue;
                                        }
                                    }
                                    result = Some((
                                        (entity, entity_transform),
                                        entity_transform.translation.z,
                                    ));
                                }
                            }
                            result.map(|(result, _)| result)
                        });
                    *state = if let Some((entity, entity_transform)) = entity_under_cursor {
                        let (entity, entity_transform) = if let Ok(YoleckRouteClickTo(
                            root_entity,
                        )) = root_resolver.get(entity)
                        {
                            let root_entity_transform = global_transform_query.get(*root_entity)
                                .expect("when routing to root entity, the root entity should have its own GlobalTransform");
                            (*root_entity, root_entity_transform)
                        } else {
                            (entity, entity_transform)
                        };
                        directives_writer.send(YoleckDirective::set_selected(Some(entity)));
                        YoleckClicksOnObjectsState::BeingDragged {
                            entity,
                            prev_screen_pos: screen_pos,
                            offset: world_pos - entity_transform.translation.truncate(),
                        }
                    } else {
                        directives_writer.send(YoleckDirective::set_selected(None));
                        YoleckClicksOnObjectsState::Empty
                    }
                }
                (
                    MouseButtonOp::BeingPressed,
                    YoleckClicksOnObjectsState::BeingDragged {
                        entity,
                        prev_screen_pos,
                        offset,
                    },
                ) => {
                    if 0.1 <= prev_screen_pos.distance_squared(screen_pos) {
                        directives_writer.send(YoleckDirective::pass_to_entity(
                            *entity,
                            world_pos - *offset,
                        ));
                        *state = YoleckClicksOnObjectsState::BeingDragged {
                            entity: *entity,
                            prev_screen_pos: screen_pos,
                            offset: *offset,
                        };
                    }
                }
                _ => {}
            }
        }
    }
}

fn camera_2d_pan(
    mut egui_context: ResMut<EguiContext>,
    windows: Res<Windows>,
    buttons: Res<Input<MouseButton>>,
    mut cameras_query: Query<
        (Entity, &mut Transform, &GlobalTransform, &Camera),
        With<OrthographicProjection>,
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
            let world_pos =
                screen_pos_to_world_pos(screen_pos, window, camera_global_transform, camera);

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
    mut cameras_query: Query<
        (&mut Transform, &GlobalTransform, &Camera),
        With<OrthographicProjection>,
    >,
    mut wheel_events_reader: EventReader<MouseWheel>,
) {
    if egui_context.ctx_mut().is_pointer_over_area() {
        return;
    }

    let zoom_amount: f32 = wheel_events_reader
        .iter()
        .map(|wheel_event| match wheel_event.unit {
            bevy::input::mouse::MouseScrollUnit::Line => wheel_event.y * 0.2,
            bevy::input::mouse::MouseScrollUnit::Pixel => wheel_event.y * 0.001,
        })
        .sum();

    if zoom_amount == 0.0 {
        return;
    }

    let scale_by = (-zoom_amount).exp();

    for (mut camera_transform, camera_global_transform, camera) in cameras_query.iter_mut() {
        let window = if let RenderTarget::Window(window_id) = camera.target {
            windows.get(window_id).unwrap()
        } else {
            continue;
        };
        if let Some(screen_pos) = window.cursor_position() {
            let world_pos =
                screen_pos_to_world_pos(screen_pos, window, camera_global_transform, camera);
            camera_transform.scale.x *= scale_by;
            camera_transform.scale.y *= scale_by;
            let mut new_global_transform = *camera_global_transform;
            new_global_transform.scale.x *= scale_by;
            new_global_transform.scale.y *= scale_by;
            let new_world_pos =
                screen_pos_to_world_pos(screen_pos, window, &new_global_transform, camera);
            camera_transform.translation += (world_pos - new_world_pos).extend(0.0);
        }
    }
}

fn screen_pos_to_world_pos(
    screen_pos: Vec2,
    wnd: &Window,
    camera_transform: &GlobalTransform,
    camera: &Camera,
) -> Vec2 {
    // Code stolen from https://bevy-cheatbook.github.io/cookbook/cursor2world.html

    // get the size of the window
    let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

    // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
    let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

    // matrix for undoing the projection and camera transform
    let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix.inverse();

    // use it to convert ndc to world-space coordinates
    let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

    // reduce it to a 2D value
    world_pos.truncate()
}

/// See [`yoleck_vpeol_position_edit_adapter`].
pub struct YoleckVpeolTransform2dProjection<'a> {
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
/// # use bevy_yoleck::vpeol_2d::{yoleck_vpeol_position_edit_adapter, YoleckVpeolTransform2dProjection};
/// # use serde::{Deserialize, Serialize};
/// # #[derive(Clone, PartialEq, Serialize, Deserialize)]
/// # struct Example {
/// #     position: Vec2,
/// # }
/// # let mut app = App::new();
/// app.add_yoleck_handler({
///     YoleckTypeHandler::<Example>::new("Example")
///         .with(yoleck_vpeol_position_edit_adapter(
///             |data: &mut Example| {
///                 YoleckVpeolTransform2dProjection {
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
pub fn yoleck_vpeol_position_edit_adapter<T: 'static>(
    projection: impl 'static
        + Clone
        + Send
        + Sync
        + for<'a> Fn(&'a mut T) -> YoleckVpeolTransform2dProjection<'a>,
) -> impl FnOnce(YoleckTypeHandler<T>) -> YoleckTypeHandler<T> {
    move |handler| {
        handler.edit_with(move |mut edit: YoleckEdit<T>| {
            edit.edit(|ctx, data, ui| {
                let YoleckVpeolTransform2dProjection { translation } = projection(data);
                if let Some(pos) = ctx.get_passed_data::<Vec2>() {
                    *translation = *pos;
                }
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut translation.x).prefix("X:"));
                    ui.add(egui::DragValue::new(&mut translation.y).prefix("Y:"));
                });
            });
        })
    }
}
