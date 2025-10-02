use std::ops::RangeFrom;

use bevy::ecs::query::{QueryData, QueryFilter};
use bevy::ecs::system::{EntityCommands, SystemParam};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::entity_management::EntitiesToPopulate;

/// Wrapper for writing queries in populate systems.
#[derive(SystemParam)]
pub struct YoleckPopulate<'w, 's, Q: 'static + QueryData, F: 'static + QueryFilter = ()> {
    entities_to_populate: Res<'w, EntitiesToPopulate>,
    query: Query<'w, 's, Q, F>,
    commands: Commands<'w, 's>,
}

impl<Q: 'static + QueryData, F: 'static + QueryFilter> YoleckPopulate<'_, '_, Q, F> {
    /// Iterate over the entities that need populating in order to add/update components using
    /// a Bevy command.
    pub fn populate(
        &mut self,
        mut dlg: impl FnMut(YoleckPopulateContext, EntityCommands, <Q as QueryData>::Item<'_, '_>),
    ) {
        for (entity, populate_reason) in self.entities_to_populate.0.iter() {
            if let Ok(data) = self.query.get_mut(*entity) {
                let cmd = self.commands.entity(*entity);
                let context = YoleckPopulateContext {
                    reason: *populate_reason,
                };
                dlg(context, cmd, data);
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum PopulateReason {
    EditorInit,
    EditorUpdate,
    RealGame,
}

/// A context for [`YoleckPopulate::populate`].
pub struct YoleckPopulateContext {
    pub(crate) reason: PopulateReason,
}

impl YoleckPopulateContext {
    /// `true` if the entity is created in editor mode, `false` if created in playtest or actual game.
    pub fn is_in_editor(&self) -> bool {
        match self.reason {
            PopulateReason::EditorInit => true,
            PopulateReason::EditorUpdate => true,
            PopulateReason::RealGame => false,
        }
    }

    /// `true` if this is this is the first time the entity is populated, `false` if the entity was
    /// popultated before.
    pub fn is_first_time(&self) -> bool {
        match self.reason {
            PopulateReason::EditorInit => true,
            PopulateReason::EditorUpdate => false,
            PopulateReason::RealGame => true,
        }
    }
}

/// See [`YoleckMarking`].
#[derive(Debug, Component, PartialEq, Eq, Clone, Copy)]
pub struct YoleckSystemMarker(usize);

#[derive(Resource)]
struct MarkerGenerator(RangeFrom<usize>);

impl FromWorld for YoleckSystemMarker {
    fn from_world(world: &mut World) -> Self {
        let mut marker = world.get_resource_or_insert_with(|| MarkerGenerator(1..));
        YoleckSystemMarker(marker.0.next().unwrap())
    }
}

/// Use to mark child entities created from a specific system.
///
/// Using `despawn_descendants` is dangerous because it can delete entities created by other
/// populate systems in the same frame. Instead, a populate system that wants to despawn its own
/// child entities should do something like this:
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::prelude::*;
/// # use serde::{Deserialize, Serialize};
/// # #[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
/// # struct MyComponent;
/// fn populate_system(mut populate: YoleckPopulate<&MyComponent>, marking: YoleckMarking) {
///     populate.populate(|_ctx, mut cmd, my_component| {
///         marking.despawn_marked(&mut cmd);
///         cmd.with_children(|commands| {
///             let mut child = commands.spawn(marking.marker());
///             child.insert((
///                 // relevant Bevy components
///             ));
///         });
///     });
/// }
/// ```
#[derive(SystemParam)]
pub struct YoleckMarking<'w, 's> {
    designated_marker: Local<'s, YoleckSystemMarker>,
    children_query: Query<'w, 's, &'static Children>,
    marked_query: Query<'w, 's, (&'static ChildOf, &'static YoleckSystemMarker)>,
}

impl YoleckMarking<'_, '_> {
    /// Create a marker unique to this system.
    pub fn marker(&self) -> YoleckSystemMarker {
        *self.designated_marker
    }

    /// Despawn all the entities marked by the current system, that are descendants of the entity
    /// edited by the supplied `cmd`.
    pub fn despawn_marked(&self, cmd: &mut EntityCommands) {
        let mut marked_children_map: HashMap<Entity, Vec<Entity>> = Default::default();
        for child in self.children_query.iter_descendants(cmd.id()) {
            let Ok((child_of, marker)) = self.marked_query.get(child) else {
                continue;
            };
            if *marker == *self.designated_marker {
                marked_children_map
                    .entry(child_of.parent())
                    .or_default()
                    .push(child);
            }
        }
        for (parent, children) in marked_children_map {
            cmd.commands().entity(parent).remove_children(&children);
            for child in children {
                cmd.commands().entity(child).despawn();
            }
        }
    }
}
