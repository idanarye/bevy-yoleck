use bevy::prelude::*;
use bevy_egui::egui;

#[cfg(feature = "vpeol")]
use crate::entity_ref::validate_entity_ref_requirements_for;

#[cfg(feature = "vpeol")]
use crate::entity_ref::YoleckEntityRef;

#[cfg(feature = "vpeol")]
use std::collections::HashMap;

/// Attributes that can be applied to fields for customizing their UI
#[derive(Default, Clone)]
pub struct FieldAttrs {
    pub label: Option<String>,
    pub tooltip: Option<String>,
    pub range: Option<(f64, f64)>,
    pub speed: Option<f64>,
    pub readonly: bool,
    pub multiline: bool,
    pub entity_filter: Option<String>,
}

pub trait YoleckAutoEdit: Send + Sync + 'static {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui);

    /// Auto-edit with field-level attributes (label, tooltip, range, etc.)
    /// Default implementation wraps auto_edit with label and common decorations
    fn auto_edit_with_label_and_attrs(
        value: &mut Self,
        ui: &mut egui::Ui,
        label: &str,
        attrs: &FieldAttrs,
    ) {
        if attrs.readonly {
            ui.add_enabled_ui(false, |ui| {
                Self::auto_edit_field_impl(value, ui, label, attrs);
            });
        } else {
            Self::auto_edit_field_impl(value, ui, label, attrs);
        }
    }

    /// Internal implementation for field rendering with label
    /// Types can override this to customize behavior based on attributes
    fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
        ui.horizontal(|ui| {
            ui.label(label);
            let response = ui
                .scope(|ui| {
                    Self::auto_edit(value, ui);
                })
                .response;

            if let Some(tooltip) = &attrs.tooltip {
                response.on_hover_text(tooltip);
            }
        });
    }
}

pub fn render_auto_edit_value<T: YoleckAutoEdit>(ui: &mut egui::Ui, value: &mut T) {
    T::auto_edit(value, ui);
}

impl YoleckAutoEdit for f32 {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(value).speed(0.1));
    }

    fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
        ui.horizontal(|ui| {
            ui.label(label);
            let response = if let Some((min, max)) = attrs.range {
                ui.add(egui::Slider::new(value, min as f32..=max as f32))
            } else {
                let speed = attrs.speed.unwrap_or(0.1) as f32;
                ui.add(egui::DragValue::new(value).speed(speed))
            };

            if let Some(tooltip) = &attrs.tooltip {
                response.on_hover_text(tooltip);
            }
        });
    }
}

impl YoleckAutoEdit for f64 {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(value).speed(0.1));
    }

    fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
        ui.horizontal(|ui| {
            ui.label(label);
            let response = if let Some((min, max)) = attrs.range {
                ui.add(egui::Slider::new(value, min..=max))
            } else {
                let speed = attrs.speed.unwrap_or(0.1);
                ui.add(egui::DragValue::new(value).speed(speed))
            };

            if let Some(tooltip) = &attrs.tooltip {
                response.on_hover_text(tooltip);
            }
        });
    }
}

macro_rules! impl_auto_edit_for_integer {
    ($($ty:ty),*) => {
        $(
            impl YoleckAutoEdit for $ty {
                fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
                    ui.add(egui::DragValue::new(value).speed(1.0));
                }

                fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        let response = if let Some((min, max)) = attrs.range {
                            ui.add(egui::Slider::new(value, min as $ty..=max as $ty))
                        } else {
                            let speed = attrs.speed.unwrap_or(1.0) as f32;
                            ui.add(egui::DragValue::new(value).speed(speed))
                        };

                        if let Some(tooltip) = &attrs.tooltip {
                            response.on_hover_text(tooltip);
                        }
                    });
                }
            }
        )*
    };
}

impl_auto_edit_for_integer!(i32, i64, u32, u64, usize, isize);

impl YoleckAutoEdit for bool {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.checkbox(value, "");
    }

    fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
        ui.horizontal(|ui| {
            let response = ui.checkbox(value, label);

            if let Some(tooltip) = &attrs.tooltip {
                response.on_hover_text(tooltip);
            }
        });
    }
}

impl YoleckAutoEdit for String {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.text_edit_singleline(value);
    }

    fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
        if attrs.multiline {
            ui.label(label);
            let response = ui.text_edit_multiline(value);

            if let Some(tooltip) = &attrs.tooltip {
                response.on_hover_text(tooltip);
            }
        } else {
            ui.horizontal(|ui| {
                ui.label(label);
                let response = ui.text_edit_singleline(value);

                if let Some(tooltip) = &attrs.tooltip {
                    response.on_hover_text(tooltip);
                }
            });
        }
    }
}

