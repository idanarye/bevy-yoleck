use std::collections::VecDeque;

use bevy::prelude::*;

pub(crate) struct YoleckExclusiveSystemsPlugin;

impl Plugin for YoleckExclusiveSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<YoleckExclusiveSystemsQueue>();
        app.init_resource::<YoleckEntityCreationExclusiveSystems>();
    }
}

/// The result of an exclusive system.
#[derive(Debug)]
pub enum YoleckExclusiveSystemDirective {
    /// An exclusive system needs to return this when it is not done yet and wants to still be
    /// active in the next frame.
    Listening,
    /// An exclusive system needs to return this when it is has nothing more to do.
    ///
    /// This means that either the exclusive system received the input it was waiting for (e.g. - a
    /// user click) or that it is not viable for the currently selected entity.
    Finished,
}

pub type YoleckExclusiveSystem = Box<dyn System<In = (), Out = YoleckExclusiveSystemDirective>>;

/// The currently pending exclusive systems.
///
/// Other edit systems (exclusive or otherwise) may [`push_back`](Self::push_back) exclusive edit
/// systems into this queue:
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::prelude::*;
/// # use bevy_yoleck::exclusive_systems::*;
/// # use bevy_yoleck::vpeol::prelude::*;
/// # #[derive(Component)]
/// # struct LookingAt2D(Vec2);
/// fn regular_edit_system(
///     edit: YoleckEdit<(), With<LookingAt2D>>,
///     mut ui: ResMut<YoleckUi>,
///     mut exclusive_queue: ResMut<YoleckExclusiveSystemsQueue>,
/// ) {
///     if edit.get_single().is_err() {
///         return;
///     }
///     if ui.button("Look At").clicked() {
///         exclusive_queue.push_back(exclusive_system);
///     }
/// }
///
/// fn exclusive_system(
///     mut edit: YoleckEdit<&mut LookingAt2D>,
///     // Getting the actual input is still quite manual. May be chanced in the future.
///     cameras_query: Query<&VpeolCameraState>,
///     ui: ResMut<YoleckUi>,
///     buttons: Res<ButtonInput<MouseButton>>,
/// ) -> YoleckExclusiveSystemDirective {
///     let Ok(mut looking_at) = edit.get_single_mut() else {
///         return YoleckExclusiveSystemDirective::Finished;
///     };
///
///     let Some(cursor_ray) = cameras_query.iter().find_map(|camera_state| camera_state.cursor_ray) else {
///         return YoleckExclusiveSystemDirective::Listening;
///     };
///     looking_at.0 = cursor_ray.origin.truncate();
///
///     if ui.ctx().is_pointer_over_area() {
///         return YoleckExclusiveSystemDirective::Listening;
///     }
///
///     if buttons.just_released(MouseButton::Left) {
///         return YoleckExclusiveSystemDirective::Finished;
///     }
///
///     return YoleckExclusiveSystemDirective::Listening;
/// }
/// ```
#[derive(Resource, Default)]
pub struct YoleckExclusiveSystemsQueue(VecDeque<YoleckExclusiveSystem>);

impl YoleckExclusiveSystemsQueue {
    /// Add an exclusive system to be ran starting from the next frame.
    ///
    /// If there are already exclusive systems running or enqueued, the new one will run after they
    /// finish.
    pub fn push_back<P>(&mut self, system: impl IntoSystem<(), YoleckExclusiveSystemDirective, P>) {
        self.0.push_back(Box::new(IntoSystem::into_system(system)));
    }

    /// Add an exclusive system to be ran starting from the next frame.
    ///
    /// If there are already exclusive systems enqueued, the new one will run before them. If there
    /// is an exclusive system already running, the new one will only run after it finishes.
    pub fn push_front<P>(
        &mut self,
        system: impl IntoSystem<(), YoleckExclusiveSystemDirective, P>,
    ) {
        self.0.push_front(Box::new(IntoSystem::into_system(system)));
    }

    /// Remove all enqueued exclusive systems.
    ///
    /// This does not affect an exclusive system that is already running. That system will keep
    /// running until it returns [`Finished`](YoleckExclusiveSystemDirective::Finished).
    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub(crate) fn pop_front(&mut self) -> Option<YoleckExclusiveSystem> {
        self.0.pop_front()
    }
}

#[derive(Resource)]
pub(crate) struct YoleckActiveExclusiveSystem(pub YoleckExclusiveSystem);

/// The exclusive systems that will run automatically when a new entity is created.
///
/// Note that this may contain exclusive systems that are not relevant for all entities. These
/// exclusive systems are expected to return [`Finished`](YoleckExclusiveSystemDirective::Finished)
/// immediately when they do not apply, so that the next ones would run immediately
#[derive(Default, Resource)]
pub struct YoleckEntityCreationExclusiveSystems(
    #[allow(clippy::type_complexity)]
    Vec<Box<dyn Sync + Send + Fn(&mut YoleckExclusiveSystemsQueue)>>,
);

impl YoleckEntityCreationExclusiveSystems {
    /// Add a modification to the exclusive systems queue when new entities are created.
    pub fn on_entity_creation(
        &mut self,
        dlg: impl 'static + Sync + Send + Fn(&mut YoleckExclusiveSystemsQueue),
    ) {
        self.0.push(Box::new(dlg));
    }

    pub(crate) fn create_queue(&self) -> YoleckExclusiveSystemsQueue {
        let mut queue = YoleckExclusiveSystemsQueue::default();
        for dlg in self.0.iter() {
            dlg(&mut queue);
        }
        queue
    }
}
