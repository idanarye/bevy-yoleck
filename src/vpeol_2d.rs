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
//! app.add_plugins(EguiPlugin {
//!     enable_multipass_for_primary_context: true,
//! });
//! app.add_plugins(YoleckPluginForEditor);
//! // Use `Vpeol2dPluginForGame` instead when setting up for game.
//! app.add_plugins(Vpeol2dPluginForEditor);
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
//!     .spawn(Camera2d::default())
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
//!         let Ok((entity, mut example)) = edit.single_mut() else { return };
//!         if let Some(pos) = passed_data.get::<Vec3>(entity) {
//!             example.position = pos.truncate();
//!         }
//!     }
//!
//!     fn populate_example(mut populate: YoleckPopulate<&Example>) {
//!         populate.populate(|_ctx, mut cmd, example| {
//!             cmd.insert(Transform::from_translation(example.position.extend(0.0)));
//!             cmd.insert(Sprite {
//!                 // Actual sprite data
//!                 ..Default::default()
//!             });
//!         });
//!     }
//!     ```

use std::any::TypeId;

use crate::bevy_egui::{egui, EguiContexts};
use crate::exclusive_systems::{
    YoleckEntityCreationExclusiveSystems, YoleckExclusiveSystemDirective,
};
use crate::vpeol::{
    handle_clickable_children_system, ray_intersection_with_mesh, VpeolBasePlugin,
    VpeolCameraState, VpeolDragPlane, VpeolRepositionLevel, VpeolRootResolver, VpeolSystemSet,
    WindowGetter,
};
use bevy::input::mouse::MouseWheel;
use bevy::math::DVec2;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::view::VisibleEntities;
use bevy::sprite::Anchor;
use bevy::text::TextLayoutInfo;
use serde::{Deserialize, Serialize};

use crate::{prelude::*, YoleckSchedule};

/// Add the systems required for loading levels that use vpeol_2d components
pub struct Vpeol2dPluginForGame;

impl Plugin for Vpeol2dPluginForGame {
    fn build(&self, app: &mut App) {
        app.add_systems(
            YoleckSchedule::OverrideCommonComponents,
            vpeol_2d_populate_transform,
        );
        #[cfg(feature = "bevy_reflect")]
        register_reflect_types(app);
    }
}

