use std::any::TypeId;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use bevy::ecs::query::{ReadOnlyWorldQuery, WorldQuery};
use bevy::ecs::system::{EntityCommands, SystemParam};
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_egui::egui;
use serde::{Deserialize, Serialize};

use crate::entity_management::EntitiesToPopulate;
use crate::knobs::{KnobFromCache, YoleckKnobsCache};
use crate::{BoxedArc, YoleckComponentHandler};

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

#[allow(unused)]
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

pub trait YoleckComponent: Default + Component + Serialize + for<'a> Deserialize<'a> {
    const KEY: &'static str;
}

pub struct YoleckEntityType {
    pub name: String,
    pub(crate) components: Vec<YoleckComponentHandler>,
    #[allow(clippy::type_complexity)]
    pub(crate) on_init:
        Vec<Box<dyn 'static + Sync + Send + Fn(YoleckEditorState, &mut EntityCommands)>>,
}

impl YoleckEntityType {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            components: Default::default(),
            on_init: Default::default(),
        }
    }

    pub fn with<T: YoleckComponent>(mut self) -> Self {
        self.components.push(YoleckComponentHandler::new::<T>());
        self
    }

    pub fn insert_on_init<T: Bundle>(
        mut self,
        bundle_maker: impl 'static + Sync + Send + Fn() -> T,
    ) -> Self {
        self.on_init.push(Box::new(move |_, cmd| {
            cmd.insert(bundle_maker());
        }));
        self
    }

    pub fn insert_on_init_during_editor<T: Bundle>(
        mut self,
        bundle_maker: impl 'static + Sync + Send + Fn() -> T,
    ) -> Self {
        self.on_init.push(Box::new(move |editor_state, cmd| {
            if matches!(editor_state, YoleckEditorState::EditorActive) {
                cmd.insert(bundle_maker());
            }
        }));
        self
    }

    pub fn insert_on_init_during_game<T: Bundle>(
        mut self,
        bundle_maker: impl 'static + Sync + Send + Fn() -> T,
    ) -> Self {
        self.on_init.push(Box::new(move |editor_state, cmd| {
            if matches!(editor_state, YoleckEditorState::GameActive) {
                cmd.insert(bundle_maker());
            }
        }));
        self
    }
}

#[derive(Component)]
pub struct YoleckEdit {
    pub(crate) passed_data: HashMap<TypeId, BoxedArc>,
}

impl YoleckEdit {
    pub fn get_passed_data<T: 'static>(&self) -> Option<&T> {
        self.passed_data.get(&TypeId::of::<T>())?.downcast_ref()
    }
}

#[derive(SystemParam)]
pub struct YoleckPopulate<'w, 's, Q: 'static + WorldQuery, F: 'static + ReadOnlyWorldQuery = ()> {
    entities_to_populate: Res<'w, EntitiesToPopulate>,
    query: Query<'w, 's, Q, F>,
    commands: Commands<'w, 's>,
}

impl<Q: 'static + WorldQuery, F: 'static + ReadOnlyWorldQuery> YoleckPopulate<'_, '_, Q, F> {
    pub fn populate(
        &mut self,
        mut dlg: impl FnMut((), EntityCommands, <Q as WorldQuery>::Item<'_>),
    ) {
        for entity in self.entities_to_populate.0.iter() {
            if let Ok(data) = self.query.get_mut(*entity) {
                let cmd = self.commands.entity(*entity);
                dlg((), cmd, data);
            }
        }
    }
}

#[derive(SystemParam)]
pub struct YoleckKnobs<'w, 's> {
    knobs_cache: ResMut<'w, YoleckKnobsCache>,
    commands: Commands<'w, 's>,
    knobs_query: Query<'w, 's, &'static YoleckKnobData>,
}

impl<'w, 's> YoleckKnobs<'w, 's> {
    pub fn knob<'a, K>(&'a mut self, key: K) -> YoleckKnobHandle<'w, 's, 'a>
    where
        K: 'static + Send + Sync + Hash + Eq,
    {
        let KnobFromCache { mut cmd, is_new } = self.knobs_cache.access(key, &mut self.commands);
        let passed;
        if is_new {
            cmd.insert(YoleckKnobData {
                passed_data: Default::default(),
            });
            passed = Default::default();
        } else if let Ok(knob_data) = self.knobs_query.get(cmd.id()) {
            passed = knob_data.passed_data.clone();
        } else {
            passed = Default::default();
        }
        YoleckKnobHandle {
            cmd,
            is_new,
            passed,
        }
    }
}

#[derive(Component)]
pub(crate) struct YoleckKnobData {
    pub(crate) passed_data: HashMap<TypeId, BoxedArc>,
}
