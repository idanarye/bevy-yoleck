use bevy::ecs::system::SystemId;
use bevy::prelude::*;
use bevy_egui::egui;
use std::ops::{Deref, DerefMut};

use crate::util::EditSpecificResources;

/// An handle for the egui UI frame used in panel sections definitions
#[derive(Resource)]
pub struct YoleckPanelUi(pub egui::Ui);

impl Deref for YoleckPanelUi {
    type Target = egui::Ui;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for YoleckPanelUi {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub(crate) trait EditorPanel: Resource + Sized {
    fn iter_sections(&self) -> impl Iterator<Item = SystemId<(), Result>>;
    fn wrapper(
        &mut self,
        ctx: &mut egui::Context,
        add_content: impl FnOnce(&mut Self, &mut egui::Ui),
    ) -> egui::Response;

    fn show_panel(world: &mut World, ctx: &mut egui::Context) -> egui::Response {
        world.resource_scope(|world, mut this: Mut<Self>| {
            this.wrapper(ctx, |this, ui| {
                let frame = egui::Frame::new();
                let mut prepared = frame.begin(ui);
                let content_ui = std::mem::replace(
                    &mut prepared.content_ui,
                    ui.new_child(egui::UiBuilder {
                        max_rect: Some(ui.max_rect()),
                        layout: Some(*ui.layout()), // Is this necessary?
                        ..Default::default()
                    }),
                );
                world.insert_resource(YoleckPanelUi(content_ui));

                world.resource_scope(|world, mut edit_specific: Mut<EditSpecificResources>| {
                    edit_specific.inject_to_world(world);
                    for section in this.iter_sections() {
                        world.run_system(section).unwrap().unwrap();
                    }
                    edit_specific.take_from_world(world);
                });

                let YoleckPanelUi(content_ui) = world.remove_resource().expect(
                    "The YoleckPanelUi resource was put in the world by this very function",
                );
                prepared.content_ui = content_ui;
                prepared.end(ui);
            })
        })
    }
}

/// Sections for the left panel of the Yoleck editor window.
///
/// Already contains sections by default, but can be used to customize the editor by adding more
/// sections. Each section is a Bevy system in the form of a [`SystemId`] with no input and a Bevy
/// [`Result<()>`] for an output. These can be obtained by registering a system function on the
/// Bevy app using [`register_system`](App::register_system).
///
/// The section system can draw on the panel using [`YoleckPanelUi`], accessible as a [`ResMut`].
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::{YoleckEditorLeftPanelSections, egui, YoleckPanelUi};
/// # let mut app = App::new();
/// let time_since_startup_section = app.register_system(|mut ui: ResMut<YoleckPanelUi>, time: Res<Time>| {
///     ui.label(format!("Time since startup is {:?}", time.elapsed()));
///     Ok(())
/// });
/// app.world_mut().resource_mut::<YoleckEditorLeftPanelSections>().0.push(time_since_startup_section);
/// ```
#[derive(Resource)]
pub struct YoleckEditorLeftPanelSections(pub Vec<SystemId<(), Result>>);

impl FromWorld for YoleckEditorLeftPanelSections {
    fn from_world(world: &mut World) -> Self {
        Self(vec![
            world.register_system(crate::editor::new_entity_section),
            world.register_system(crate::editor::entity_selection_section),
        ])
    }
}

impl EditorPanel for YoleckEditorLeftPanelSections {
    fn iter_sections(&self) -> impl Iterator<Item = SystemId<(), Result>> {
        self.0.iter().copied()
    }

    fn wrapper(
        &mut self,
        ctx: &mut egui::Context,
        add_content: impl FnOnce(&mut Self, &mut egui::Ui),
    ) -> egui::Response {
        egui::SidePanel::left("yoleck_left_panel")
            .resizable(true)
            .default_width(300.0)
            .max_width(ctx.content_rect().width() / 4.0)
            .show(ctx, |ui| {
                ui.heading("Level Hierarchy");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    add_content(self, ui);
                });
            })
            .response
    }
}

/// Sections for the right panel of the Yoleck editor window. Works the same as
/// [`YoleckEditorLeftPanelSections`].
#[derive(Resource)]
pub struct YoleckEditorRightPanelSections(pub Vec<SystemId<(), Result>>);

impl FromWorld for YoleckEditorRightPanelSections {
    fn from_world(world: &mut World) -> Self {
        Self(vec![
            world.register_system(crate::editor::entity_editing_section),
        ])
    }
}

