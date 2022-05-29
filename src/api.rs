use std::any::TypeId;
use std::marker::PhantomData;

use bevy::ecs::system::{EntityCommands, SystemParam};
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_egui::egui;

use crate::{BoxedArc, YoleckManaged};

/// Whether or not the Yoleck editor is active.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum YoleckEditorState {
    /// Editor mode. The editor is active and can be used to edit entities.
    EditorActive,
    /// Game mode. Either the actual game or playtest from the editor mode.
    GameActive,
}

#[derive(Clone, Copy)]
pub(crate) enum PopulateReason {
    EditorInit,
    EditorUpdate,
    RealGame,
}

/// A context for [`YoleckPopulate::populate`].
pub struct YoleckPopulateContext<'a> {
    pub(crate) reason: PopulateReason,
    // I may add stuff that need 'a later, and I don't want to change the signature
    pub(crate) _phantom_data: PhantomData<&'a ()>,
}

impl<'a> YoleckPopulateContext<'a> {
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

/// A context for [`YoleckEdit::edit`].
pub struct YoleckEditContext<'a> {
    pub(crate) passed: &'a HashMap<TypeId, BoxedArc>,
}

impl YoleckEditContext<'_> {
    pub fn get_passed_data<T: 'static>(&self) -> Option<&T> {
        if let Some(dynamic) = self.passed.get(&TypeId::of::<T>()) {
            dynamic.downcast_ref()
        } else {
            None
        }
    }
}

#[doc(hidden)]
pub struct YoleckUiForEditSystem(pub egui::Ui);

/// Parameter for systems that edit entities. See [`YoleckEdit::edit`].
#[derive(SystemParam)]
pub struct YoleckEdit<'w, 's, T: 'static> {
    #[allow(dead_code)]
    query: Query<'w, 's, &'static mut YoleckManaged>,
    #[allow(dead_code)]
    context: Res<'w, YoleckUserSystemContext>,
    ui: ResMut<'w, YoleckUiForEditSystem>,
    #[system_param(ignore)]
    _phantom_data: PhantomData<fn() -> T>,
}

impl<'w, 's, T: 'static> YoleckEdit<'w, 's, T> {
    /// Implement entity editing.
    ///
    /// A system that uses [`YoleckEdit`] needs to be added to an handler using
    /// [`edit_with`](crate::YoleckTypeHandler::edit_with). These systems usually only need to
    /// call this method with a closure that accepts three arguments:
    ///
    /// * A context
    /// * The data to be edited.
    /// * An egui UI handler.
    ///
    /// The closure is then responsible for allowing the user to edit the data using the UI and
    /// using [data passed from other systems](YoleckEditContext::get_passed_data).
    ///
    /// ```no_run
    /// # use bevy::prelude::*;
    /// # use bevy_yoleck::{YoleckEdit, egui, YoleckTypeHandler, YoleckExtForApp};
    /// # use serde::{Deserialize, Serialize};
    /// # #[derive(Clone, PartialEq, Serialize, Deserialize)]
    /// # struct Example {
    /// #     number: u32,
    /// # }
    /// # let mut app = App::new();
    /// app.add_yoleck_handler({
    ///     YoleckTypeHandler::<Example>::new("Example")
    ///         .edit_with(edit_example)
    /// });
    ///
    /// fn edit_example(mut edit: YoleckEdit<Example>) {
    ///     edit.edit(|_ctx, data, ui| {
    ///         ui.add(egui::Slider::new(&mut data.number, 0..=10));
    ///     });
    /// }
    /// ```
    pub fn edit(&mut self, mut dlg: impl FnMut(&YoleckEditContext, &mut T, &mut egui::Ui)) {
        match &*self.context {
            YoleckUserSystemContext::Nope
            | YoleckUserSystemContext::PopulateEdited(_)
            | YoleckUserSystemContext::PopulateInitiated { .. } => {
                panic!("Wrong state");
            }
            YoleckUserSystemContext::Edit { entity, passed } => {
                let edit_context = YoleckEditContext { passed };
                let mut yoleck_managed = self
                    .query
                    .get_mut(*entity)
                    .expect("Edited entity does not exist");
                let data = yoleck_managed
                    .data
                    .downcast_mut::<T>()
                    .expect("Edited data is of wrong type");
                dlg(&edit_context, data, &mut self.ui.0);
            }
        }
    }
}

/// Parameter for systems that populate entities. See [`YoleckPopulate::populate`].
#[derive(SystemParam)]
pub struct YoleckPopulate<'w, 's, T: 'static> {
    query: Query<'w, 's, &'static mut YoleckManaged>,
    context: Res<'w, YoleckUserSystemContext>,
    commands: Commands<'w, 's>,
    #[system_param(ignore)]
    _phantom_data: PhantomData<fn() -> T>,
}

