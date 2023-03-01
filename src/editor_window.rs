use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

use crate::YoleckEditorSections;

pub(crate) fn yoleck_editor_window(world: &mut World) {
    world.resource_scope(|world, mut egui_context: Mut<EguiContext>| {
        world.resource_scope(
            |world, mut yoleck_editor_sections: Mut<YoleckEditorSections>| {
                egui::Window::new("Level Editor").show(egui_context.ctx_mut(), |ui| {
                    for section in yoleck_editor_sections.0.iter_mut() {
                        section.0.invoke(world, ui);
                    }
                });
            },
        );
    });
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