impl EditorPanel for YoleckEditorRightPanelSections {
    fn iter_sections(&self) -> impl Iterator<Item = SystemId<(), Result>> {
        self.0.iter().copied()
    }

    fn wrapper(
        &mut self,
        ctx: &mut egui::Context,
        add_content: impl FnOnce(&mut Self, &mut egui::Ui),
    ) -> egui::Response {
        egui::SidePanel::right("yoleck_right_panel")
            .resizable(true)
            .default_width(300.0)
            .max_width(ctx.content_rect().width() / 4.0)
            .show(ctx, |ui| {
                ui.heading("Properties");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    add_content(self, ui);
                });
            })
            .response
    }
}

/// Sections for the top panel of the Yoleck editor window. Works the same as
/// [`YoleckEditorLeftPanelSections`].
#[derive(Resource)]
pub struct YoleckEditorTopPanelSections(pub Vec<SystemId<(), Result>>);

impl FromWorld for YoleckEditorTopPanelSections {
    fn from_world(world: &mut World) -> Self {
        Self(vec![
            world.register_system(crate::level_files_manager::level_files_manager_top_section),
            world.register_system(crate::level_files_manager::playtest_buttons_section),
        ])
    }
}

impl EditorPanel for YoleckEditorTopPanelSections {
    fn iter_sections(&self) -> impl Iterator<Item = SystemId<(), Result>> {
        self.0.iter().copied()
    }

    fn wrapper(
        &mut self,
        ctx: &mut egui::Context,
        add_content: impl FnOnce(&mut Self, &mut egui::Ui),
    ) -> egui::Response {
        egui::TopBottomPanel::top("yoleck_top_panel")
            .resizable(false)
            .show(ctx, |ui| {
                let inner_margin = 3.;

                ui.add_space(inner_margin);
                ui.horizontal(|ui| {
                    ui.add_space(inner_margin);
                    ui.label("Yoleck Editor");
                    ui.separator();
                    add_content(self, ui);
                    ui.add_space(inner_margin);
                });
                ui.add_space(inner_margin);
            })
            .response
    }
}

/// A tab in the bottom panel of the Yoleck editor window.
///
/// The [`sections`](Self::sections) parameter is a list of [`SystemId`] obtained similarly to the
/// ones in [`YoleckEditorLeftPanelSections`].
pub struct YoleckEditorBottomPanelTab {
    pub name: String,
    pub sections: Vec<SystemId<(), Result>>,
}

/// Tabs for the bottom panel of the Yoleck editor window.
///
/// Works similar to [`YoleckEditorLeftPanelSections`], except instead of a single list of systems
/// they reside within [`tabs`](Self::tabs).
#[derive(Resource)]
pub struct YoleckEditorBottomPanelSections {
    pub tabs: Vec<YoleckEditorBottomPanelTab>,
    active_tab: usize,
}

impl FromWorld for YoleckEditorBottomPanelSections {
    fn from_world(world: &mut World) -> Self {
        Self {
            tabs: vec![YoleckEditorBottomPanelTab {
                name: "Console".to_owned(),
                sections: vec![world.register_system(crate::console::console_panel_section)],
            }],
            active_tab: 0,
        }
    }
}

impl EditorPanel for YoleckEditorBottomPanelSections {
    fn iter_sections(&self) -> impl Iterator<Item = SystemId<(), Result>> {
        self.tabs
            .get(self.active_tab)
            .into_iter()
            .flat_map(|tab| tab.sections.iter().copied())
    }

    fn wrapper(
        &mut self,
        ctx: &mut egui::Context,
        add_content: impl FnOnce(&mut Self, &mut egui::Ui),
    ) -> egui::Response {
        egui::TopBottomPanel::bottom("yoleck_bottom_panel")
            .resizable(true)
            .default_height(200.0)
            .max_height(ctx.content_rect().height() / 4.0)
            .show(ctx, |ui| {
                let inner_margin = 3.;
                ui.add_space(inner_margin);

                let mut new_active_tab = self.active_tab;
                ui.horizontal(|ui| {
                    for (i, tab) in self.tabs.iter().enumerate() {
                        if ui
                            .selectable_label(new_active_tab == i, &tab.name)
                            .clicked()
                        {
                            new_active_tab = i;
                        }
                    }
                });
                self.active_tab = new_active_tab;

                ui.separator();

                add_content(self, ui);
            })
            .response
    }
}
