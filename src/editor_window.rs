use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

use crate::util::EditSpecificResources;
use crate::YoleckEditorSections;

pub(crate) fn yoleck_editor_window(
    world: &mut World,
    mut egui_query: Local<Option<QueryState<&mut EguiContext>>>,
) {
    let egui_query = egui_query.get_or_insert_with(|| world.query());
    let mut borrowed_egui = if let Ok(mut egui_context) = egui_query.single_mut(world) {
        core::mem::take(egui_context.as_mut())
    } else {
        return;
    };
    egui::Window::new("Level Editor")
        .vscroll(true)
        .show(borrowed_egui.get_mut(), |ui| {
            world.resource_scope(
                |world, mut yoleck_editor_sections: Mut<YoleckEditorSections>| {
                    world.resource_scope(|world, mut edit_specific: Mut<EditSpecificResources>| {
                        edit_specific.inject_to_world(world);
                        for section in yoleck_editor_sections.0.iter_mut() {
                            section.0.invoke(world, ui);
                        }
                        edit_specific.take_from_world(world);
                    });
                },
            );
        });
    if let Ok(mut egui_context) = egui_query.single_mut(world) {
        *egui_context = borrowed_egui;
    }
}

#[allow(clippy::type_complexity)]
enum YoleckEditorSectionInner {
    Uninitialized(
        Box<
            dyn 'static
                + Send
                + Sync
                + FnOnce(
                    &mut World,
                )
                    -> Box<dyn 'static + Send + Sync + FnMut(&mut World, &mut egui::Ui)>,
        >,
    ),
    Middle,
    Initialized(Box<dyn 'static + Send + Sync + FnMut(&mut World, &mut egui::Ui)>),
}

impl YoleckEditorSectionInner {
    fn invoke(&mut self, world: &mut World, ui: &mut egui::Ui) {
        match self {
            Self::Uninitialized(_) => {
                if let Self::Uninitialized(system_constructor) =
                    core::mem::replace(self, Self::Middle)
                {
                    let mut system = system_constructor(world);
                    system(world, ui);
                    *self = Self::Initialized(system);
                } else {
                    panic!("It was just Uninitialized...");
                }
            }
            Self::Middle => panic!("Cannot start in the middle state when being invoked"),
            Self::Initialized(system) => {
                system(world, ui);
            }
        }
    }
}

/// A single section of the UI. See [`YoleckEditorSections`](crate::YoleckEditorSections).
pub struct YoleckEditorSection(YoleckEditorSectionInner);

impl<C, S> From<C> for YoleckEditorSection
where
    C: 'static + Send + Sync + FnOnce(&mut World) -> S,
    S: 'static + Send + Sync + FnMut(&mut World, &mut egui::Ui),
{
    fn from(system_constructor: C) -> Self {
        Self(YoleckEditorSectionInner::Uninitialized(Box::new(
            move |world| Box::new(system_constructor(world)),
        )))
    }
}
