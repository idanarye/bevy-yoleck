use std::ops::{Deref, DerefMut};

use bevy::ecs::query::{QueryData, QueryFilter, QueryIter, QuerySingleError};
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
/// The methods of `YoleckEdit` that have the same name as methods of a regular Bevy `Query`
/// delegate to them, but if there are edited entities that do not fit the query they will act as
/// if they found no match.
#[derive(SystemParam)]
pub struct YoleckEdit<'w, 's, Q: 'static + QueryData, F: 'static + QueryFilter = ()> {
    query: Query<'w, 's, Q, (With<YoleckEditMarker>, F)>,
    verification_query: Query<'w, 's, (), With<YoleckEditMarker>>,
}

impl<'s, Q: 'static + QueryData, F: 'static + QueryFilter> YoleckEdit<'_, 's, Q, F> {
    pub fn single(
        &self,
    ) -> Result<<<Q as QueryData>::ReadOnly as QueryData>::Item<'_, 's>, QuerySingleError> {
        let single = self.query.single()?;
        // This will return an error if multiple entities are selected (but only one fits F and Q)
        self.verification_query.single()?;
        Ok(single)
    }

    pub fn single_mut(&mut self) -> Result<<Q as QueryData>::Item<'_, 's>, QuerySingleError> {
        let single = self.query.single_mut()?;
        // This will return an error if multiple entities are selected (but only one fits F and Q)
        self.verification_query.single()?;
        Ok(single)
    }

    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    /// Check if some non-matching entities are selected for editing.
    ///
    /// Use this, together with [`is_empty`](Self::is_empty) for systems that can edit multiple
    /// entities but want to not show their UI when some irrelevant entities are selected as well.
    pub fn has_nonmatching(&self) -> bool {
        // Note - cannot use len for query.iter() because then `F` would be limited to archetype
        // filters only.
        self.query.iter().count() != self.verification_query.iter().len()
    }

    /// Iterate over all the matching entities, _even_ if some selected entities do not match.
    ///
    /// If both matching and non-matching entities are selected, this will iterate over the
    /// matching entities only. If it is not desired to iterate at all in such cases,
    /// check [`has_nonmatching`](Self::has_nonmatching) must be checked manually.
    pub fn iter_matching(
        &mut self,
    ) -> QueryIter<'_, '_, <Q as QueryData>::ReadOnly, (bevy::prelude::With<YoleckEditMarker>, F)>
    {
        self.query.iter()
    }

    /// Iterate mutably over all the matching entities, _even_ if some selected entities do not match.
    ///
    /// If both matching and non-matching entities are selected, this will iterate over the
    /// matching entities only. If it is not desired to iterate at all in such cases,
    /// check [`has_nonmatching`](Self::has_nonmatching) must be checked manually.
    pub fn iter_matching_mut(&mut self) -> QueryIter<'_, '_, Q, (With<YoleckEditMarker>, F)> {
        self.query.iter_mut()
    }
}

/// An handle for the egui UI frame used in editing systems.
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
