use bevy::prelude::*;

use bevy::utils::HashMap;
use uuid::Uuid;

/// A UUID automatically added to entity types defined with
/// [`with_uuid`](crate::YoleckEntityType::with_uuid)
///
/// This UUID can be used to refer to the entity in a persistent way - e.g. from a
/// [`YoleckComponent`](crate::prelude::YoleckComponent) of another entity. The `Entity` ID itself
/// will change between runs, but the UUID can reliably store the connection between the entities
/// in the `.yol` file.
///
/// To find an entity by UUID use [`YoleckUuidRegistry`].
#[derive(Component, Debug)]
pub struct YoleckEntityUuid(pub(crate) Uuid);

impl YoleckEntityUuid {
    pub fn get(&self) -> Uuid {
        self.0
    }
}

/// Helper registry for finding [`with_uuid`](crate::YoleckEntityType::with_uuid) defined entities
/// by their UUID.
///
/// To find a UUID given the `Entity` - check its [`YoleckEntityUuid`] component.
#[derive(Resource)]
pub struct YoleckUuidRegistry(pub(crate) HashMap<Uuid, Entity>);

impl YoleckUuidRegistry {
    pub fn get(&self, uuid: Uuid) -> Option<Entity> {
        self.0.get(&uuid).copied()
    }
}
