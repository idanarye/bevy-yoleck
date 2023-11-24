//! # Viewport Editing Overlay - utilities for editing entities from a viewport.
//!
//! This module does not do much, but provide common functionalities for more concrete modules like
//! [`vpeol_2d`](crate::vpeol_2d) and [`vpeol_3d`](crate::vpeol_3d).
//!
//! `vpeol` modules also support `bevy_reflect::Reflect` by enabling the feature `beavy_reflect`.

use bevy::ecs::query::ReadOnlyWorldQuery;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::primitives::Aabb;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::transform::TransformSystem;
use bevy::utils::HashMap;
use bevy::window::{PrimaryWindow, WindowRef};
use bevy_egui::EguiContexts;

use crate::knobs::YoleckKnobMarker;
use crate::prelude::{YoleckEditorState, YoleckUi};
use crate::{YoleckDirective, YoleckEditMarker, YoleckManaged, YoleckRunEditSystemsSystemSet};

pub mod prelude {
    pub use crate::vpeol::{
        VpeolCameraState, VpeolDragPlane, VpeolRepositionLevel, VpeolSelectionCuePlugin,
        VpeolWillContainClickableChildren, YoleckKnobClick,
    };
    #[cfg(feature = "vpeol_2d")]
    pub use crate::vpeol_2d::{
        Vpeol2dCameraControl, Vpeol2dPluginForEditor, Vpeol2dPluginForGame, Vpeol2dPosition,
        Vpeol2dRotatation, Vpeol2dScale,
    };
    #[cfg(feature = "vpeol_3d")]
    pub use crate::vpeol_3d::{
        Vpeol3dCameraControl, Vpeol3dPluginForEditor, Vpeol3dPluginForGame, Vpeol3dPosition,
        Vpeol3dRotatation, Vpeol3dScale, Vpeol3dThirdAxisWithKnob,
    };
}

/// Order of Vpeol operations. Important for abstraction and backends to talk with each other.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub enum VpeolSystemSet {
    /// Initialize [`VpeolCameraState`]
    ///
    /// * Clear all pointing.
    /// * Update [`entities_of_interest`](VpeolCameraState::entities_of_interest).
    /// * Update cursor position (can be overridden later if needed)
    PrepareCameraState,
    /// Mostly used by the backend to iterate over the entities and determine which ones are
    /// being pointed (using [`consider`](VpeolCameraState::consider))
    UpdateCameraState,
    /// Interpret the mouse data and pass it back to Yoleck.
    HandleCameraState,
}

/// Add base systems common for Vpeol editing.
pub struct VpeolBasePlugin;

impl Plugin for VpeolBasePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                VpeolSystemSet::PrepareCameraState
                    .run_if(in_state(YoleckEditorState::EditorActive)),
                VpeolSystemSet::UpdateCameraState.run_if(in_state(YoleckEditorState::EditorActive)),
                VpeolSystemSet::HandleCameraState.run_if(in_state(YoleckEditorState::EditorActive)),
                YoleckRunEditSystemsSystemSet,
            )
                .chain(), // .run_if(in_state(YoleckEditorState::EditorActive)),
        );
        app.add_systems(
            Update,
            (prepare_camera_state, update_camera_world_position)
                .in_set(VpeolSystemSet::PrepareCameraState),
        );
        app.add_systems(
            Update,
            handle_camera_state.in_set(VpeolSystemSet::HandleCameraState),
        );
        #[cfg(feature = "bevy_reflect")]
        app.register_type::<VpeolDragPlane>();
    }
}

/// A plane to define the drag direction of entities.
///
/// This is both a component and a resource. Entities that have the component will use the plane
/// defined by it, while entities that don't will use the global one defined by the resource.
/// Child entities will use the plane of the root Yoleck managed entity (if it has one). Knobs will
/// use the one attached to the knob entity.
///
/// This configuration is only meaningful for 3D, but vpeol_2d still requires it resource.
/// `Vpeol2dPluginForEditor` already adds it as `Vec3::Z`. Don't modify it.
#[derive(Component, Resource)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy::reflect::Reflect))]
pub struct VpeolDragPlane {
    /// The normal of the plane.
    pub normal: Vec3,
}

