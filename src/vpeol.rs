//! # Viewport Editing Overlay - utilities for editing entities from a viewport.
//!
//! This module does not do much, but provide common functionalities for more concrete modules like
//! [`vpeol_2d`](crate::vpeol_2d).

use bevy::ecs::query::ReadOnlyWorldQuery;
use bevy::prelude::*;
use bevy::transform::TransformSystem;
use bevy::utils::HashMap;

use crate::{YoleckEditorState, YoleckState};

pub struct YoleckVpeolBasePlugin;

impl Plugin for YoleckVpeolBasePlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set({
            SystemSet::on_update(YoleckEditorState::EditorActive)
                .label(YoleckVpeolSystemLabel::PrepareCameraState)
                .before(YoleckVpeolSystemLabel::UpdateCameraState)
                .before(YoleckVpeolSystemLabel::HandleCameraState)
                .with_system(prepare_camera_state)
        });
        app.add_system_set({
            SystemSet::on_update(YoleckEditorState::EditorActive)
                .label(YoleckVpeolSystemLabel::HandleCameraState)
                .after(YoleckVpeolSystemLabel::PrepareCameraState)
                .after(YoleckVpeolSystemLabel::UpdateCameraState)
                .with_system(handle_camera_state)
        });
    }
}

fn prepare_camera_state(mut query: Query<&mut YoleckVpeolCameraState>) {
    for mut camera_state in query.iter_mut() {
        camera_state.entity_under_cursor = None;
    }
}

fn handle_camera_state(mut query: Query<&mut YoleckVpeolCameraState>) {
    for camera_state in query.iter_mut() {
        info!("{:?}", camera_state.entity_under_cursor);
    }
}

#[derive(SystemLabel)]
pub enum YoleckVpeolSystemLabel {
    PrepareCameraState,
    UpdateCameraState,
    HandleCameraState,
}

#[derive(Component, Default, Debug)]
pub struct YoleckVpeolCameraState {
    /// The topmost entity being pointed by the cursor.
    pub entity_under_cursor: Option<(Entity, YoleckVpeolCursorPointing)>,
    /// Entities that may or may not be topmost, but the editor needs to know whether or not they
    /// are pointed at.
    pub entities_of_interest: HashMap<Entity, Option<YoleckVpeolCursorPointing>>,
}

#[derive(Clone, Debug)]
pub struct YoleckVpeolCursorPointing {
    pub cursor_position_world_coords: Vec3,
    pub z_depth_screen_coords: f32,
}

impl YoleckVpeolCameraState {
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
            let pointing = YoleckVpeolCursorPointing {
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
                YoleckVpeolCursorPointing {
                    cursor_position_world_coords: cursor_position_world_coords(),
                    z_depth_screen_coords,
                },
            ));
        }
    }
}

/// A [passed data](crate::api::YoleckEditContext::get_passed_data) to a knob entity that indicate it was
/// clicked by the level editor.
pub struct YoleckKnobClick;

/// Marker for entities that will be interacted in the viewport using their children.
///
/// Populate systems should mark the entity with this component when applicable. The viewport
/// overlay plugin is responsible for handling it by using [`handle_clickable_children_system`].
#[derive(Component)]
pub struct YoleckWillContainClickableChildren;

/// Marker for viewport editor overlay plugins to route child interaction to parent entites.
#[derive(Component)]
pub struct YoleckRouteClickTo(pub Entity);

/// Add [`YoleckRouteClickTo`] of entities marked with [`YoleckWillContainClickableChildren`].
pub fn handle_clickable_children_system<F, B>(
    parents_query: Query<(Entity, &Children), With<YoleckWillContainClickableChildren>>,
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
                .remove::<YoleckWillContainClickableChildren>();
        }
    }
}

/// Add a pulse effect when an entity is being selected.
pub struct YoleckVpeolSelectionCuePlugin {
    /// How long, in seconds, the entire pulse effect will take. Defaults to 0.3.
    pub effect_duration: f32,
    /// By how much (relative to original size) the entity will grow during the pulse. Defaults to 0.3.
    pub effect_magnitude: f32,
}

impl Default for YoleckVpeolSelectionCuePlugin {
    fn default() -> Self {
        Self {
            effect_duration: 0.3,
            effect_magnitude: 0.3,
        }
    }
}

impl Plugin for YoleckVpeolSelectionCuePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(manage_selection_transform_components);
        app.add_system_to_stage(
            CoreStage::PostUpdate,
            add_selection_cue_before_transform_propagate(
                1.0 / self.effect_duration,
                2.0 * self.effect_magnitude,
            )
            .before(TransformSystem::TransformPropagate),
        );
        app.add_system_to_stage(
            CoreStage::PostUpdate,
            restore_transform_from_cache_after_transform_propagate
                .after(TransformSystem::TransformPropagate),
        );
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