impl YoleckAutoEdit for Vec2 {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut value.x).prefix("x: ").speed(0.1));
            ui.add(egui::DragValue::new(&mut value.y).prefix("y: ").speed(0.1));
        });
    }

    fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
        let speed = attrs.speed.unwrap_or(0.1) as f32;
        let response = ui
            .horizontal(|ui| {
                ui.label(label);
                ui.add(
                    egui::DragValue::new(&mut value.x)
                        .prefix("x: ")
                        .speed(speed),
                );
                ui.add(
                    egui::DragValue::new(&mut value.y)
                        .prefix("y: ")
                        .speed(speed),
                );
            })
            .response;

        if let Some(tooltip) = &attrs.tooltip {
            response.on_hover_text(tooltip);
        }
    }
}

impl YoleckAutoEdit for Vec3 {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut value.x).prefix("x: ").speed(0.1));
            ui.add(egui::DragValue::new(&mut value.y).prefix("y: ").speed(0.1));
            ui.add(egui::DragValue::new(&mut value.z).prefix("z: ").speed(0.1));
        });
    }

    fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
        let speed = attrs.speed.unwrap_or(0.1) as f32;
        let response = ui
            .horizontal(|ui| {
                ui.label(label);
                ui.add(
                    egui::DragValue::new(&mut value.x)
                        .prefix("x: ")
                        .speed(speed),
                );
                ui.add(
                    egui::DragValue::new(&mut value.y)
                        .prefix("y: ")
                        .speed(speed),
                );
                ui.add(
                    egui::DragValue::new(&mut value.z)
                        .prefix("z: ")
                        .speed(speed),
                );
            })
            .response;

        if let Some(tooltip) = &attrs.tooltip {
            response.on_hover_text(tooltip);
        }
    }
}

impl YoleckAutoEdit for Vec4 {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut value.x).prefix("x: ").speed(0.1));
            ui.add(egui::DragValue::new(&mut value.y).prefix("y: ").speed(0.1));
            ui.add(egui::DragValue::new(&mut value.z).prefix("z: ").speed(0.1));
            ui.add(egui::DragValue::new(&mut value.w).prefix("w: ").speed(0.1));
        });
    }

    fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
        let speed = attrs.speed.unwrap_or(0.1) as f32;
        let response = ui
            .horizontal(|ui| {
                ui.label(label);
                ui.add(
                    egui::DragValue::new(&mut value.x)
                        .prefix("x: ")
                        .speed(speed),
                );
                ui.add(
                    egui::DragValue::new(&mut value.y)
                        .prefix("y: ")
                        .speed(speed),
                );
                ui.add(
                    egui::DragValue::new(&mut value.z)
                        .prefix("z: ")
                        .speed(speed),
                );
                ui.add(
                    egui::DragValue::new(&mut value.w)
                        .prefix("w: ")
                        .speed(speed),
                );
            })
            .response;

        if let Some(tooltip) = &attrs.tooltip {
            response.on_hover_text(tooltip);
        }
    }
}

impl YoleckAutoEdit for Quat {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        let (mut yaw, mut pitch, mut roll) = value.to_euler(EulerRot::YXZ);
        yaw = yaw.to_degrees();
        pitch = pitch.to_degrees();
        roll = roll.to_degrees();

        ui.horizontal(|ui| {
            let mut changed = false;
            changed |= ui
                .add(
                    egui::DragValue::new(&mut yaw)
                        .prefix("yaw: ")
                        .speed(1.0)
                        .suffix("°"),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut pitch)
                        .prefix("pitch: ")
                        .speed(1.0)
                        .suffix("°"),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut roll)
                        .prefix("roll: ")
                        .speed(1.0)
                        .suffix("°"),
                )
                .changed();

            if changed {
                *value = Quat::from_euler(
                    EulerRot::YXZ,
                    yaw.to_radians(),
                    pitch.to_radians(),
                    roll.to_radians(),
                );
            }
        });
    }

    fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
        let speed = attrs.speed.unwrap_or(1.0) as f32;
        let response = ui
            .horizontal(|ui| {
                ui.label(label);
                let (mut yaw, mut pitch, mut roll) = value.to_euler(EulerRot::YXZ);
                yaw = yaw.to_degrees();
                pitch = pitch.to_degrees();
                roll = roll.to_degrees();

                let mut changed = false;
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut yaw)
                            .prefix("yaw: ")
                            .speed(speed)
                            .suffix("°"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut pitch)
                            .prefix("pitch: ")
                            .speed(speed)
                            .suffix("°"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut roll)
                            .prefix("roll: ")
                            .speed(speed)
                            .suffix("°"),
                    )
                    .changed();

                if changed {
                    *value = Quat::from_euler(
                        EulerRot::YXZ,
                        yaw.to_radians(),
                        pitch.to_radians(),
                        roll.to_radians(),
                    );
                }
            })
            .response;

        if let Some(tooltip) = &attrs.tooltip {
            response.on_hover_text(tooltip);
        }
    }
}