impl VpeolDragPlane {
    pub const XY: VpeolDragPlane = VpeolDragPlane { normal: Vec3::Z };
    pub const XZ: VpeolDragPlane = VpeolDragPlane { normal: Vec3::Y };
}

/// Data passed between Vpeol abstraction and backends.
#[derive(Component, Default, Debug)]
pub struct VpeolCameraState {
    /// Where this camera considers the cursor to be in the world.
    pub cursor_ray: Option<Ray>,
    /// The topmost entity being pointed by the cursor.
    pub entity_under_cursor: Option<(Entity, VpeolCursorPointing)>,
    /// Entities that may or may not be topmost, but the editor needs to know whether or not they
    /// are pointed at.
    pub entities_of_interest: HashMap<Entity, Option<VpeolCursorPointing>>,
    /// The mouse selection state.
    pub clicks_on_objects_state: VpeolClicksOnObjectsState,
}

/// Information on how the cursor is pointing at an entity.
#[derive(Clone, Debug)]
pub struct VpeolCursorPointing {
    /// The location on the entity, in world coords, where the cursor is pointing.
    pub cursor_position_world_coords: Vec3,
    /// Used to determine entity selection priorities.
    pub z_depth_screen_coords: f32,
}

/// State for determining how the user is interacting with entities using the mouse buttons.
#[derive(Default, Debug)]
pub enum VpeolClicksOnObjectsState {
    #[default]
    Empty,
    BeingDragged {
        entity: Entity,
        /// Used for deciding if the cursor has moved.
        prev_screen_pos: Vec2,
        /// Offset from the entity's center to the cursor's position on the drag plane.
        offset: Vec3,
        select_on_mouse_release: bool,
    },
}

impl VpeolCameraState {
    /// Tell Vpeol the the user is pointing at an entity.
    ///
    /// This function may ignore the input if the entity is covered by another entity and is not an
    /// entity of interest.
    pub fn consider(
        &mut self,
        entity: Entity,
        z_depth_screen_coords: f32,
        cursor_position_world_coords: impl FnOnce() -> Vec3,
    ) {
        let should_update_entity = if let Some((_, old_cursor)) = self.entity_under_cursor.as_ref()
        {
            old_cursor.z_depth_screen_coords < z_depth_screen_coords
        } else {
            true
        };

        if let Some(of_interest) = self.entities_of_interest.get_mut(&entity) {
            let pointing = VpeolCursorPointing {
                cursor_position_world_coords: cursor_position_world_coords(),
                z_depth_screen_coords,
            };
            if should_update_entity {
                self.entity_under_cursor = Some((entity, pointing.clone()));
            }
            *of_interest = Some(pointing);
        } else if should_update_entity {
            self.entity_under_cursor = Some((
                entity,
                VpeolCursorPointing {
                    cursor_position_world_coords: cursor_position_world_coords(),
                    z_depth_screen_coords,
                },
            ));
        }
    }

    pub fn pointing_at_entity(&self, entity: Entity) -> Option<&VpeolCursorPointing> {
        if let Some((entity_under_cursor, pointing_at)) = &self.entity_under_cursor {
            if *entity_under_cursor == entity {
                return Some(pointing_at);
            }
        }
        self.entities_of_interest.get(&entity)?.as_ref()
    }
}

fn prepare_camera_state(
    mut query: Query<&mut VpeolCameraState>,
    knob_query: Query<Entity, With<YoleckKnobMarker>>,
) {
    for mut camera_state in query.iter_mut() {
        camera_state.entity_under_cursor = None;
        camera_state.entities_of_interest = knob_query
            .iter()
            .chain(match camera_state.clicks_on_objects_state {
                VpeolClicksOnObjectsState::Empty => None,
                VpeolClicksOnObjectsState::BeingDragged { entity, .. } => Some(entity),
            })
            .map(|entity| (entity, None))
            .collect();
    }
}

