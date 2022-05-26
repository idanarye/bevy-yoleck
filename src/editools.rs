use bevy::ecs::query::{FilterFetch, WorldQuery};
use bevy::prelude::*;

#[derive(Component)]
pub struct WillContainClickableChildren;

#[derive(Component)]
pub struct RouteClickTo(pub Entity);

pub fn handle_clickable_children_system<F, B>(
    parents_query: Query<(Entity, &Children), With<WillContainClickableChildren>>,
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
                .remove::<WillContainClickableChildren>();
        }
    }
}
