//! # Viewport Editing Overlay - utilities for editing entities from a viewport.
//!
//! This module does not do much, but provide common functionalities for more concrete modules like
//! [`vpeol_2d`](crate::vpeol_2d).

use bevy::ecs::query::ReadOnlyWorldQuery;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::transform::TransformSystem;
use bevy::utils::HashMap;
use bevy::window::{PrimaryWindow, WindowRef};
use bevy_egui::EguiContexts;

use crate::knobs::YoleckKnobMarker;
use crate::prelude::YoleckEditorState;
use crate::{YoleckDirective, YoleckState};

/// Order of Vpeol operations. Important for abstraction and backends to talk with each other.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub enum VpeolSystemSet {
    /// Initialize [`VpeolCameraState`]
    ///
    /// * Clear all pointing.
    /// * Update [`entities_of_interest`](VpeolCameraState::entities_of_interest).
    /// * Update cursor position (can be overriden later if needed)
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
            (
                VpeolSystemSet::PrepareCameraState,
                VpeolSystemSet::UpdateCameraState,
                VpeolSystemSet::HandleCameraState,
            )
                .chain()
                .in_set(OnUpdate(YoleckEditorState::EditorActive)),
        );
        app.add_system({
            prepare_camera_state.in_set(VpeolSystemSet::PrepareCameraState)
            // .in_set(OnUpdate(YoleckEditorState::EditorActive))
        });
        app.add_system({
            handle_camera_state.in_set(VpeolSystemSet::HandleCameraState)
            // .in_set(OnUpdate(YoleckEditorState::EditorActive))
        });
    }
}

/// Data passed between Vpeol abstraction and backends.
#[derive(Component, Default, Debug)]
pub struct VpeolCameraState {
    /// Where this camera considers the cursor to be in the world.
    pub cursor_in_world_position: Option<Vec3>,
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
        /// Offset from the entity's center to
        /// [`cursor_in_world_position`](VpeolCameraState::cursor_in_world_position).
        offset: Vec3,
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
    buttons: Res<Input<MouseButton>>,
    global_transform_query: Query<&GlobalTransform>,
    knob_query: Query<Entity, With<YoleckKnobMarker>>,
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
        for (_, mut camera_state) in query.iter_mut() {
            camera_state.clicks_on_objects_state = VpeolClicksOnObjectsState::Empty;
        }
        return;
    };
    for (camera, mut camera_state) in query.iter_mut() {
        let Some(cursor_in_world_position) = camera_state.cursor_in_world_position else { continue };

        let RenderTarget::Window(window_ref) = camera.target else { continue };
        let Some(window) = window_getter.get_window(window_ref) else { continue };
        let Some(cursor_in_screen_pos) = window.cursor_position() else { continue };

        match (&mouse_button_op, &camera_state.clicks_on_objects_state) {
            (MouseButtonOp::JustPressed, VpeolClicksOnObjectsState::Empty) => {
                if let Some(knob_entity) = knob_query
                    .iter()
                    .find(|knob_entity| camera_state.pointing_at_entity(*knob_entity).is_some())
                {
                    directives_writer.send(YoleckDirective::pass_to_entity(
                        knob_entity,
                        YoleckKnobClick,
                    ));
                    let Ok(knob_transform) = global_transform_query.get(knob_entity) else { continue };
                    camera_state.clicks_on_objects_state = VpeolClicksOnObjectsState::BeingDragged {
                        entity: knob_entity,
                        prev_screen_pos: cursor_in_screen_pos,
                        offset: cursor_in_world_position - knob_transform.translation(),
                    }
                } else {
                    camera_state.clicks_on_objects_state = if let Some((entity, _cursor_pointing)) =
                        &camera_state.entity_under_cursor
                    {
                        let Ok(entity_transform) = global_transform_query.get(*entity) else { continue };
                        directives_writer.send(YoleckDirective::set_selected(Some(*entity)));
                        VpeolClicksOnObjectsState::BeingDragged {
                            entity: *entity,
                            prev_screen_pos: cursor_in_screen_pos,
                            offset: cursor_in_world_position - entity_transform.translation(),
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
                },
            ) => {
                if 0.1 <= prev_screen_pos.distance_squared(cursor_in_screen_pos) {
                    directives_writer.send(YoleckDirective::pass_to_entity(
                        *entity,
                        cursor_in_world_position - *offset,
                    ));
                    camera_state.clicks_on_objects_state =
                        VpeolClicksOnObjectsState::BeingDragged {
                            entity: *entity,
                            prev_screen_pos: cursor_in_screen_pos,
                            offset: *offset,
                        };
                }
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

/// Marker for viewport editor overlay plugins to route child interaction to parent entites.
#[derive(Component)]
pub struct YoleckRouteClickTo(pub Entity);

/// Helper utility for finding the Yoleck controlled entity that's in charge of an entity the user
/// points at.
#[derive(SystemParam)]
pub struct VpeolRootResolver<'w, 's> {
    root_resolver: Query<'w, 's, &'static YoleckRouteClickTo>,
}

impl VpeolRootResolver<'_, '_> {
    /// Find the Yoleck controlled entity that's in charge of an entity the user points at.
    pub fn resolve_root(&self, entity: Entity) -> Entity {
        if let Ok(YoleckRouteClickTo(root_entity)) = self.root_resolver.get(entity) {
            *root_entity
        } else {
            entity
        }
    }
}

/// Add [`YoleckRouteClickTo`] of entities marked with [`VpeolWillContainClickableChildren`].
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
                    .insert((YoleckRouteClickTo(parent), B::default()));
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
        app.add_system(manage_selection_transform_components);
        app.add_system({
            add_selection_cue_before_transform_propagate(
                1.0 / self.effect_duration,
                2.0 * self.effect_magnitude,
            )
            .in_base_set(CoreSet::PostUpdate)
            .before(TransformSystem::TransformPropagate)
        });
        app.add_system({
            restore_transform_from_cache_after_transform_propagate
                .in_base_set(CoreSet::PostUpdate)
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
    yoleck: Res<YoleckState>,
    existance_query: Query<Option<&SelectionCueAnimation>>,
    animated_query: Query<Entity, With<SelectionCueAnimation>>,
    mut commands: Commands,
) {
    let entitiy_being_edited = yoleck.entity_being_edited();
    if let Some(entity) = entitiy_being_edited {
        if matches!(existance_query.get(entity), Ok(None)) {
            commands.entity(entity).insert(SelectionCueAnimation {
                cached_transform: Default::default(),
                progress: 0.0,
            });
        }
    }
    for entity in animated_query.iter() {
        if Some(entity) != entitiy_being_edited {
            commands.entity(entity).remove::<SelectionCueAnimation>();
        }
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