fn update_camera_world_position(
    mut cameras_query: Query<(&mut VpeolCameraState, &GlobalTransform, &Camera)>,
    window_getter: WindowGetter,
) {
    for (mut camera_state, camera_transform, camera) in cameras_query.iter_mut() {
        camera_state.cursor_ray = (|| {
            let RenderTarget::Window(window_ref) = camera.target else {
                return None;
            };
            let window = window_getter.get_window(window_ref)?;
            let cursor_in_screen_pos = window.cursor_position()?;
            camera.viewport_to_world(camera_transform, cursor_in_screen_pos)
        })();
    }
}

#[derive(SystemParam)]
pub(crate) struct WindowGetter<'w, 's> {
    windows: Query<'w, 's, &'static Window>,
    primary_window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
}

impl WindowGetter<'_, '_> {
    pub fn get_window(&self, window_ref: WindowRef) -> Option<&Window> {
        match window_ref {
            WindowRef::Primary => self.primary_window.get_single().ok(),
            WindowRef::Entity(window_id) => self.windows.get(window_id).ok(),
        }
    }
}

fn handle_camera_state(
    mut egui_context: EguiContexts,
    mut query: Query<(&Camera, &mut VpeolCameraState)>,
    window_getter: WindowGetter,
    mouse_buttons: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
    global_transform_query: Query<&GlobalTransform>,
    selected_query: Query<(), With<YoleckEditMarker>>,
    knob_query: Query<Entity, With<YoleckKnobMarker>>,
    mut directives_writer: EventWriter<YoleckDirective>,
    global_drag_plane: Res<VpeolDragPlane>,
    drag_plane_overrides_query: Query<&VpeolDragPlane>,
) {
    enum MouseButtonOp {
        JustPressed,
        BeingPressed,
        JustReleased,
    }
    let mouse_button_op = if mouse_buttons.just_pressed(MouseButton::Left) {
        if egui_context.ctx_mut().is_pointer_over_area() {
            return;
        }
        MouseButtonOp::JustPressed
    } else if mouse_buttons.just_released(MouseButton::Left) {
        MouseButtonOp::JustReleased
    } else if mouse_buttons.pressed(MouseButton::Left) {
        MouseButtonOp::BeingPressed
    } else {
        for (_, mut camera_state) in query.iter_mut() {
            camera_state.clicks_on_objects_state = VpeolClicksOnObjectsState::Empty;
        }
        return;
    };
    for (camera, mut camera_state) in query.iter_mut() {
        let Some(cursor_ray) = camera_state.cursor_ray else {
            continue;
        };
        let calc_cursor_in_world_position = |entity: Entity, plane_origin: Vec3| -> Option<Vec3> {
            let drag_plane_normal =
                if let Ok(drag_plane_override) = drag_plane_overrides_query.get(entity) {
                    drag_plane_override.normal
                } else {
                    global_drag_plane.normal
                };
            let distance = cursor_ray.intersect_plane(plane_origin, drag_plane_normal)?;
            Some(cursor_ray.get_point(distance))
        };

        let RenderTarget::Window(window_ref) = camera.target else {
            continue;
        };
        let Some(window) = window_getter.get_window(window_ref) else {
            continue;
        };
        let Some(cursor_in_screen_pos) = window.cursor_position() else {
            continue;
        };

        match (&mouse_button_op, &camera_state.clicks_on_objects_state) {
            (MouseButtonOp::JustPressed, VpeolClicksOnObjectsState::Empty) => {
                if keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
                    if let Some((entity, _)) = &camera_state.entity_under_cursor {
                        directives_writer.send(YoleckDirective::toggle_selected(*entity));
                    }
                } else if let Some((knob_entity, cursor_pointing)) =
                    knob_query.iter().find_map(|knob_entity| {
                        Some((knob_entity, camera_state.pointing_at_entity(knob_entity)?))
                    })
                {
                    directives_writer.send(YoleckDirective::pass_to_entity(
                        knob_entity,
                        YoleckKnobClick,
                    ));
                    let Ok(knob_transform) = global_transform_query.get(knob_entity) else {
                        continue;
                    };
                    let Some(cursor_in_world_position) = calc_cursor_in_world_position(
                        knob_entity,
                        cursor_pointing.cursor_position_world_coords,
                    ) else {
                        continue;
                    };
                    camera_state.clicks_on_objects_state = VpeolClicksOnObjectsState::BeingDragged {
                        entity: knob_entity,
                        prev_screen_pos: cursor_in_screen_pos,
                        offset: cursor_in_world_position - knob_transform.translation(),
                        select_on_mouse_release: false,
                    }
                } else {
                    camera_state.clicks_on_objects_state = if let Some((entity, cursor_pointing)) =
                        &camera_state.entity_under_cursor
                    {
                        let Ok(entity_transform) = global_transform_query.get(*entity) else {
                            continue;
                        };
                        let select_on_mouse_release = selected_query.contains(*entity);
                        if !select_on_mouse_release {
                            directives_writer.send(YoleckDirective::set_selected(Some(*entity)));
                        }
                        let Some(cursor_in_world_position) = calc_cursor_in_world_position(
                            *entity,
                            cursor_pointing.cursor_position_world_coords,
                        ) else {
                            continue;
                        };
                        VpeolClicksOnObjectsState::BeingDragged {
                            entity: *entity,
                            prev_screen_pos: cursor_in_screen_pos,
                            offset: cursor_in_world_position - entity_transform.translation(),
                            select_on_mouse_release,
                        }
                    } else {
                        directives_writer.send(YoleckDirective::set_selected(None));
                        VpeolClicksOnObjectsState::Empty
                    };
                }
            }
            (
                MouseButtonOp::BeingPressed,
                VpeolClicksOnObjectsState::BeingDragged {
                    entity,
                    prev_screen_pos,
                    offset,
                    select_on_mouse_release: _,
                },
            ) => {
                if 0.1 <= prev_screen_pos.distance_squared(cursor_in_screen_pos) {
                    let Ok(entity_transform) = global_transform_query.get(*entity) else {
                        continue;
                    };
                    let drag_point = entity_transform.translation() + *offset;
                    let Some(cursor_in_world_position) =
                        calc_cursor_in_world_position(*entity, drag_point)
                    else {
                        continue;
                    };
                    directives_writer.send(YoleckDirective::pass_to_entity(
                        *entity,
                        cursor_in_world_position - *offset,
                    ));
                    camera_state.clicks_on_objects_state =
                        VpeolClicksOnObjectsState::BeingDragged {
                            entity: *entity,
                            prev_screen_pos: cursor_in_screen_pos,
                            offset: *offset,
                            select_on_mouse_release: false,
                        };
                }
            }
            (
                MouseButtonOp::JustReleased,
                VpeolClicksOnObjectsState::BeingDragged {
                    entity,
                    prev_screen_pos: _,
                    offset: _,
                    select_on_mouse_release: true,
                },
            ) => {
                directives_writer.send(YoleckDirective::set_selected(Some(*entity)));
                camera_state.clicks_on_objects_state = VpeolClicksOnObjectsState::Empty;
            }
            _ => {}
        }
    }
}

