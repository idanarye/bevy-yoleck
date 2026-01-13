use std::any::TypeId;

use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::YoleckManaged;
use crate::entity_uuid::YoleckUuidRegistry;
use crate::errors::YoleckEntityRefCannotBeResolved;

/// A reference to another Yoleck entity, stored by UUID for persistence.
///
/// This allows one entity to reference another entity in a way that survives saving and loading.
/// The reference is stored as a UUID in the level file, which gets resolved to an actual `Entity`
/// at runtime.
///
/// # Requirements
///
/// **Important:** Only entities with `.with_uuid()` can be referenced. When defining entity types
/// that should be referenceable, make sure to add `.with_uuid()` to the entity type:
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::prelude::*;
/// # let mut app = App::new();
/// app.add_yoleck_entity_type({
///     YoleckEntityType::new("Planet")
///         .with_uuid()  // Required for entity references!
///         // ... other configuration
/// #       ;YoleckEntityType::new("Planet")
/// });
/// ```
///
/// # Editor Features
///
/// In the editor, entity references can be set using:
/// - Dropdown menu to select from available entities
/// - Drag and drop from the entity list (only entities with UUID can be dragged)
/// - Viewport click selection using the 🎯 button
///
/// # Usage
///
/// Add a `YoleckEntityRef` field to your component with the `entity_ref` attribute to filter by
/// entity type:
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::prelude::*;
/// # use serde::{Deserialize, Serialize};
/// #[derive(Component, YoleckComponent, YoleckAutoEdit, Serialize, Deserialize, Clone, PartialEq, Default)]
/// struct LaserPointer {
///     #[yoleck(entity_ref = "Planet")]
///     target: YoleckEntityRef,
/// }
/// ```
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Debug)]
pub struct YoleckEntityRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    uuid: Option<Uuid>,
    #[serde(skip)]
    resolved: Option<Entity>,
}

// impl std::hash::Hash for YoleckEntityRef {
// fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
// self.uuid.hash(state);
// }
// }

// impl PartialEq for YoleckEntityRef {
// fn eq(&self, other: &Self) -> bool {
// self.uuid == other.uuid
// }
// }

impl YoleckEntityRef {
    pub fn new() -> Self {
        Self {
            uuid: None,
            resolved: None,
        }
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self {
            uuid: Some(uuid),
            resolved: None,
        }
    }

    pub fn is_some(&self) -> bool {
        self.uuid.is_some()
    }

    pub fn is_none(&self) -> bool {
        self.uuid.is_none()
    }

    pub fn entity(&self) -> Option<Entity> {
        self.resolved
    }

    pub fn uuid(&self) -> Option<Uuid> {
        self.uuid
    }

    pub fn clear(&mut self) {
        self.uuid = None;
        self.resolved = None;
    }

    pub fn set(&mut self, uuid: Uuid) {
        self.uuid = Some(uuid);
        self.resolved = None;
    }

    pub fn resolve(
        &mut self,
        registry: &YoleckUuidRegistry,
    ) -> Result<(), YoleckEntityRefCannotBeResolved> {
        if let Some(uuid) = self.uuid {
            self.resolved = registry.get(uuid);
            if self.resolved.is_none() {
                return Err(YoleckEntityRefCannotBeResolved { uuid });
            }
        } else {
            self.resolved = None;
        }
        Ok(())
    }
}

pub trait YoleckEntityRefAccessor: Sized + Send + Sync + 'static {
    fn entity_ref_fields() -> &'static [(&'static str, Option<&'static str>)];
    fn get_entity_ref_mut(&mut self, field_name: &str) -> &mut YoleckEntityRef;
    // TODO: make this more versatile
    fn resolve_entity_refs(&mut self, registry: &YoleckUuidRegistry);
}

pub(crate) fn validate_entity_ref_requirements_for<T: YoleckEntityRefAccessor>(
    construction_specs: &crate::YoleckEntityConstructionSpecs,
) {
    for (field_name, filter) in T::entity_ref_fields() {
        if let Some(required_entity_type) = filter
            && let Some(entity_type_info) =
                construction_specs.get_entity_type_info(required_entity_type)
            && !entity_type_info.has_uuid
        {
            error!(
                "Component '{}' field '{}' requires entity type '{}' to have UUID.",
                std::any::type_name::<T>(),
                field_name,
                required_entity_type
            );
        }
    }
}

pub fn resolve_entity_refs<
    T: 'static + Component<Mutability = Mutable> + YoleckEntityRefAccessor,
>(
    mut query: Query<(&mut T, &mut YoleckManaged)>,
    registry: Res<YoleckUuidRegistry>,
) {
    for (mut component, mut managed) in query.iter_mut() {
        component.resolve_entity_refs(registry.as_ref());
        if let Some(data) = managed.components_data.get_mut(&TypeId::of::<T>())
            && let Some(data) = data.downcast_mut::<T>()
        {
            data.resolve_entity_refs(registry.as_ref());
        }
    }
}
