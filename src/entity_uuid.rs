use bevy::prelude::*;

use bevy::utils::{HashMap, Uuid};

#[derive(Component, Deref, Debug)]
pub struct YoleckEntityUuid(pub(crate) Uuid);

#[derive(Resource)]
pub struct YoleckUuidRegistry(pub(crate) HashMap<Uuid, Entity>);

impl YoleckUuidRegistry {
    pub fn get(&self, uuid: Uuid) -> Option<Entity> {
        self.0.get(&uuid).copied()
    }
}
