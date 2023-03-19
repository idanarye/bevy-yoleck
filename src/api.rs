use std::any::TypeId;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use bevy::ecs::system::{EntityCommands, SystemParam};
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_egui::egui;
use serde::{Deserialize, Serialize};

use crate::knobs::{KnobFromCache, YoleckKnobsCache};
use crate::{BoxedArc, YoleckComponentHandler, YoleckManaged};

/// Whether or not the Yoleck editor is active.
#[derive(States, Default, Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum YoleckEditorState {
    /// Editor mode. The editor is active and can be used to edit entities.
    #[default]
    EditorActive,
    /// Game mode. Either the actual game or playtest from the editor mode.
    GameActive,
}

/// Sync the game's state back and forth when the level editor enters and exits playtest mode.
///
/// Add this as a plugin. When using it, there is no need to initialize the state with `add_state`
/// - `YoleckSyncWithEditorState` will initialize it and set its initial value to `when_editor`.
/// This means that the state's default value should be it's initial value for non-editor mode
/// (which is not necessarily `when_game`, because the game may start in a menu state or a loading
/// state)
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::{YoleckSyncWithEditorState, YoleckPluginForEditor, YoleckPluginForGame};
/// # use bevy_yoleck::bevy_egui::EguiPlugin;
/// #[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
/// enum GameState {
///     #[default]
///     Loading,
///     Game,
///     Editor,
/// }
///
/// # let mut app = App::new();
/// # let executable_started_in_editor_mode = true;
/// if executable_started_in_editor_mode {
///     // These two plugins are needed for editor mode:
///     app.add_plugin(EguiPlugin);
///     app.add_plugin(YoleckPluginForEditor);
///
///     app.add_plugin(YoleckSyncWithEditorState {
///         when_editor: GameState::Editor,
///         when_game: GameState::Game,
///     });
/// } else {
///     // This plugin is needed for game mode:
///     app.add_plugin(YoleckPluginForGame);
///
///     app.add_state::<GameState>();
/// }
pub struct YoleckSyncWithEditorState<T>
where
    T: 'static + States + Sync + Send + std::fmt::Debug + Clone + std::cmp::Eq + std::hash::Hash,
{
    pub when_editor: T,
    pub when_game: T,
}

impl<T> Plugin for YoleckSyncWithEditorState<T>
where
    T: 'static + States + Sync + Send + std::fmt::Debug + Clone + std::cmp::Eq + std::hash::Hash,
{
    fn build(&self, app: &mut App) {
        app.add_state::<T>();
        let initial_state = self.when_editor.clone();
        app.add_startup_system(move |mut game_state: ResMut<NextState<T>>| {
            game_state.set(initial_state.clone());
        });
        let when_editor = self.when_editor.clone();
        let when_game = self.when_game.clone();
        app.add_system(
            move |editor_state: Res<State<YoleckEditorState>>,
                  mut game_state: ResMut<NextState<T>>| {
                game_state.set(match editor_state.0 {
                    YoleckEditorState::EditorActive => when_editor.clone(),
                    YoleckEditorState::GameActive => when_game.clone(),
                });
            },
        );
    }
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
    entity: Entity,
    pub(crate) passed: &'a mut HashMap<Entity, HashMap<TypeId, BoxedArc>>,
    knobs_cache: &'a mut YoleckKnobsCache,
}