#[cfg(feature = "bevy_reflect")]
fn register_reflect_types(app: &mut App) {
    app.register_type::<Vpeol2dPosition>();
    app.register_type::<Vpeol2dRotatation>();
    app.register_type::<Vpeol2dScale>();
    app.register_type::<Vpeol2dCameraControl>();
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
        app.add_plugins(VpeolBasePlugin);
        app.add_plugins(Vpeol2dPluginForGame);
        app.insert_resource(VpeolDragPlane::XY);

        app.add_systems(
            Update,
            (
                update_camera_status_for_sprites,
                update_camera_status_for_2d_meshes,
                update_camera_status_for_text_2d,
            )
                .in_set(VpeolSystemSet::UpdateCameraState),
        );
        app.add_systems(
            PostUpdate, // to prevent camera shaking (only seen it in 3D, but still)
            (camera_2d_pan, camera_2d_zoom).run_if(in_state(YoleckEditorState::EditorActive)),
        );
        app.add_systems(
            Update,
            (
                ApplyDeferred,
                handle_clickable_children_system::<
                    Or<(With<Sprite>, (With<TextLayoutInfo>, With<Anchor>))>,
                    (),
                >,
                ApplyDeferred,
            )
                .chain()
                .run_if(in_state(YoleckEditorState::EditorActive)),
        );
        app.add_yoleck_edit_system(vpeol_2d_edit_position);
        app.world_mut()
            .resource_mut::<YoleckEntityCreationExclusiveSystems>()
            .on_entity_creation(|queue| queue.push_back(vpeol_2d_init_position));
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

#[allow(clippy::type_complexity)]
fn update_camera_status_for_sprites(
    mut cameras_query: Query<(&mut VpeolCameraState, &VisibleEntities)>,
    entities_query: Query<(Entity, &GlobalTransform, &Sprite)>,
    image_assets: Res<Assets<Image>>,
    texture_atlas_layout_assets: Res<Assets<TextureAtlasLayout>>,
    root_resolver: VpeolRootResolver,
) {
    for (mut camera_state, visible_entities) in cameras_query.iter_mut() {
        let Some(cursor) = CursorInWorldPos::from_camera_state(&camera_state) else {
            continue;
        };

        for (entity, entity_transform, sprite) in
            entities_query.iter_many(visible_entities.iter(TypeId::of::<Sprite>()))
        // entities_query.iter()
        {
            let size = if let Some(custom_size) = sprite.custom_size {
                custom_size
            } else if let Some(texture_atlas) = sprite.texture_atlas.as_ref() {
                let Some(texture_atlas_layout) =
                    texture_atlas_layout_assets.get(&texture_atlas.layout)
                else {
                    continue;
                };
                texture_atlas_layout.textures[texture_atlas.index]
                    .size()
                    .as_vec2()
            } else if let Some(texture) = image_assets.get(&sprite.image) {
                texture.size().as_vec2()
            } else {
                continue;
            };
            if cursor.check_square(entity_transform, &sprite.anchor, size) {
                let z_depth = entity_transform.translation().z;
                let Some(root_entity) = root_resolver.resolve_root(entity) else {
                    continue;
                };
                camera_state.consider(root_entity, z_depth, || {
                    cursor.cursor_in_world_pos.extend(z_depth)
                });
            }
        }
    }
}

fn update_camera_status_for_2d_meshes(
    mut cameras_query: Query<(&mut VpeolCameraState, &VisibleEntities)>,
    entities_query: Query<(Entity, &GlobalTransform, &Mesh2d)>,
    mesh_assets: Res<Assets<Mesh>>,
    root_resolver: VpeolRootResolver,
) {
    for (mut camera_state, visible_entities) in cameras_query.iter_mut() {
        let Some(cursor_ray) = camera_state.cursor_ray else {
            continue;
        };
        for (entity, global_transform, mesh) in
            entities_query.iter_many(visible_entities.iter(TypeId::of::<Mesh2d>()))
        {
            let Some(mesh) = mesh_assets.get(&mesh.0) else {
                continue;
            };

            let inverse_transform = global_transform.compute_matrix().inverse();

            let ray_in_object_coords = Ray3d {
                origin: inverse_transform.transform_point3(cursor_ray.origin),
                direction: inverse_transform
                    .transform_vector3(*cursor_ray.direction)
                    .try_into()
                    .unwrap(),
            };

            let Some(distance) = ray_intersection_with_mesh(ray_in_object_coords, mesh) else {
                continue;
            };

            let Some(root_entity) = root_resolver.resolve_root(entity) else {
                continue;
            };
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
        let Some(cursor) = CursorInWorldPos::from_camera_state(&camera_state) else {
            continue;
        };

        for (entity, entity_transform, text_layout_info, anchor) in
            // Weird that it is not `WithText`...
            entities_query.iter_many(visible_entities.iter(TypeId::of::<Sprite>()))
        {
            if cursor.check_square(entity_transform, anchor, text_layout_info.size) {
                let z_depth = entity_transform.translation().z;
                let Some(root_entity) = root_resolver.resolve_root(entity) else {
                    continue;
                };
                camera_state.consider(root_entity, z_depth, || {
                    cursor.cursor_in_world_pos.extend(z_depth)
                });
            }
        }
    }
}

/// Pan and zoom a camera entity with the mouse while inisde the editor.
#[derive(Component)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy::reflect::Reflect))]
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
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut cameras_query: Query<
        (Entity, &mut Transform, &VpeolCameraState),
        With<Vpeol2dCameraControl>,
    >,
    mut last_cursor_world_pos_by_camera: Local<HashMap<Entity, Vec2>>,
) -> Result {
    enum MouseButtonOp {
        JustPressed,
        BeingPressed,
    }

    let mouse_button_op = if mouse_buttons.just_pressed(MouseButton::Right) {
        if egui_context.ctx_mut()?.is_pointer_over_area() {
            return Ok(());
        }
        MouseButtonOp::JustPressed
    } else if mouse_buttons.pressed(MouseButton::Right) {
        MouseButtonOp::BeingPressed
    } else {
        last_cursor_world_pos_by_camera.clear();
        return Ok(());
    };

    for (camera_entity, mut camera_transform, camera_state) in cameras_query.iter_mut() {
        let Some(cursor_ray) = camera_state.cursor_ray else {
            continue;
        };
        let world_pos = cursor_ray.origin.truncate();

        match mouse_button_op {
            MouseButtonOp::JustPressed => {
                last_cursor_world_pos_by_camera.insert(camera_entity, world_pos);
            }
            MouseButtonOp::BeingPressed => {
                if let Some(prev_pos) = last_cursor_world_pos_by_camera.get_mut(&camera_entity) {
                    let movement = *prev_pos - world_pos;
                    camera_transform.translation += movement.extend(0.0);
                }
            }
        }
    }
    Ok(())
}

fn camera_2d_zoom(
    mut egui_context: EguiContexts,
    window_getter: WindowGetter,
    mut cameras_query: Query<(
        &mut Transform,
        &VpeolCameraState,
        &Camera,
        &Vpeol2dCameraControl,
    )>,
    mut wheel_events_reader: EventReader<MouseWheel>,
) -> Result {
    if egui_context.ctx_mut()?.is_pointer_over_area() {
        return Ok(());
    }

    for (mut camera_transform, camera_state, camera, camera_control) in cameras_query.iter_mut() {
        let Some(cursor_ray) = camera_state.cursor_ray else {
            continue;
        };
        let world_pos = cursor_ray.origin.truncate();

        let zoom_amount: f32 = wheel_events_reader
            .read()
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
        camera_transform.scale.x *= scale_by;
        camera_transform.scale.y *= scale_by;
        let Some(cursor_in_screen_pos) = window.cursor_position() else {
            continue;
        };
        let Ok(new_cursor_ray) =
            camera.viewport_to_world(&((*camera_transform.as_ref()).into()), cursor_in_screen_pos)
        else {
            continue;
        };
        let new_world_pos = new_cursor_ray.origin.truncate();
        camera_transform.translation += (world_pos - new_world_pos).extend(0.0);
    }
    Ok(())
}

/// A position component that's edited and populated by vpeol_2d.
#[derive(Clone, PartialEq, Serialize, Deserialize, Component, Default, YoleckComponent)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy::reflect::Reflect))]
pub struct Vpeol2dPosition(pub Vec2);

/// A rotation component that's populated (but not edited) by vpeol_2d.
///
/// The rotation is in radians around the Z axis.
#[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy::reflect::Reflect))]
pub struct Vpeol2dRotatation(pub f32);

/// A scale component that's populated (but not edited) by vpeol_2d.
#[derive(Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy::reflect::Reflect))]
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
    if edit.is_empty() || edit.has_nonmatching() {
        return;
    }
    // Use double precision to prevent rounding errors when there are many entities.
    let mut average = DVec2::ZERO;
    let mut num_entities = 0;
    let mut transition = Vec2::ZERO;
    for (entity, position) in edit.iter_matching() {
        if let Some(pos) = passed_data.get::<Vec3>(entity) {
            transition = pos.truncate() - position.0;
        }
        average += position.0.as_dvec2();
        num_entities += 1;
    }
    average /= num_entities as f64;

    ui.horizontal(|ui| {
        let mut new_average = average;
        ui.add(egui::DragValue::new(&mut new_average.x).prefix("X:"));
        ui.add(egui::DragValue::new(&mut new_average.y).prefix("Y:"));
        transition += (new_average - average).as_vec2();
    });

    if transition.is_finite() && transition != Vec2::ZERO {
        for (_, mut position) in edit.iter_matching_mut() {
            position.0 += transition;
        }
    }
}

