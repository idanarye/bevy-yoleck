use bevy::prelude::*;

pub(crate) struct YoleckExclusiveSystemsPlugin;

impl Plugin for YoleckExclusiveSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<YoleckExclusiveSystemsQueue>();
        app.init_resource::<YoleckEntityCreationExclusiveSystems>();
    }
}

#[derive(Debug)]
pub enum YoleckExclusiveSystemDirective {
    Listening,
    Finished,
}

// type YoleckExclusiveSystem = Box<dyn System<In = (), Out = YoleckExclusiveSystemDirective>>;
pub type YoleckExclusiveSystem = Box<dyn System<In = (), Out = YoleckExclusiveSystemDirective>>;

#[derive(Resource, Default)]
pub struct YoleckExclusiveSystemsQueue(Vec<YoleckExclusiveSystem>);

impl YoleckExclusiveSystemsQueue {
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

#[derive(Default, Resource)]
pub struct YoleckEntityCreationExclusiveSystems(
    Vec<Box<dyn Sync + Send + Fn() -> YoleckExclusiveSystem>>,
);

impl YoleckEntityCreationExclusiveSystems {
    pub fn push_first<P, T: IntoSystem<(), YoleckExclusiveSystemDirective, P>>(
        &mut self,
        factory: impl 'static + Sync + Send + Fn() -> T,
    ) {
        self.0.insert(
            0,
            Box::new(move || Box::new(IntoSystem::into_system(factory()))),
        );
    }

    pub fn generate(&self) -> impl '_ + Iterator<Item = YoleckExclusiveSystem> {
        self.0.iter().map(|factory| factory())
    }
}
