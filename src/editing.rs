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
    /// # use bevy_yoleck::{YoleckEdit, egui};
    /// # struct Example {
    /// #     position: Vec2,
    /// # }
    /// fn edit_example(mut edit: YoleckEdit<Example>) {
    ///     edit.edit(|ctx, data, _ui| {
    ///         if let Some(pos) = ctx.get_passed_data::<Vec3>() {
    ///             data.position = pos.truncate();
    ///         }
    ///     });
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
