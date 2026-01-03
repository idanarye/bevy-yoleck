use bevy::prelude::*;
use bevy_egui::egui;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auto_edit::YoleckAutoEdit;
use crate::entity_uuid::YoleckUuidRegistry;

#[cfg(feature = "vpeol")]
use bevy::ecs::component::Mutable;
#[cfg(feature = "vpeol")]
use crate::editing::{YoleckEdit, YoleckUi};
#[cfg(feature = "vpeol")]
use crate::entity_uuid::YoleckEntityUuid;
#[cfg(feature = "vpeol")]
use crate::exclusive_systems::{YoleckExclusiveSystemDirective, YoleckExclusiveSystemsQueue};
#[cfg(feature = "vpeol")]
use crate::{yoleck_exclusive_system_cancellable, YoleckManaged};

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

    pub fn get_entity(&self) -> Option<Entity> {
        self.resolved
    }

    pub fn get_uuid(&self) -> Option<Uuid> {
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

    pub fn resolve(&mut self, registry: &YoleckUuidRegistry) {
        if let Some(uuid) = self.uuid {
            self.resolved = registry.get(uuid);
        } else {
            self.resolved = None;
        }
    }
}

impl YoleckAutoEdit for YoleckEntityRef {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(uuid) = value.uuid {
                ui.label(format!("{}", uuid));
                if ui.small_button("✕").clicked() {
                    value.clear();
                }
            } else {
                ui.label("None");
            }
        });
    }
}

#[cfg(feature = "vpeol")]
fn entity_ref_dropdown_ui(
    ui: &mut egui::Ui,
    current_uuid: Option<Uuid>,
    entities: &[(Entity, Uuid, String, String)],
    filter: Option<&str>,
) -> Option<Option<Uuid>> {
    let filtered: Vec<_> = entities
        .iter()
        .filter(|(_, _, type_name, _)| filter.map_or(true, |f| type_name == f))
        .collect();

    let current_label = current_uuid
        .and_then(|uuid| {
            filtered
                .iter()
                .find(|(_, u, _, _)| *u == uuid)
                .map(|(_, _, type_name, name)| {
                    if name.is_empty() {
                        format!(
                            "{} ({})",
                            type_name,
                            uuid.to_string().chars().take(8).collect::<String>()
                        )
                    } else {
                        format!("{} - {}", type_name, name)
                    }
                })
        })
        .unwrap_or_else(|| "None".to_string());

    let mut result = None;

    egui::ComboBox::from_id_salt("entity_ref_dropdown")
        .selected_text(current_label)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(current_uuid.is_none(), "None")
                .clicked()
            {
                result = Some(None);
            }
            for (_, uuid, type_name, name) in filtered.iter() {
                let label = if name.is_empty() {
                    format!(
                        "{} ({})",
                        type_name,
                        uuid.to_string().chars().take(8).collect::<String>()
                    )
                } else {
                    format!("{} - {}", type_name, name)
                };
                if ui
                    .selectable_label(current_uuid == Some(*uuid), label)
                    .clicked()
                {
                    result = Some(Some(*uuid));
                }
            }
        });

    result
}

pub trait YoleckEntityRefAccessor: Sized + Send + Sync + 'static {
    fn entity_ref_fields() -> &'static [(&'static str, Option<&'static str>)];
    fn get_entity_ref_mut(&mut self, field_name: &str) -> &mut YoleckEntityRef;
}

#[cfg(feature = "vpeol")]
#[derive(Resource, Default)]
pub(crate) struct YoleckEntityRefRequirements {
    pub requirements: Vec<(String, String, String)>, // (component_type, field_name, required_entity_type)
}

