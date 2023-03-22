use bevy::ecs::query::{ReadOnlyWorldQuery, WorldQuery};
use bevy::ecs::system::{EntityCommands, SystemParam};
use bevy::prelude::*;

use crate::entity_management::EntitiesToPopulate;

#[derive(SystemParam)]
pub struct YoleckPopulate<'w, 's, Q: 'static + WorldQuery, F: 'static + ReadOnlyWorldQuery = ()> {
    entities_to_populate: Res<'w, EntitiesToPopulate>,
    query: Query<'w, 's, Q, F>,
    commands: Commands<'w, 's>,
}

impl<Q: 'static + WorldQuery, F: 'static + ReadOnlyWorldQuery> YoleckPopulate<'_, '_, Q, F> {
    pub fn populate(
        &mut self,
        mut dlg: impl FnMut(YoleckPopulateContext, EntityCommands, <Q as WorldQuery>::Item<'_>),
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
