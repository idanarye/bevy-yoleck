use std::ops::{Deref, DerefMut};

use bevy::ecs::query::{QueryIter, QuerySingleError, ReadOnlyWorldQuery, WorldQuery};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::egui;

/// Marks which entities are currently being edited in the level editor.
#[derive(Component)]
pub struct YoleckEditMarker;

/// Wrapper for writing queries in edit systems.
///
/// To future-proof for the multi-entity editing feature, use this instead of
/// regular queries with `With<YoleckEditMarker>`.
///
/// The methods of `YoleckEdit` delegate to the methods of a Bevy's `Query` with the same name, but
/// if there are edited entities that do not fit the query they will act as if they found no match.
#[derive(SystemParam)]
pub struct YoleckEdit<'w, 's, Q: 'static + WorldQuery, F: 'static + ReadOnlyWorldQuery = ()> {
    query: Query<'w, 's, Q, (With<YoleckEditMarker>, F)>,
    verification_query: Query<'w, 's, (), With<YoleckEditMarker>>,
}

impl<'w, 's, Q: 'static + WorldQuery, F: 'static + ReadOnlyWorldQuery> YoleckEdit<'w, 's, Q, F> {
    pub fn get_single(
        &self,
    ) -> Result<<<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'_>, QuerySingleError> {
        let single = self.query.get_single()?;
        // This will return an error if multiple entities are selected (but only one fits F and Q)
        self.verification_query.get_single()?;
        Ok(single)
    }

    pub fn get_single_mut(&mut self) -> Result<<Q as WorldQuery>::Item<'_>, QuerySingleError> {
        let single = self.query.get_single_mut()?;
        // This will return an error if multiple entities are selected (but only one fits F and Q)
        self.verification_query.get_single()?;
        Ok(single)
    }

    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    pub fn has_nonmatching(&self) -> bool {
        // Note - cannot use len for query.iter() because then F would be limited to archtype
        // filters only.
        self.query.iter().count() != self.verification_query.iter().len()
    }

    pub fn iter(
        &mut self,
    ) -> QueryIter<<Q as WorldQuery>::ReadOnly, (bevy::prelude::With<YoleckEditMarker>, F)> {
        self.query.iter()
    }

    pub fn iter_mut(&mut self) -> QueryIter<Q, (With<YoleckEditMarker>, F)> {
        self.query.iter_mut()
    }
}

/// An handle for the egui UI frame used in editing sytems.
#[derive(Resource)]
pub struct YoleckUi(pub egui::Ui);

impl Deref for YoleckUi {
    type Target = egui::Ui;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for YoleckUi {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
