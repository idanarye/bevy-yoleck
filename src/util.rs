use bevy::prelude::*;

trait ContextSpecificResource: 'static + Sync + Send {
    fn inject_to_world(&mut self, world: &mut World);
    fn take_from_world(&mut self, world: &mut World);
}

impl<T> ContextSpecificResource for Option<T>
where
    T: Resource,
{
    fn inject_to_world(&mut self, world: &mut World) {
        world.insert_resource(self.take().unwrap());
    }

    fn take_from_world(&mut self, world: &mut World) {
        *self = world.remove_resource();
    }
}

#[derive(Resource)]
pub(crate) struct EditSpecificResources(Vec<Box<dyn ContextSpecificResource>>);

impl EditSpecificResources {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with(mut self, resource: impl Resource) -> Self {
        self.0.push(Box::new(Some(resource)));
        self
    }

    pub fn inject_to_world(&mut self, world: &mut World) {
        for resource in self.0.iter_mut() {
            resource.inject_to_world(world);
        }
    }

    pub fn take_from_world(&mut self, world: &mut World) {
        for resource in self.0.iter_mut().rev() {
            resource.take_from_world(world);
        }
    }
}
