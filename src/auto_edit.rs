use bevy::prelude::*;
use bevy_egui::egui;

pub trait YoleckAutoEdit: Send + Sync + 'static {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui);
}

pub fn render_auto_edit_value<T: YoleckAutoEdit>(ui: &mut egui::Ui, value: &mut T) {
    T::auto_edit(value, ui);
}

impl YoleckAutoEdit for f32 {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(value).speed(0.1));
    }
}

impl YoleckAutoEdit for f64 {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(value).speed(0.1));
    }
}

impl YoleckAutoEdit for i32 {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(value).speed(1.0));
    }
}

impl YoleckAutoEdit for i64 {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(value).speed(1.0));
    }
}

impl YoleckAutoEdit for u32 {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(value).speed(1.0));
    }
}

impl YoleckAutoEdit for u64 {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(value).speed(1.0));
    }
}

impl YoleckAutoEdit for usize {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(value).speed(1.0));
    }
}

impl YoleckAutoEdit for isize {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(value).speed(1.0));
    }
}

impl YoleckAutoEdit for bool {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.checkbox(value, "");
    }
}

impl YoleckAutoEdit for String {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.text_edit_singleline(value);
    }
}

impl YoleckAutoEdit for Vec2 {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut value.x).prefix("x: ").speed(0.1));
            ui.add(egui::DragValue::new(&mut value.y).prefix("y: ").speed(0.1));
        });
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
}

impl YoleckAutoEdit for Quat {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        let (mut yaw, mut pitch, mut roll) = value.to_euler(EulerRot::YXZ);
        yaw = yaw.to_degrees();
        pitch = pitch.to_degrees();
        roll = roll.to_degrees();
        
        ui.horizontal(|ui| {
            let mut changed = false;
            changed |= ui.add(egui::DragValue::new(&mut yaw).prefix("yaw: ").speed(1.0).suffix("°")).changed();
            changed |= ui.add(egui::DragValue::new(&mut pitch).prefix("pitch: ").speed(1.0).suffix("°")).changed();
            changed |= ui.add(egui::DragValue::new(&mut roll).prefix("roll: ").speed(1.0).suffix("°")).changed();
            
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
}

impl YoleckAutoEdit for Color {
    fn auto_edit(value: &mut Self, ui: &mut egui::Ui) {
        let srgba = value.to_srgba();
        let mut color_arr = [srgba.red, srgba.green, srgba.blue, srgba.alpha];
        if ui.color_edit_button_rgba_unmultiplied(&mut color_arr).changed() {
            *value = Color::srgba(color_arr[0], color_arr[1], color_arr[2], color_arr[3]);
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
            if let Some(ref mut inner) = value {
                T::auto_edit(inner, ui);
            }
        });
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

pub trait YoleckAutoEditEnum: Sized + Clone + PartialEq + 'static {
    fn variants() -> &'static [Self];
    fn variant_name(&self) -> &'static str;
}

pub fn auto_edit_enum<T: YoleckAutoEditEnum>(value: &mut T, ui: &mut egui::Ui) {
    egui::ComboBox::from_id_salt(std::any::type_name::<T>())
        .selected_text(value.variant_name())
        .show_ui(ui, |ui| {
            for variant in T::variants() {
                ui.selectable_value(value, variant.clone(), variant.variant_name());
            }
        });
}

use crate::editing::{YoleckEdit, YoleckUi};
use crate::specs_registration::YoleckComponent;
use crate::YoleckExtForApp;

#[cfg(feature = "vpeol")]
use bevy::ecs::component::Mutable;
#[cfg(feature = "vpeol")]
use crate::entity_ref::{edit_entity_refs_system, YoleckEntityRefAccessor};

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
        T: Component<Mutability = Mutable> + YoleckComponent + YoleckAutoEdit + YoleckEntityRefAccessor,
    >(
        &mut self,
    );

    #[cfg(not(feature = "vpeol"))]
    fn add_yoleck_auto_edit<T: YoleckComponent + YoleckAutoEdit>(&mut self);
}

impl YoleckAutoEditExt for App {
    #[cfg(feature = "vpeol")]
    fn add_yoleck_auto_edit<
        T: Component<Mutability = Mutable> + YoleckComponent + YoleckAutoEdit + YoleckEntityRefAccessor,
    >(
        &mut self,
    ) {
        self.add_yoleck_edit_system(auto_edit_system::<T>);
        self.add_yoleck_edit_system(edit_entity_refs_system::<T>);
        
        let mut requirements = self.world_mut()
            .get_resource_or_insert_with(crate::entity_ref::YoleckEntityRefRequirements::default);
        
        let component_type = std::any::type_name::<T>();
        for (field_name, filter) in T::entity_ref_fields() {
            if let Some(required_entity_type) = filter {
                requirements.requirements.push((
                    component_type.to_string(),
                    field_name.to_string(),
                    required_entity_type.to_string(),
                ));
            }
        }
    }

    #[cfg(not(feature = "vpeol"))]
    fn add_yoleck_auto_edit<T: YoleckComponent + YoleckAutoEdit>(&mut self) {
        self.add_yoleck_edit_system(auto_edit_system::<T>);
    }
}

