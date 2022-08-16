//! # Viewport Editing Overlay - utilities for editing entities from a viewport.
//!
//! This module does not do much, but provide common functionalities for more concrete modules like
//! [`vpeol_2d`](crate::vpeol_2d).

use bevy::ecs::query::WorldQuery;
use bevy::prelude::*;
use bevy::transform::TransformSystem;

use crate::YoleckState;

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
    F: WorldQuery,
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
                let mut cmd = commands.entity(child);
                cmd.insert(YoleckRouteClickTo(parent));
                cmd.insert_bundle(B::default());
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