/// A [passed data](crate::knobs::YoleckKnobHandle::get_passed_data) to a knob entity that indicate
/// it was clicked by the level editor.
pub struct YoleckKnobClick;

/// Marker for entities that will be interacted in the viewport using their children.
///
/// Populate systems should mark the entity with this component when applicable. The viewport
/// overlay plugin is responsible for handling it by using [`handle_clickable_children_system`].
#[derive(Component)]
pub struct VpeolWillContainClickableChildren;

/// Marker for viewport editor overlay plugins to route child interaction to parent entities.
#[derive(Component)]
pub struct VpeolRouteClickTo(pub Entity);

/// Helper utility for finding the Yoleck controlled entity that's in charge of an entity the user
/// points at.
#[derive(SystemParam)]
pub struct VpeolRootResolver<'w, 's> {
    root_resolver: Query<'w, 's, &'static VpeolRouteClickTo>,
    has_managed_query: Query<'w, 's, Or<(With<YoleckManaged>, With<YoleckKnobMarker>)>>,
}

impl VpeolRootResolver<'_, '_> {
    /// Find the Yoleck controlled entity that's in charge of an entity the user points at.
    pub fn resolve_root(&self, entity: Entity) -> Option<Entity> {
        if let Ok(VpeolRouteClickTo(root_entity)) = self.root_resolver.get(entity) {
            Some(*root_entity)
        } else {
            self.has_managed_query.get(entity).ok()?;
            Some(entity)
        }
    }
}