impl<'w, 's, T: 'static> YoleckPopulate<'w, 's, T> {
    /// Implement entity populating.
    ///
    /// A system that uses [`YoleckPopulate`] needs to be added to an handler using
    /// [`populate_with`](crate::YoleckTypeHandler::populate_with). These systems usually only
    /// need to call this method with a closure that accepts three arguments:
    ///
    /// * A context
    /// * The data to be used for populating.
    /// * A Bevy command.
    ///
    /// The closure is then responsible for adding components to the command based on data from the
    /// entity. The closure may also add children - but since this method may be called to
    /// re-populate an already populated entity that already has children, if it does so it should
    /// use `despawn_descendants` to remove existing children.
    ///
    /// ```no_run
    /// # use bevy::prelude::*;
    /// # use bevy_yoleck::{YoleckPopulate, YoleckTypeHandler, YoleckExtForApp};
    /// # use serde::{Deserialize, Serialize};
    /// # #[derive(Clone, PartialEq, Serialize, Deserialize)]
    /// # struct Example {
    /// #     position: Vec2,
    /// # }
    /// # struct GameAssets {
    /// #     example_sprite: Handle<Image>,
    /// # }
    /// # let mut app = App::new();
    /// app.add_yoleck_handler({
    ///     YoleckTypeHandler::<Example>::new("Example")
    ///         .populate_with(populate_example)
    /// });
    ///
    /// fn populate_example(mut populate: YoleckPopulate<Example>, assets: Res<GameAssets>) {
    ///     populate.populate(|_ctx, data, mut cmd| {
    ///         cmd.insert_bundle(SpriteBundle {
    ///             sprite: Sprite {
    ///                 custom_size: Some(Vec2::new(100.0, 100.0)),
    ///                 ..Default::default()
    ///             },
    ///             transform: Transform::from_translation(data.position.extend(0.0)),
    ///             texture: assets.example_sprite.clone(),
    ///             ..Default::default()
    ///         });
    ///     });
    /// }
    /// ```
    pub fn populate(
        &mut self,
        mut dlg: impl FnMut(&YoleckPopulateContext, &mut T, EntityCommands),
    ) {
        match &*self.context {
            YoleckUserSystemContext::Nope | YoleckUserSystemContext::Edit { .. } => {
                panic!("Wrong state");
            }
            YoleckUserSystemContext::PopulateEdited(entity) => {
                let populate_context = YoleckPopulateContext {
                    reason: PopulateReason::EditorUpdate,
                    _phantom_data: Default::default(),
                };
                let mut yoleck_managed = self
                    .query
                    .get_mut(*entity)
                    .expect("Edited entity does not exist");
                let data = yoleck_managed
                    .data
                    .downcast_mut::<T>()
                    .expect("Edited data is of wrong type");
                dlg(&populate_context, data, self.commands.entity(*entity));
            }
            YoleckUserSystemContext::PopulateInitiated {
                is_in_editor,
                entities,
            } => {
                let populate_context = YoleckPopulateContext {
                    reason: if *is_in_editor {
                        PopulateReason::EditorInit
                    } else {
                        PopulateReason::RealGame
                    },
                    _phantom_data: Default::default(),
                };
                for entity in entities {
                    let mut yoleck_managed = self
                        .query
                        .get_mut(*entity)
                        .expect("Edited entity does not exist");
                    let data = yoleck_managed
                        .data
                        .downcast_mut::<T>()
                        .expect("Edited data is of wrong type");
                    dlg(&populate_context, data, self.commands.entity(*entity));
                }
            }
        }
    }
}

pub enum YoleckUserSystemContext {
    Nope,
    Edit {
        entity: Entity,
        passed: HashMap<TypeId, BoxedArc>,
    },
    PopulateEdited(Entity),
    PopulateInitiated {
        is_in_editor: bool,
        entities: Vec<Entity>,
    },
}

impl YoleckUserSystemContext {
    pub(crate) fn get_edit_entity(&self) -> Entity {
        if let Self::Edit { entity, .. } = self {
            *entity
        } else {
            panic!("Wrong state");
        }
    }
}

/// Events emitted by the Yoleck editor.
///
/// Modules that provide editing overlays over the viewport (like [vpeol](crate::vpeol)) can
/// use these events to update their status to match with the editor.
#[derive(Debug)]
pub enum YoleckEditorEvent {
    EntitySelected(Entity),
    EntityDeselected(Entity),
    EditedEntityPopulated(Entity),
}