#[cfg(feature = "vpeol")]
pub(crate) fn validate_entity_ref_requirements(
    requirements: Res<YoleckEntityRefRequirements>,
    construction_specs: Res<crate::YoleckEntityConstructionSpecs>,
) {
    for (component_type, field_name, required_entity_type) in &requirements.requirements {
        if let Some(entity_type_info) = construction_specs.get_entity_type_info(required_entity_type) {
            if !entity_type_info.has_uuid {
                error!(
                    "Entity reference field '{}' in component '{}' requires entity type '{}' to have UUID, \
                     but it was registered without .with_uuid(). \
                     Add .with_uuid() when calling YoleckEntityType::new(\"{}\") in add_yoleck_entity_type().",
                    field_name,
                    component_type,
                    required_entity_type,
                    required_entity_type
                );
            }
        }
    }
}

#[cfg(feature = "vpeol")]
pub fn edit_entity_refs_system<T: Component<Mutability = Mutable> + YoleckEntityRefAccessor>(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<(Entity, &mut T)>,
    entities_query: Query<(Entity, &YoleckEntityUuid, &YoleckManaged)>,
    mut exclusive_queue: ResMut<YoleckExclusiveSystemsQueue>,
) {
    let Ok((_source_entity, mut component)) = edit.single_mut() else {
        return;
    };

    let entities: Vec<_> = entities_query
        .iter()
        .map(|(entity, uuid, managed)| {
            (
                entity,
                uuid.get(),
                managed.type_name.clone(),
                managed.name.clone(),
            )
        })
        .collect();

    for (field_name, filter) in T::entity_ref_fields() {
        let entity_ref = component.get_entity_ref_mut(field_name);
        let current_uuid = entity_ref.uuid;

        let response = ui.horizontal(|ui| {
            ui.label(*field_name);

            if let Some(new_value) =
                entity_ref_dropdown_ui(ui, current_uuid, &entities, filter.as_deref())
            {
                let entity_ref = component.get_entity_ref_mut(field_name);
                if let Some(uuid) = new_value {
                    entity_ref.set(uuid);
                } else {
                    entity_ref.clear();
                }
            }

            if ui
                .button("🎯")
                .on_hover_text("Click to select in viewport")
                .clicked()
            {
                let field_name_owned = field_name.to_string();
                exclusive_queue.push_back(
                    crate::vpeol::vpeol_read_click_on_entity::<With<YoleckEntityUuid>>
                        .pipe(crate::yoleck_map_entity_to_uuid)
                        .pipe(yoleck_entity_ref_select_handler::<T>(field_name_owned))
                        .pipe(yoleck_exclusive_system_cancellable),
                );
            }

            let entity_ref = component.get_entity_ref_mut(field_name);
            if entity_ref.is_some() && ui.small_button("✕").clicked() {
                entity_ref.clear();
            }
        }).response;

        if let Some(dropped_uuid) = response.dnd_release_payload::<Uuid>() {
            let dropped_uuid = *dropped_uuid;
            let entity_ref = component.get_entity_ref_mut(field_name);
            if filter.is_some() {
                if let Some((_, _, type_name, _)) = entities.iter().find(|(_, uuid, _, _)| *uuid == dropped_uuid) {
                    if filter.as_deref() == Some(type_name) {
                        entity_ref.set(dropped_uuid);
                    }
                }
            } else {
                entity_ref.set(dropped_uuid);
            }
        }
    }
}

#[cfg(feature = "vpeol")]
fn yoleck_entity_ref_select_handler<T: Component<Mutability = Mutable> + YoleckEntityRefAccessor>(
    field_name: String,
) -> impl Fn(In<Option<Uuid>>, YoleckEdit<&mut T>) -> YoleckExclusiveSystemDirective {
    move |In(target): In<Option<Uuid>>, mut edit: YoleckEdit<&mut T>| {
        let Ok(mut component) = edit.single_mut() else {
            return YoleckExclusiveSystemDirective::Finished;
        };

        if let Some(uuid) = target {
            let entity_ref = component.get_entity_ref_mut(&field_name);
            entity_ref.set(uuid);
            YoleckExclusiveSystemDirective::Finished
        } else {
            YoleckExclusiveSystemDirective::Listening
        }
    }
}