impl YoleckAutoEdit for Color {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        let srgba = value.to_srgba();
        let mut color_arr = [srgba.red, srgba.green, srgba.blue, srgba.alpha];
        if ui
            .color_edit_button_rgba_unmultiplied(&mut color_arr)
            .changed()
        {
            *value = Color::srgba(color_arr[0], color_arr[1], color_arr[2], color_arr[3]);
        }
    }

    fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
        let response = ui
            .horizontal(|ui| {
                ui.label(label);
                let srgba = value.to_srgba();
                let mut color_arr = [srgba.red, srgba.green, srgba.blue, srgba.alpha];
                if ui
                    .color_edit_button_rgba_unmultiplied(&mut color_arr)
                    .changed()
                {
                    *value = Color::srgba(color_arr[0], color_arr[1], color_arr[2], color_arr[3]);
                }
            })
            .response;

        if let Some(tooltip) = &attrs.tooltip {
            response.on_hover_text(tooltip);
        }
    }
}

impl<T: YoleckAutoEdit + Default> YoleckAutoEdit for Option<T> {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut has_value = value.is_some();
            if ui.checkbox(&mut has_value, "").changed() {
                if has_value {
                    *value = Some(T::default());
                } else {
                    *value = None;
                }
            }
            if let Some(inner) = value.as_mut() {
                T::auto_edit(inner, ui);
            }
        });
    }

    fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
        let response = ui
            .horizontal(|ui| {
                ui.label(label);
                let mut has_value = value.is_some();
                if ui.checkbox(&mut has_value, "").changed() {
                    if has_value {
                        *value = Some(T::default());
                    } else {
                        *value = None;
                    }
                }
                if let Some(inner) = value.as_mut() {
                    T::auto_edit(inner, ui);
                }
            })
            .response;

        if let Some(tooltip) = &attrs.tooltip {
            response.on_hover_text(tooltip);
        }
    }
}

impl<T: YoleckAutoEdit + Default> YoleckAutoEdit for Vec<T> {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        let mut to_remove = None;
        for (idx, item) in value.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("[{}]", idx));
                T::auto_edit(item, ui);
                if ui.small_button("−").clicked() {
                    to_remove = Some(idx);
                }
            });
        }
        if let Some(idx) = to_remove {
            value.remove(idx);
        }
        if ui.small_button("+").clicked() {
            value.push(T::default());
        }
    }

    fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
        let response = ui.collapsing(label, |ui| {
            let mut to_remove = None;
            for (idx, item) in value.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("[{}]", idx));
                    T::auto_edit(item, ui);
                    if ui.small_button("−").clicked() {
                        to_remove = Some(idx);
                    }
                });
            }
            if let Some(idx) = to_remove {
                value.remove(idx);
            }
            if ui.small_button("+").clicked() {
                value.push(T::default());
            }
        });

        if let Some(tooltip) = &attrs.tooltip {
            response.header_response.on_hover_text(tooltip);
        }
    }
}

impl<T: YoleckAutoEdit> YoleckAutoEdit for [T] {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        for (idx, item) in value.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("[{}]", idx));
                T::auto_edit(item, ui);
            });
        }
    }
}

#[cfg(feature = "vpeol")]
#[derive(Clone)]
struct EntityRefDisplayInfo {
    pub type_name: String,
    pub name: String,
}