impl<'a> YoleckEditContext<'a> {
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
        if let Some(dynamic) = self
            .passed
            .get(&self.entity)
            .and_then(|m| m.get(&TypeId::of::<T>()))
        {
            dynamic.downcast_ref()
        } else {
            None
        }
    }

    /// Create a knob - an helper entity the level editor can use to edit the entity.
    ///
    /// ```no_run
    /// # use bevy::prelude::*;
    /// # use bevy_yoleck::{YoleckEdit, egui};
    /// # struct KidWithBalloon {
    /// #     position: Vec2,
    /// #     baloon_offset: Vec2,
    /// # }
    /// # #[derive(Resource)]
    /// # struct MyAssets {
    /// #     baloon_sprite: Handle<Image>,
    /// # }
    /// fn edit_kid_with_balloon(mut edit: YoleckEdit<KidWithBalloon>, mut commands: Commands, assets: Res<MyAssets>) {
    ///     edit.edit(|ctx, data, _ui| {
    ///         let mut balloon_knob = ctx.knob(&mut commands, "balloon");
    ///         let knob_position = data.position + data.baloon_offset;
    ///         balloon_knob.cmd.insert(SpriteBundle {
    ///             transform: Transform::from_translation(knob_position.extend(1.0)),
    ///             texture: assets.baloon_sprite.clone(),
    ///             ..Default::default()
    ///         });
    ///         if let Some(new_baloon_pos) = balloon_knob.get_passed_data::<Vec3>() {
    ///             data.baloon_offset = new_baloon_pos.truncate() - data.position;
    ///         }
    ///     });
    /// }
    /// ```
    pub fn knob<'b, 'w, 's, K>(
        &mut self,
        commands: &'b mut Commands<'w, 's>,
        key: K,
    ) -> YoleckKnobHandle<'w, 's, 'b>
    where
        K: 'static + Send + Sync + Hash + Eq,
    {
        let KnobFromCache { cmd, is_new } = self.knobs_cache.access(key, commands);
        let passed = self.passed.remove(&cmd.id()).unwrap_or_default();
        YoleckKnobHandle {
            cmd,
            is_new,
            passed,
        }
    }
}

#[doc(hidden)]
#[derive(Resource)]
pub struct YoleckUiForEditSystem(pub egui::Ui);

/// Parameter for systems that edit entities. See [`YoleckEdit::edit`].
#[derive(SystemParam)]
pub struct YoleckEdit<'w, 's, T: 'static> {
    #[allow(dead_code)]
    query: Query<'w, 's, &'static mut YoleckManaged>,
    #[allow(dead_code)]
    context: ResMut<'w, YoleckUserSystemContext>,
    ui: ResMut<'w, YoleckUiForEditSystem>,
    knobs_cache: ResMut<'w, YoleckKnobsCache>,
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
    pub fn edit(&mut self, mut dlg: impl FnMut(&mut YoleckEditContext, &mut T, &mut egui::Ui)) {
        match &mut *self.context {
            YoleckUserSystemContext::Nope
            | YoleckUserSystemContext::PopulateEdited(_)
            | YoleckUserSystemContext::PopulateInitiated { .. } => {
                panic!("Wrong state");
            }
            YoleckUserSystemContext::Edit { entity, passed } => {
                let mut edit_context = YoleckEditContext {
                    entity: *entity,
                    passed,
                    knobs_cache: &mut self.knobs_cache,
                };
                let mut yoleck_managed = self
                    .query
                    .get_mut(*entity)
                    .expect("Edited entity does not exist");
                let data = yoleck_managed
                    .data
                    .downcast_mut::<T>()
                    .expect("Edited data is of wrong type");
                dlg(&mut edit_context, data, &mut self.ui.0);
            }
        }
    }
}

/// An handle for intearcing with a knob from an [edit system](YoleckEdit::edit).
pub struct YoleckKnobHandle<'w, 's, 'a> {
    /// The command of the knob entity.
    pub cmd: EntityCommands<'w, 's, 'a>,
    /// `true` if the knob entity is just created this frame.
    pub is_new: bool,
    passed: HashMap<TypeId, BoxedArc>,
}

impl YoleckKnobHandle<'_, '_, '_> {
    /// Get data sent to the knob from external systems (usually interaciton from the level
    /// editor). See [`YoleckEditContext::get_passed_data`].
    pub fn get_passed_data<T: 'static>(&self) -> Option<&T> {
        if let Some(dynamic) = self.passed.get(&TypeId::of::<T>()) {
            dynamic.downcast_ref()
        } else {
            None
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
    /// # #[derive(Resource)]
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
    ///         cmd.insert(SpriteBundle {
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

#[derive(Resource)]
pub enum YoleckUserSystemContext {
    Nope,
    Edit {
        entity: Entity,
        passed: HashMap<Entity, HashMap<TypeId, BoxedArc>>,
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

pub trait YoleckComponent: Component + Serialize + for<'a> Deserialize<'a> {
    const KEY: &'static str;
    const VERSION: usize = 1;
}

pub struct YoleckEntityType {
    pub name: String,
    pub(crate) components: Vec<YoleckComponentHandler>,
}

impl YoleckEntityType {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            components: Default::default(),
        }
    }

    pub fn with<T: YoleckComponent>(mut self) -> Self {
        self.components.push(YoleckComponentHandler::new::<T>());
        self
    }
}