fn vpeol_2d_init_position(
    mut egui_context: EguiContexts,
    ui: Res<YoleckUi>,
    mut edit: YoleckEdit<&mut Vpeol2dPosition>,
    cameras_query: Query<&VpeolCameraState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
) -> YoleckExclusiveSystemDirective {
    let Ok(mut position) = edit.single_mut() else {
        return YoleckExclusiveSystemDirective::Finished;
    };

    let Some(cursor_ray) = cameras_query
        .iter()
        .find_map(|camera_state| camera_state.cursor_ray)
    else {
        return YoleckExclusiveSystemDirective::Listening;
    };

    position.0 = cursor_ray.origin.truncate();

    if egui_context.ctx_mut().unwrap().is_pointer_over_area() || ui.ctx().is_pointer_over_area() {
        return YoleckExclusiveSystemDirective::Listening;
    }

    if mouse_buttons.just_released(MouseButton::Left) {
        return YoleckExclusiveSystemDirective::Finished;
    }

    YoleckExclusiveSystemDirective::Listening
}

fn vpeol_2d_populate_transform(
    mut populate: YoleckPopulate<(
        &Vpeol2dPosition,
        Option<&Vpeol2dRotatation>,
        Option<&Vpeol2dScale>,
        &YoleckBelongsToLevel,
    )>,
    levels_query: Query<&VpeolRepositionLevel>,
) {
    populate.populate(
        |_ctx, mut cmd, (position, rotation, scale, belongs_to_level)| {
            let mut transform = Transform::from_translation(position.0.extend(0.0));
            if let Some(Vpeol2dRotatation(rotation)) = rotation {
                transform = transform.with_rotation(Quat::from_rotation_z(*rotation));
            }
            if let Some(Vpeol2dScale(scale)) = scale {
                transform = transform.with_scale(scale.extend(1.0));
            }

            if let Ok(VpeolRepositionLevel(level_transform)) =
                levels_query.get(belongs_to_level.level)
            {
                transform = *level_transform * transform;
            }

            cmd.insert((transform, GlobalTransform::from(transform)));
        },
    )
}
