//! # Viewport Editing Overlay for 3D games.
//!
//! Use this module to implement simple 3D editing for 3D games.
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
//!
//! // - Use `Vpeol3dPluginForGame` instead when setting up for game.
//! // - Use topdown is for games that utilize the XZ plane. There is also
//! //   `Vpeol3dPluginForEditor::sidescroller` for games that mainly need the XY plane.
//! app.add_plugin(Vpeol3dPluginForEditor::topdown());
//! ```
//!
//! Add the following components to the camera entity:
//! * [`VpeolCameraState`] in order to select and drag entities.
//! * [`Vpeol3dCameraControl`] in order to control the camera with the mouse. This one can be
//!   skipped if there are other means to control the camera inside the editor, or if no camera
//!   control inside the editor is desired.
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoleck::vpeol::prelude::*;
//! # let commands: Commands = panic!();
//! commands
//!     .spawn(Camera3dBundle::default())
//!     .insert(VpeolCameraState::default())
//!     // Use a variant of the camera controls that fit the choice of editor plugin.
//!     .insert(Vpeol3dCameraControl::topdown());
//! ```
//!
//! Entity selection by clicking on it is supported by just adding the plugin. To implement
//! dragging, there are two options:
//!
//! 1. Add  the [`Vpeol3dPosition`] Yoleck component and use it as the source of position (there
//!    are also [`Vpeol3dRotatation`] and [`Vpeol3dScale`], but they don't currently get editing
//!    support from vpeol_3d). To enable dragging across the third axis, add
//!    [`Vpeol3dThirdAxisWithKnob`] as well.
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
//!             .with::<Vpeol3dPosition>() // vpeol_3d dragging
//!             .with::<Example>() // entity's specific data and systems
//!             // Optional:
//!             .insert_on_init_during_editor(|| Vpeol3dThirdAxisWithKnob {
//!                 knob_distance: 2.0,
//!                 knob_scale: 0.5,
//!             })
//!     });
//!     ```
//! 2. Use data passing. vpeol_3d will pass a `Vec3` to the entity being dragged:
//!     ```no_run
//!     # use bevy::prelude::*;
//!     # use bevy_yoleck::prelude::*;
//!     # use serde::{Deserialize, Serialize};
//!     # #[derive(Clone, PartialEq, Serialize, Deserialize, Component, Default, YoleckComponent)]
//!     # struct Example {
//!     #     position: Vec3,
//!     # }
//!     # let mut app = App::new();
//!     fn edit_example(mut edit: YoleckEdit<(Entity, &mut Example)>, passed_data: Res<YoleckPassedData>) {
//!         let Ok((entity, mut example)) = edit.get_single_mut() else { return };
//!         if let Some(pos) = passed_data.get::<Vec3>(entity) {
//!             example.position = *pos;
//!         }
//!     }
//!
//!     fn populate_example(mut populate: YoleckPopulate<&Example>) {
//!         populate.populate(|_ctx, mut cmd, example| {
//!             cmd.insert(SpriteBundle {
//!                 transform: Transform::from_translation(example.position),
//!                 // Actual model/scene components
//!                 ..Default::default()
//!             });
//!         });
//!     }
//!     ```
//!     When using this option, [`Vpeol3dThirdAxisWithKnob`] can still be used to add the third
//!     axis knob.

use crate::bevy_egui::egui;
use crate::vpeol::{
    handle_clickable_children_system, VpeolBasePlugin, VpeolCameraState, VpeolDragPlane,
    VpeolRootResolver, VpeolSystemSet,
};
use crate::{prelude::*, YoleckDirective, YoleckPopulateBaseSet};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::primitives::Aabb;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::view::VisibleEntities;
use bevy::utils::HashMap;
use bevy_egui::EguiContexts;
use serde::{Deserialize, Serialize};

/// Add the systems required for loading levels that use vpeol_3d components
pub struct Vpeol3dPluginForGame;

impl Plugin for Vpeol3dPluginForGame {
    fn build(&self, app: &mut App) {
        app.yoleck_populate_schedule_mut().add_system(
            vpeol_3d_populate_transform
                .in_base_set(YoleckPopulateBaseSet::OverrideCommonComponents),
        );
    }
}

