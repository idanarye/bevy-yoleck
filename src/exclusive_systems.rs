use bevy::prelude::*;

#[derive(Debug)]
pub enum YoleckExclusiveSystemDirective {
    Listening,
    Finished,
}

// type YoleckExclusiveSystem = Box<dyn System<In = (), Out = YoleckExclusiveSystemDirective>>;
type YoleckExclusiveSystem = Box<dyn System<In = (), Out = YoleckExclusiveSystemDirective>>;

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
}

#[derive(Resource)]
pub(crate) struct YoleckActiveExclusiveSystem(pub YoleckExclusiveSystem);
