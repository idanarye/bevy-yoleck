use std::ops::{Deref, DerefMut};

use bevy::ecs::query::{QuerySingleError, ReadOnlyWorldQuery, WorldQuery};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::egui;

#[derive(Component)]
pub struct YoleckEditMarker;

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
}

/// TODO: document
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