/// Add the systems required for 3D editing.
///
/// * 3D camera control (for cameras with [`Vpeol3dCameraControl`])
/// * Entity selection.
/// * Entity dragging.
/// * Connecting nested entities.
pub struct Vpeol3dPluginForEditor {
    /// The normal to configure the global [`VpeolDragPlane`] resource with.
    ///
    /// Indiviual entities can override this with their own [`VpeolDragPlane`] component.
    ///
    /// It is a good idea to match this to [`Vpeol3dCameraControl::plane_normal`].
    pub drag_plane_normal: Vec3,
}

impl Vpeol3dPluginForEditor {
    /// For sidescroller games - drag entities along the XY plane.
    ///
    /// Indiviual entities can override this with a [`VpeolDragPlane`] component.
    ///
    /// Adding [`Vpeol3dThirdAxisWithKnob`] can be used to allow Z axis manipulation.
    ///
    /// This combines well with [`Vpeol3dCameraControl::sidescroller`].
    pub fn sidescroller() -> Self {
        Self {
            drag_plane_normal: Vec3::Z,
        }
    }

    /// For games that are not sidescrollers - drag entities along the XZ plane.
    ///
    /// Indiviual entities can override this with a [`VpeolDragPlane`] component.
    ///
    /// Adding [`Vpeol3dThirdAxisWithKnob`] can be used to allow Y axis manipulation.
    ///
    /// This combines well with [`Vpeol3dCameraControl::topdown`].
    pub fn topdown() -> Self {
        Self {
            drag_plane_normal: Vec3::Y,
        }
    }
}

impl Plugin for Vpeol3dPluginForEditor {
    fn build(&self, app: &mut App) {
        app.add_plugin(VpeolBasePlugin);
        app.add_plugin(Vpeol3dPluginForGame);
        app.insert_resource(VpeolDragPlane {
            normal: self.drag_plane_normal,
        });

        app.add_systems(
            (update_camera_status_for_models,).in_set(VpeolSystemSet::UpdateCameraState),
        );
        app.add_systems(
            (
                camera_3d_pan,
                camera_3d_move_along_plane_normal,
                camera_3d_rotate,
            )
                .in_set(OnUpdate(YoleckEditorState::EditorActive)),
        );
        app.add_systems(
            (
                apply_system_buffers,
                handle_clickable_children_system::<With<Handle<Mesh>>, ()>,
                apply_system_buffers,
            )
                .chain()
                .in_set(OnUpdate(YoleckEditorState::EditorActive)),
        );
        app.add_yoleck_edit_system(vpeol_3d_edit_position);
        app.add_yoleck_edit_system(vpeol_3d_edit_third_axis_with_knob);
    }
}

fn ray_intersection_with_aabb(ray: Ray, aabb: Aabb) -> Option<f32> {
    let center: Vec3 = aabb.center.into();
    let mut max_low = f32::NEG_INFINITY;
    let mut min_high = f32::INFINITY;
    for (axis, half_extent) in [
        (Vec3::X, aabb.half_extents.x),
        (Vec3::Y, aabb.half_extents.y),
        (Vec3::Z, aabb.half_extents.z),
    ] {
        let low = ray.intersect_plane(center - half_extent * axis, axis);
        let high = ray.intersect_plane(center + half_extent * axis, axis);
        let (low, high) = if 0.0 <= ray.direction.dot(axis) {
            (low, high)
        } else {
            (high, low)
        };
        if let Some(low) = low {
            max_low = max_low.max(low);
        }
        if let Some(high) = high {
            min_high = min_high.min(high);
        } else {
            return None;
        }
    }
    if max_low <= min_high {
        Some(max_low)
    } else {
        None
    }
}

fn iter_triangles<'a>(mesh: &'a Mesh) -> Option<impl 'a + Iterator<Item = Triangle>> {
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        return None;
    }
    let indices = mesh.indices()?;
    let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { return None };
    let mut it = indices.iter();
    Some(std::iter::from_fn(move || {
        Some(Triangle(
            [it.next()?, it.next()?, it.next()?].map(|idx| Vec3::from_array(positions[idx])),
        ))
    }))
}

#[derive(Debug)]
struct Triangle([Vec3; 3]);

impl Triangle {
    fn ray_intersection(&self, ray: Ray) -> Option<f32> {
        let directions = [
            self.0[1] - self.0[0],
            self.0[2] - self.0[1],
            self.0[0] - self.0[2],
        ];
        let normal = directions[0].cross(directions[1]); // no need to normalize it
        let distance = ray.intersect_plane(self.0[0], normal)?;
        let point = ray.get_point(distance);
        if self
            .0
            .iter()
            .zip(directions.iter())
            .all(|(vertex, direction)| {
                let vertical = direction.cross(normal);
                vertical.dot(point - *vertex) <= 0.0
            })
        {
            Some(distance)
        } else {
            None
        }
    }
}

