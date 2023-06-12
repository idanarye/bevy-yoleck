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
/// Other edit systems (exclusive or otherwise) may [`enqueue`](Self::enqueue) exclusive edit
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
///     edit: YoleckEdit<With<LookingAt2D>>,
///     mut ui: ResMut<YoleckUi>,
///     mut exclusive_queue: ResMut<YoleckExclusiveSystemsQueue>,
/// ) {
///     if edit.get_single().is_err() {
///         return;
///     }
///     if ui.button("Look At").clicked() {
///         exclusive_queue.enqueue(exclusive_system);
///     }
/// }
///
/// fn exclusive_system(
///     mut edit: YoleckEdit<&mut LookingAt2D>,
///     // Getting the actual input is still quite manual. May be chanced in the future.
///     cameras_query: Query<&VpeolCameraState>,
///     ui: ResMut<YoleckUi>,
///     buttons: Res<Input<MouseButton>>,
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
pub struct YoleckExclusiveSystemsQueue(Vec<YoleckExclusiveSystem>);

impl YoleckExclusiveSystemsQueue {
    /// Add an exclusive system to be ran starting from the next frame.
    ///
    /// If there are already exclusive systems running or enqueued, the new one will run after they
    /// finish.
    pub fn enqueue<P>(&mut self, system: impl IntoSystem<(), YoleckExclusiveSystemDirective, P>) {
        self.0.push(Box::new(IntoSystem::into_system(system)));
    }

    pub(crate) fn take(&mut self) -> Option<YoleckExclusiveSystem> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    pub(crate) fn set(&mut self, exclusive_systems: impl Iterator<Item = YoleckExclusiveSystem>) {
        self.0 = exclusive_systems.collect();
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
    Vec<Box<dyn Sync + Send + Fn() -> YoleckExclusiveSystem>>,
);

impl YoleckEntityCreationExclusiveSystems {
    /// Add a new exclusive system that would run on entity creation.
    ///
    /// The system is provided as a closure that returns a system, and will run before all the
    /// systems that were added before it.
    pub fn push_first<P, T: IntoSystem<(), YoleckExclusiveSystemDirective, P>>(
        &mut self,
        factory: impl 'static + Sync + Send + Fn() -> T,
    ) {
        self.0.insert(
            0,
            Box::new(move || Box::new(IntoSystem::into_system(factory()))),
        );
    }

    /// Add a new exclusive system that would run on entity creation.
    ///
    /// The system is provided as a closure that returns a system, and will run after all the
    /// systems that were added before it.
    pub fn push_last<P, T: IntoSystem<(), YoleckExclusiveSystemDirective, P>>(
        &mut self,
        factory: impl 'static + Sync + Send + Fn() -> T,
    ) {
        self.0.push(Box::new(move || {
            Box::new(IntoSystem::into_system(factory()))
        }));
    }

    /// Generate all the systems that are supposed to run when an entity is created.
    pub fn generate(&self) -> impl '_ + Iterator<Item = YoleckExclusiveSystem> {
        self.0.iter().map(|factory| factory())
    }
}