/// Add [`VpeolRouteClickTo`] of entities marked with [`VpeolWillContainClickableChildren`].
pub fn handle_clickable_children_system<F, B>(
    parents_query: Query<(Entity, &Children), With<VpeolWillContainClickableChildren>>,
    children_query: Query<&Children>,
    should_add_query: Query<Entity, F>,
    mut commands: Commands,
) where
    F: ReadOnlyWorldQuery,
    B: Default + Bundle,
{
    for (parent, children) in parents_query.iter() {
        if children.is_empty() {
            continue;
        }
        let mut any_added = false;
        let mut children_to_check: Vec<Entity> = children.iter().copied().collect();
        while let Some(child) = children_to_check.pop() {
            if let Ok(child_children) = children_query.get(child) {
                children_to_check.extend(child_children.iter().copied());
            }
            if should_add_query.get(child).is_ok() {
                commands
                    .entity(child)
                    .insert((VpeolRouteClickTo(parent), B::default()));
                any_added = true;
            }
        }
        if any_added {
            commands
                .entity(parent)
                .remove::<VpeolWillContainClickableChildren>();
        }
    }
}

/// Add a pulse effect when an entity is being selected.
pub struct VpeolSelectionCuePlugin {
    /// How long, in seconds, the entire pulse effect will take. Defaults to 0.3.
    pub effect_duration: f32,
    /// By how much (relative to original size) the entity will grow during the pulse. Defaults to 0.3.
    pub effect_magnitude: f32,
}

impl Default for VpeolSelectionCuePlugin {
    fn default() -> Self {
        Self {
            effect_duration: 0.3,
            effect_magnitude: 0.3,
        }
    }
}

impl Plugin for VpeolSelectionCuePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, manage_selection_transform_components);
        app.add_systems(PostUpdate, {
            add_selection_cue_before_transform_propagate(
                1.0 / self.effect_duration,
                2.0 * self.effect_magnitude,
            )
            .before(TransformSystem::TransformPropagate)
        });
        app.add_systems(PostUpdate, {
            restore_transform_from_cache_after_transform_propagate
                .after(TransformSystem::TransformPropagate)
        });
    }
}

#[derive(Component)]
struct SelectionCueAnimation {
    cached_transform: Transform,
    progress: f32,
}

fn manage_selection_transform_components(
    add_cue_query: Query<Entity, (Without<SelectionCueAnimation>, With<YoleckEditMarker>)>,
    remove_cue_query: Query<Entity, (With<SelectionCueAnimation>, Without<YoleckEditMarker>)>,
    mut commands: Commands,
) {
    for entity in add_cue_query.iter() {
        commands.entity(entity).insert(SelectionCueAnimation {
            cached_transform: Default::default(),
            progress: 0.0,
        });
    }
    for entity in remove_cue_query.iter() {
        commands.entity(entity).remove::<SelectionCueAnimation>();
    }
}

fn add_selection_cue_before_transform_propagate(
    time_speedup: f32,
    magnitude_scale: f32,
) -> impl FnMut(Query<(&mut SelectionCueAnimation, &mut Transform)>, Res<Time>) {
    move |mut query, time| {
        for (mut animation, mut transform) in query.iter_mut() {
            animation.cached_transform = *transform;
            if animation.progress < 1.0 {
                animation.progress += time_speedup * time.delta_seconds();
                let extra = if animation.progress < 0.5 {
                    animation.progress
                } else {
                    1.0 - animation.progress
                };
                transform.scale *= 1.0 + magnitude_scale * extra;
            }
        }
    }
}