fn update_camera_status_for_models(
    mut cameras_query: Query<(&mut VpeolCameraState, &VisibleEntities)>,
    entities_query: Query<(Entity, &GlobalTransform, &Handle<Mesh>)>,
    mesh_assets: Res<Assets<Mesh>>,
    root_resolver: VpeolRootResolver,
) {
    for (mut camera_state, visible_entities) in cameras_query.iter_mut() {
        let Some(cursor_ray) = camera_state.cursor_ray else { continue };
        for (entity, global_transform, mesh) in entities_query.iter_many(&visible_entities.entities)
        {
            let Some(mesh) = mesh_assets.get(mesh) else { continue };
            let Some(aabb) = mesh.compute_aabb() else { continue };

            let inverse_transform = global_transform.compute_matrix().inverse();

            let ray_in_object_coords = Ray {
                origin: inverse_transform.transform_point3(cursor_ray.origin),
                direction: inverse_transform.transform_vector3(cursor_ray.direction),
            };

            let Some(distance_to_aabb) = ray_intersection_with_aabb(ray_in_object_coords, aabb) else { continue };

            let distance = if let Some(mut triangles) = iter_triangles(&mesh) {
                if let Some(distance) =
                    triangles.find_map(|triangle| triangle.ray_intersection(ray_in_object_coords))
                {
                    distance
                } else {
                    continue;
                }
            } else {
                distance_to_aabb
            };

            let Some(root_entity) = root_resolver.resolve_root(entity) else { continue };
            camera_state.consider(root_entity, -distance, || cursor_ray.get_point(distance));
        }
    }
}

/// Move and rotate a camera entity with the mouse while inisde the editor.
#[derive(Component)]
pub struct Vpeol3dCameraControl {
    /// Panning is done by dragging a plane with this as its origin.
    pub plane_origin: Vec3,
    /// Panning is done by dragging a plane with this as its normal.
    pub plane_normal: Vec3,
    /// Is `Some`, enable mouse rotation. The up direction of the camera will be the specific
    /// direction.
    ///
    /// It is a good idea to match this to [`Vpeol3dPluginForEditor::drag_plane_normal`].
    pub allow_rotation_while_maintaining_up: Option<Vec3>,
    /// How much to change the proximity to the plane when receiving scroll event in
    /// `MouseScrollUnit::Line` units.
    pub proximity_per_scroll_line: f32,
    /// How much to change the proximity to the plane when receiving scroll event in
    /// `MouseScrollUnit::Pixel` units.
    pub proximity_per_scroll_pixel: f32,
}

impl Vpeol3dCameraControl {
    /// Preset for sidescroller games, where the the game world is on the XY plane.
    ///
    /// With this preset, the camera rotation is disabled.
    ///
    /// This combines well with [`Vpeol3dPluginForEditor::sidescroller`].
    pub fn sidescroller() -> Self {
        Self {
            plane_origin: Vec3::ZERO,
            plane_normal: -Vec3::Z,
            allow_rotation_while_maintaining_up: None,
            proximity_per_scroll_line: 2.0,
            proximity_per_scroll_pixel: 0.01,
        }
    }

    /// Preset for games where the the game world is mainly on the XZ plane (though there can still
    /// be verticality)
    ///
    /// This combines well with [`Vpeol3dPluginForEditor::topdown`].
    pub fn topdown() -> Self {
        Self {
            plane_origin: Vec3::ZERO,
            plane_normal: Vec3::Y,
            allow_rotation_while_maintaining_up: Some(Vec3::Y),
            proximity_per_scroll_line: 2.0,
            proximity_per_scroll_pixel: 0.01,
        }
    }

    fn ray_intersection(&self, ray: Ray) -> Option<Vec3> {
        let distance = ray.intersect_plane(self.plane_origin, self.plane_normal)?;
        Some(ray.get_point(distance))
    }
}