#[cfg(feature = "vpeol")]
impl YoleckAutoEdit for YoleckEntityRef {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(uuid) = value.uuid() {
                ui.label(uuid.to_string());
                if ui.small_button("✕").clicked() {
                    value.clear();
                }
            } else {
                ui.label("None");
            }
        });
    }

    fn auto_edit_field_impl(value: &mut Self, ui: &mut egui::Ui, label: &str, attrs: &FieldAttrs) {
        // Get entity info map once for both display and drag&drop validation
        let entity_info_map = ui.ctx().data(|data| {
            data.get_temp::<HashMap<uuid::Uuid, EntityRefDisplayInfo>>(egui::Id::new(
                "yoleck_entity_ref_display_info",
            ))
        });

        let response = ui
            .horizontal(|ui| {
                ui.label(label);

                let display_text = if let Some(uuid) = value.uuid() {
                    if let Some(ref info_map) = entity_info_map {
                        if let Some(info) = info_map.get(&uuid) {
                            if info.name.is_empty() {
                                let uuid_str = uuid.to_string();
                                let uuid_short = &uuid_str[..uuid_str.len().min(8)];
                                format!("{} ({})", info.type_name, uuid_short)
                            } else {
                                format!("{} - {}", info.type_name, info.name)
                            }
                        } else {
                            uuid.to_string()
                        }
                    } else {
                        uuid.to_string()
                    }
                } else {
                    "None".to_string()
                };

                ui.add(
                    egui::Button::new(
                        egui::RichText::new(display_text)
                            .text_style(ui.style().drag_value_text_style.clone()),
                    )
                    .wrap_mode(egui::TextWrapMode::Extend)
                    .min_size(ui.spacing().interact_size),
                );

                if value.is_some() && ui.small_button("✕").clicked() {
                    value.clear();
                }

                if let Some(tooltip) = &attrs.tooltip {
                    ui.label("ⓘ").on_hover_text(tooltip);
                }
            })
            .response;

        // Handle drag & drop
        if let Some(dropped_uuid) = response.dnd_release_payload::<uuid::Uuid>() {
            let dropped_uuid = *dropped_uuid;

            let should_accept = if let Some(filter) = &attrs.entity_filter {
                entity_info_map
                    .as_ref()
                    .and_then(|map| map.get(&dropped_uuid))
                    .is_none_or(|info| &info.type_name == filter)
            } else {
                true
            };

            if should_accept {
                value.set(dropped_uuid);
            }
        }
    }
}

use crate::YoleckExtForApp;
use crate::editing::{YoleckEdit, YoleckUi};
use crate::specs_registration::YoleckComponent;

#[cfg(feature = "vpeol")]
use crate::entity_ref::YoleckEntityRefAccessor;
#[cfg(feature = "vpeol")]
use bevy::ecs::component::Mutable;

#[cfg(feature = "vpeol")]
use crate::YoleckManaged;
#[cfg(feature = "vpeol")]
use crate::entity_uuid::YoleckEntityUuid;

#[cfg(feature = "vpeol")]
pub fn auto_edit_system<T: YoleckComponent + YoleckAutoEdit + YoleckEntityRefAccessor>(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<&mut T>,
    entities_query: Query<(&YoleckEntityUuid, &YoleckManaged)>,
) {
    let Ok(mut component) = edit.single_mut() else {
        return;
    };

    // Populate entity display info in egui context only if component has entity ref fields
    if !T::entity_ref_fields().is_empty() {
        let entity_count = entities_query.iter().len();
        let mut entity_info_map = HashMap::with_capacity(entity_count);

        for (entity_uuid, managed) in entities_query.iter() {
            entity_info_map.insert(
                entity_uuid.get(),
                EntityRefDisplayInfo {
                    type_name: managed.type_name.clone(),
                    name: managed.name.clone(),
                },
            );
        }

        ui.ctx().data_mut(|data| {
            data.insert_temp(
                egui::Id::new("yoleck_entity_ref_display_info"),
                entity_info_map,
            );
        });
    }

    ui.group(|ui| {
        ui.label(egui::RichText::new(T::KEY).strong());
        ui.separator();
        T::auto_edit(&mut component, ui);
    });
}

#[cfg(not(feature = "vpeol"))]
pub fn auto_edit_system<T: YoleckComponent + YoleckAutoEdit>(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<&mut T>,
) {
    let Ok(mut component) = edit.single_mut() else {
        return;
    };

    ui.group(|ui| {
        ui.label(egui::RichText::new(T::KEY).strong());
        ui.separator();
        T::auto_edit(&mut component, ui);
    });
}

pub trait YoleckAutoEditExt {
    #[cfg(feature = "vpeol")]
    fn add_yoleck_auto_edit<
        T: Component<Mutability = Mutable>
            + YoleckComponent
            + YoleckAutoEdit
            + YoleckEntityRefAccessor,
    >(
        &mut self,
    );

    #[cfg(not(feature = "vpeol"))]
    fn add_yoleck_auto_edit<T: YoleckComponent + YoleckAutoEdit>(&mut self);
}

impl YoleckAutoEditExt for App {
    #[cfg(feature = "vpeol")]
    fn add_yoleck_auto_edit<
        T: Component<Mutability = Mutable>
            + YoleckComponent
            + YoleckAutoEdit
            + YoleckEntityRefAccessor,
    >(
        &mut self,
    ) {
        self.add_yoleck_edit_system(auto_edit_system::<T>);

        let construction_specs = self
            .world_mut()
            .get_resource::<crate::YoleckEntityConstructionSpecs>();

        if let Some(specs) = construction_specs {
            validate_entity_ref_requirements_for::<T>(specs);
        }
    }

    #[cfg(not(feature = "vpeol"))]
    fn add_yoleck_auto_edit<T: YoleckComponent + YoleckAutoEdit>(&mut self) {
        self.add_yoleck_edit_system(auto_edit_system::<T>);
    }
}