fn restore_transform_from_cache_after_transform_propagate(
    mut query: Query<(&SelectionCueAnimation, &mut Transform)>,
) {
    for (animation, mut transform) in query.iter_mut() {
        *transform = animation.cached_transform;
    }
}

pub(crate) fn ray_intersection_with_mesh(ray: Ray, mesh: &Mesh) -> Option<f32> {
    let aabb = mesh.compute_aabb()?;
    let distance_to_aabb = ray_intersection_with_aabb(ray, aabb)?;

    if let Some(mut triangles) = iter_triangles(mesh) {
        triangles.find_map(|triangle| triangle.ray_intersection(ray))
    } else {
        Some(distance_to_aabb)
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
        let dot = ray.direction.dot(axis);
        if dot == 0.0 {
            let distance_from_center = (ray.origin - center).dot(axis);
            if half_extent < distance_from_center.abs() {
                return None;
            }
        } else {
            let low = ray.intersect_plane(center - half_extent * axis, axis);
            let high = ray.intersect_plane(center + half_extent * axis, axis);
            let (low, high) = if 0.0 <= dot { (low, high) } else { (high, low) };
            if let Some(low) = low {
                max_low = max_low.max(low);
            }
            if let Some(high) = high {
                min_high = min_high.min(high);
            } else {
                return None;
            }
        }
    }
    if max_low <= min_high {
        Some(max_low)
    } else {
        None
    }
}

fn iter_triangles(mesh: &Mesh) -> Option<impl '_ + Iterator<Item = Triangle>> {
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        return None;
    }
    let indices = mesh.indices()?;
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return None;
    };
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

/// Detects an entity that's being clicked on. Meant to be used with [Yoleck's exclusive edit
/// systems](crate::exclusive_systems::YoleckExclusiveSystemsQueue) and with Bevy's system piping.
///
/// Note that this only returns `Some` when the user clicks on an entity - it does not finish the
/// exclusive system. The other systems that this gets piped into should decide whether or not it
/// should be finished.
pub fn vpeol_read_click_on_entity<Filter: ReadOnlyWorldQuery>(
    mut ui: ResMut<YoleckUi>,
    cameras_query: Query<&VpeolCameraState>,
    yoleck_managed_query: Query<&YoleckManaged>,
    filter_query: Query<(), Filter>,
    buttons: Res<Input<MouseButton>>,
    mut candidate: Local<Option<Entity>>,
) -> Option<Entity> {
    let target = if ui.ctx().is_pointer_over_area() {
        None
    } else {
        cameras_query
            .iter()
            .find_map(|camera_state| Some(camera_state.entity_under_cursor.as_ref()?.0))
    };

    let Some(target) = target else {
        ui.label("No Target");
        return None;
    };

    let Ok(yoleck_managed) = yoleck_managed_query.get(target) else {
        ui.label("No Target");
        return None;
    };

    if !filter_query.contains(target) {
        ui.label(format!("Invalid Target ({})", yoleck_managed.type_name));
        return None;
    }
    ui.label(format!(
        "Targeting {:?} ({})",
        target, yoleck_managed.type_name
    ));

    if buttons.just_pressed(MouseButton::Left) {
        *candidate = Some(target);
    } else if buttons.just_released(MouseButton::Left) {
        if let Some(candidate) = candidate.take() {
            if candidate == target {
                return Some(target);
            }
        }
    }
    None
}

/// Apply a transform to every entity in the level.
///
/// Note that:
/// * It is the duty of [`vpeol_2d`](crate::vpeol_2d)/[`vpeol_3d`](crate::vpeol_3d) to handle the
///   actual repositioning, and they do so only for entities that use their existing components
///   ([`Vpeol2dPosition`](crate::vpeol_2d::Vpeol2dPosition)/[`Vpeol3dPosition`](crate::vpeol_3d::Vpeol3dPosition)
///   and friends). If there are entities that do not use these mechanisms, it falls under the
///   responsibility of whatever populates their `Transform` to take this component (of their level
///   entity) into account.
/// * The repositioning is done directly on the `Transform` - not on the `GlobalTransform`.
#[derive(Component)]
pub struct VpeolRepositionLevel(pub Transform);