fn camera_3d_pan(
    mut egui_context: EguiContexts,
    buttons: Res<Input<MouseButton>>,
    mut cameras_query: Query<(
        Entity,
        &mut Transform,
        &VpeolCameraState,
        &Vpeol3dCameraControl,
    )>,
    mut last_cursor_world_pos_by_camera: Local<HashMap<Entity, Vec3>>,
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

    for (camera_entity, mut camera_transform, camera_state, camera_control) in
        cameras_query.iter_mut()
    {
        let Some(cursor_ray) = camera_state.cursor_ray else { continue };
        match mouse_button_op {
            MouseButtonOp::JustPressed => {
                let Some(world_pos) = camera_control.ray_intersection(cursor_ray) else { continue };
                last_cursor_world_pos_by_camera.insert(camera_entity, world_pos);
            }
            MouseButtonOp::BeingPressed => {
                if let Some(prev_pos) = last_cursor_world_pos_by_camera.get_mut(&camera_entity) {
                    let Some(world_pos) = camera_control.ray_intersection(cursor_ray) else { continue };
                    let movement = *prev_pos - world_pos;
                    camera_transform.translation += movement;
                }
            }
        }
    }
}

fn camera_3d_move_along_plane_normal(
    mut egui_context: EguiContexts,
    mut cameras_query: Query<(&mut Transform, &Vpeol3dCameraControl)>,
    mut wheel_events_reader: EventReader<MouseWheel>,
) {
    if egui_context.ctx_mut().is_pointer_over_area() {
        return;
    }

    for (mut camera_transform, camera_control) in cameras_query.iter_mut() {
        let zoom_amount: f32 = wheel_events_reader
            .iter()
            .map(|wheel_event| match wheel_event.unit {
                bevy::input::mouse::MouseScrollUnit::Line => {
                    wheel_event.y * camera_control.proximity_per_scroll_line
                }
                bevy::input::mouse::MouseScrollUnit::Pixel => {
                    wheel_event.y * camera_control.proximity_per_scroll_pixel
                }
            })
            .sum();

        if zoom_amount == 0.0 {
            continue;
        }

        camera_transform.translation += zoom_amount * camera_control.plane_normal;
    }
}

fn camera_3d_rotate(
    mut egui_context: EguiContexts,
    buttons: Res<Input<MouseButton>>,
    mut cameras_query: Query<(
        Entity,
        &mut Transform,
        &VpeolCameraState,
        &Vpeol3dCameraControl,
    )>,
    mut last_cursor_ray_by_camera: Local<HashMap<Entity, Ray>>,
) {
    enum MouseButtonOp {
        JustPressed,
        BeingPressed,
    }

    let mouse_button_op = if buttons.just_pressed(MouseButton::Middle) {
        if egui_context.ctx_mut().is_pointer_over_area() {
            return;
        }
        MouseButtonOp::JustPressed
    } else if buttons.pressed(MouseButton::Middle) {
        MouseButtonOp::BeingPressed
    } else {
        last_cursor_ray_by_camera.clear();
        return;
    };

    for (camera_entity, mut camera_transform, camera_state, camera_control) in
        cameras_query.iter_mut()
    {
        let Some(maintaining_up) = camera_control.allow_rotation_while_maintaining_up else { continue };
        let Some(cursor_ray) = camera_state.cursor_ray else { continue };
        match mouse_button_op {
            MouseButtonOp::JustPressed => {
                last_cursor_ray_by_camera.insert(camera_entity, cursor_ray);
            }
            MouseButtonOp::BeingPressed => {
                if let Some(prev_ray) = last_cursor_ray_by_camera.get_mut(&camera_entity) {
                    let rotation =
                        Quat::from_rotation_arc(cursor_ray.direction, prev_ray.direction);
                    camera_transform.rotate(rotation);
                    let new_forward = camera_transform.forward();
                    camera_transform.look_to(new_forward, maintaining_up);
                }
            }
        }
    }
}

/// A position component that's edited and populated by vpeol_3d.
///
/// Editing is done with egui, or by dragging the entity on a [`VpeolDragPlane`]  that passes
/// through the entity. To support dragging perpendicular to that plane, use
/// [`Vpeol3dThirdAxisWithKnob`].
#[derive(Clone, PartialEq, Serialize, Deserialize, Component, Default, YoleckComponent)]
#[serde(transparent)]
pub struct Vpeol3dPosition(pub Vec3);

/// Add a knob for dragging the entity perpendicular to the [`VpeolDragPlane`].
///
/// Dragging the knob will not actually change any component - it will only pass to the entity a
/// `Vec3` that describes the drag. Since regular entity dragging is also implemented by passing a
/// `Vec3`, just adding this component should be enough if there is already an edit system in place
/// that reads that `Vec3` (such as the edit system for [`Vpeol3dPosition`])
#[derive(Component)]
pub struct Vpeol3dThirdAxisWithKnob {
    /// The distance of the knob from the entity's origin.
    pub knob_distance: f32,
    /// A scale for the knob's model.
    pub knob_scale: f32,
}

