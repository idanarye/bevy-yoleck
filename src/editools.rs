//! Utilities for editing entities from a viewport.
//!
//! This module does not do much, but provide common functionalities for more concrete modules like
//! [`crate::editools_2d`] and [`crate::editools_3d`].


use bevy::ecs::query::{FilterFetch, WorldQuery};
use bevy::prelude::*;

/// Marker for entities that will be interacted in the viewport using their children.
///
/// Populate systems should mark the entity with this component when applicable. The viewport
/// overlay plugin is responsible for handling it by using [`handle_clickable_children_system`].
#[derive(Component)]
pub struct YoleckWillContainClickableChildren;

/// Marker for viewport editor overlay plugins to route child interaction to parent entites.
#[derive(Component)]
pub struct RouteClickTo(pub Entity);

/// Add [`RouteClickTo`] of entities marked with [`YoleckWillContainClickableChildren`].
pub fn handle_clickable_children_system<F, B>(
    parents_query: Query<(Entity, &Children), With<YoleckWillContainClickableChildren>>,
    children_query: Query<&Children>,
    should_add_query: Query<Entity, F>,
    mut commands: Commands,
) where
    F: WorldQuery,
    <F as WorldQuery>::Fetch: FilterFetch,
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
                cmd.insert(RouteClickTo(parent));
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
