use bevy::prelude::*;
use bevy_egui::egui;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auto_edit::YoleckAutoEdit;
use crate::entity_uuid::YoleckUuidRegistry;

#[cfg(feature = "vpeol")]
use crate::editing::{YoleckEdit, YoleckUi};
#[cfg(feature = "vpeol")]
use crate::entity_uuid::YoleckEntityUuid;
#[cfg(feature = "vpeol")]
use crate::exclusive_systems::{YoleckExclusiveSystemDirective, YoleckExclusiveSystemsQueue};
#[cfg(feature = "vpeol")]
use crate::{yoleck_exclusive_system_cancellable, YoleckManaged};
#[cfg(feature = "vpeol")]
use bevy::ecs::component::Mutable;

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

pub trait YoleckEntityRefAccessor: Sized + Send + Sync + 'static {
    fn entity_ref_fields() -> &'static [(&'static str, Option<&'static str>)];
    fn get_entity_ref_mut(&mut self, field_name: &str) -> &mut YoleckEntityRef;
}

#[cfg(feature = "vpeol")]
#[derive(Resource, Default)]
pub(crate) struct YoleckEntityRefRequirements {
    pub requirements: Vec<EntityRefRequirement>,
}

#[derive(Debug, Clone)]
pub struct EntityRefRequirement {
    pub component_type: String,
    pub field_name: String,
    pub required_entity_type: String,
}

#[cfg(feature = "vpeol")]
pub(crate) fn validate_entity_ref_requirements(
    requirements: Res<YoleckEntityRefRequirements>,
    construction_specs: Res<crate::YoleckEntityConstructionSpecs>,
) {
    for requirements in &requirements.requirements {
        if let Some(entity_type_info) =
            construction_specs.get_entity_type_info(&requirements.required_entity_type)
        {
            if !entity_type_info.has_uuid {
                error!(
                    "Entity reference field '{}' in component '{}' requires entity type '{}' to have UUID, \
                     but it was registered without .with_uuid(). \
                     Add .with_uuid() when calling YoleckEntityType::new(\"{}\") in add_yoleck_entity_type().",
                    requirements.field_name,
                    requirements.component_type,
                    requirements.required_entity_type,
                    requirements.required_entity_type
                );
            }
        }
    }
}

#[cfg(feature = "vpeol")]
pub fn edit_entity_refs_system<T: Component<Mutability = Mutable> + YoleckEntityRefAccessor>(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<&mut T>,
    entities_query: Query<(Entity, &YoleckEntityUuid, &YoleckManaged)>,
    mut exclusive_queue: ResMut<YoleckExclusiveSystemsQueue>,
) {
    let Ok(mut component) = edit.single_mut() else {
        return;
    };

    for (field_name, filter) in T::entity_ref_fields() {
        let entity_ref = component.get_entity_ref_mut(field_name);

        let response = ui
            .horizontal(|ui| {
                ui.label(*field_name);

                let current_label = if let Some(entity) = entity_ref.resolved {
                    if let Ok(managed) = entities_query.get(entity) {
                        let name = &managed.2.name;
                        let type_name = &managed.2.type_name;
                        let uuid_short = entity_ref
                            .get_uuid()
                            .map(|u| u.to_string().chars().take(8).collect::<String>())
                            .unwrap_or_else(|| "None".to_string());

                        if name.is_empty() {
                            format!("{} ({})", type_name, uuid_short)
                        } else {
                            format!("{} - {}", type_name, name)
                        }
                    } else {
                        "Unknown".to_string()
                    }
                } else {
                    entity_ref
                        .uuid
                        .map_or("None".to_string(), |uuid| uuid.to_string())
                };

                let mut selection_changed = None;

                egui::ComboBox::from_id_salt(format!("entity_ref_dropdown_{}", field_name))
                    .selected_text(current_label)
                    .show_ui(ui, |ui| {
                        let entities: Vec<_> = entities_query
                            .iter()
                            .filter(|(_, _, managed)| {
                                filter.map_or(true, |f| f == managed.type_name)
                            })
                            .collect();

                        if ui.selectable_label(entity_ref.is_none(), "None").clicked() {
                            selection_changed = Some(None);
                        }

                        for (_, uuid, managed) in entities {
                            let label = if managed.name.is_empty() {
                                format!("{} ({})", managed.type_name, uuid.get())
                            } else {
                                format!("{} - {}", managed.type_name, managed.name)
                            };
                            if ui
                                .selectable_label(entity_ref.get_uuid() == Some(uuid.get()), label)
                                .clicked()
                            {
                                selection_changed = Some(Some(uuid));
                            }
                        }
                    });

                if let Some(new_uuid) = selection_changed {
                    match new_uuid {
                        Some(uuid) => entity_ref.set(uuid.get()),
                        None => entity_ref.clear(),
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

                if entity_ref.is_some() && ui.small_button("✕").clicked() {
                    entity_ref.clear();
                }
            })
            .response;

        if let Some(dropped_uuid) = response.dnd_release_payload::<Uuid>() {
            let dropped_uuid = *dropped_uuid;
            let entity_ref = component.get_entity_ref_mut(field_name);
            if filter.is_some() {
                if let Some((_, _, managed)) = entities_query
                    .iter()
                    .find(|(_, uuid, _)| uuid.get() == dropped_uuid)
                {
                    if filter.map_or(true, |f| f == managed.type_name) {
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
fn yoleck_entity_ref_select_handler<
    T: Component<Mutability = Mutable> + YoleckEntityRefAccessor,
>(
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