/// A rotation component that's populated (but not edited) by vpeol_3d.
#[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
#[serde(transparent)]
pub struct Vpeol3dRotatation(pub Quat);

/// A scale component that's populated (but not edited) by vpeol_3d.
#[derive(Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
#[serde(transparent)]
pub struct Vpeol3dScale(pub Vec3);

impl Default for Vpeol3dScale {
    fn default() -> Self {
        Self(Vec3::ONE)
    }
}

fn vpeol_3d_edit_position(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<(Entity, &mut Vpeol3dPosition)>,
    passed_data: Res<YoleckPassedData>,
) {
    let Ok((entity, mut position)) = edit.get_single_mut() else { return };
    if let Some(pos) = passed_data.get::<Vec3>(entity) {
        position.0 = *pos;
    }
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut position.0.x).prefix("X:"));
        ui.add(egui::DragValue::new(&mut position.0.y).prefix("Y:"));
        ui.add(egui::DragValue::new(&mut position.0.z).prefix("Z:"));
    });
}

fn vpeol_3d_edit_third_axis_with_knob(
    mut edit: YoleckEdit<(
        Entity,
        &GlobalTransform,
        &Vpeol3dThirdAxisWithKnob,
        Option<&VpeolDragPlane>,
    )>,
    global_drag_plane: Res<VpeolDragPlane>,
    mut knobs: YoleckKnobs,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut mesh_and_material: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
    mut directives_writer: EventWriter<YoleckDirective>,
) {
    let Ok((entity, global_transform, third_axis_with_knob, drag_plane)) = edit.get_single_mut() else { return };

    let (mesh, material) = mesh_and_material.get_or_insert_with(|| {
        (
            mesh_assets.add(Mesh::from(shape::Cylinder {
                radius: 0.5,
                height: 1.0,
                resolution: 10,
                segments: 10,
            })),
            material_assets.add(Color::ORANGE_RED.into()),
        )
    });

    let drag_plane = drag_plane.unwrap_or(global_drag_plane.as_ref());
    let entity_position = global_transform.translation();

    for (knob_name, drag_plane_normal) in [
        ("vpeol-3d-third-axis-knob-positive", drag_plane.normal),
        ("vpeol-3d-third-axis-knob-negative", -drag_plane.normal),
    ] {
        let mut knob = knobs.knob(knob_name);
        let knob_offset = third_axis_with_knob.knob_distance * drag_plane_normal;
        let knob_transform = Transform {
            translation: entity_position + knob_offset,
            rotation: Quat::from_rotation_arc(Vec3::Y, drag_plane_normal),
            scale: third_axis_with_knob.knob_scale * Vec3::ONE,
        };
        knob.cmd.insert(VpeolDragPlane {
            normal: drag_plane
                .normal
                .cross(Vec3::X)
                .try_normalize()
                .unwrap_or(Vec3::Y),
        });
        knob.cmd.insert(PbrBundle {
            mesh: mesh.clone(),
            material: material.clone(),
            transform: knob_transform,
            global_transform: knob_transform.into(),
            ..Default::default()
        });
        if let Some(pos) = knob.get_passed_data::<Vec3>() {
            let vector_from_entity = *pos - knob_offset - entity_position;
            let along_drag_normal = vector_from_entity.dot(drag_plane.normal);
            let vector_along_drag_normal = along_drag_normal * drag_plane.normal;
            let position_along_drag_normal = entity_position + vector_along_drag_normal;
            directives_writer.send(YoleckDirective::pass_to_entity(
                entity,
                position_along_drag_normal,
            ));
        }
    }
}

fn vpeol_3d_populate_transform(
    mut populate: YoleckPopulate<(
        &Vpeol3dPosition,
        Option<&Vpeol3dRotatation>,
        Option<&Vpeol3dScale>,
    )>,
) {
    populate.populate(|_ctx, mut cmd, (position, rotation, scale)| {
        let mut transform = Transform::from_translation(position.0);
        if let Some(Vpeol3dRotatation(rotation)) = rotation {
            transform = transform.with_rotation(*rotation);
        }
        if let Some(Vpeol3dScale(scale)) = scale {
            transform = transform.with_scale(*scale);
        }
        cmd.insert(TransformBundle {
            local: transform,
            global: transform.into(),
        });
    })
}
