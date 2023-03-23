use std::any::TypeId;
use std::ops::{Deref, DerefMut};

use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_egui::egui;

use crate::BoxedArc;

#[derive(Component)]
pub struct YoleckEdit {
    pub(crate) passed_data: HashMap<TypeId, BoxedArc>,
}

impl YoleckEdit {
    /// Get data sent to the entity from external systems (usually from (usually a [ViewPort Editing OverLay](crate::vpeol))
    ///
    /// The data is sent using [a directive event](crate::YoleckDirective::pass_to_entity).
    ///
    /// ```no_run
    /// # use bevy::prelude::*;
    /// # use bevy_yoleck::prelude::*;;
    /// # #[derive(Component)]
    /// # struct Example {
    /// #     message: String,
    /// # }
    /// fn edit_example(mut query: Query<(&YoleckEdit, &mut Example)>) {
    ///     let Ok((edit, mut example)) = query.get_single_mut() else { return };
    ///     if let Some(message) = edit.get_passed_data::<String>() {
    ///         example.message = message;
    ///     }
    /// }
    /// ```
    pub fn get_passed_data<T: 'static>(&self) -> Option<&T> {
        self.passed_data.get(&TypeId::of::<T>())?.downcast_ref()
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
