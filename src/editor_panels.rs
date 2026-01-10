use bevy::prelude::*;

use crate::YoleckEditorSection;

/// Sections for the left panel of the Yoleck editor window.
///
/// Already contains sections by default, but can be used to customize the editor by adding more
/// sections. Each section is a function/closure that accepts a world and returns a closure that
/// accepts as world and a UI. The outer closure is responsible for prepareing a `SystemState` for
/// the inner closure to use.
///
/// ```no_run
/// # use bevy::prelude::*;
/// use bevy::ecs::system::SystemState;
/// # use bevy_yoleck::{YoleckEditorLeftPanelSections, egui};
/// # let mut app = App::new();
/// app.world_mut().resource_mut::<YoleckEditorLeftPanelSections>().0.push((|world: &mut World| {
///     let mut system_state = SystemState::<(
///         Res<Time>,
///     )>::new(world);
///     move |world: &mut World, ui: &mut egui::Ui| {
///         let (
///             time,
///         ) = system_state.get_mut(world);
///         ui.label(format!("Time since startup is {:?}", time.elapsed()));
///         Ok(())
///     }
/// }).into());
/// ```
#[derive(Resource)]
pub struct YoleckEditorLeftPanelSections(pub Vec<YoleckEditorSection>);

/// Sections for the right panel of the Yoleck editor window.
#[derive(Resource)]
pub struct YoleckEditorRightPanelSections(pub Vec<YoleckEditorSection>);

/// Sections for the top panel of the Yoleck editor window.
#[derive(Resource)]
pub struct YoleckEditorTopPanelSections(pub Vec<YoleckEditorSection>);

/// A tab in the bottom panel of the Yoleck editor window.
pub struct YoleckEditorBottomPanelTab {
    pub name: String,
    pub section: YoleckEditorSection,
}

impl YoleckEditorBottomPanelTab {
    pub fn new(name: impl Into<String>, section: YoleckEditorSection) -> Self {
        Self {
            name: name.into(),
            section,
        }
    }
}

/// Tabs for the bottom panel of the Yoleck editor window.
#[derive(Resource)]
pub struct YoleckEditorBottomPanelSections {
    pub tabs: Vec<YoleckEditorBottomPanelTab>,
    pub active_tab: usize,
}

impl Default for YoleckEditorRightPanelSections {
    fn default() -> Self {
        YoleckEditorRightPanelSections(vec![crate::editor::entity_editing_section.into()])
    }
}

impl Default for YoleckEditorTopPanelSections {
    fn default() -> Self {
        YoleckEditorTopPanelSections(vec![
            crate::level_files_manager::level_files_manager_top_section.into(),
            crate::level_files_manager::playtest_buttons_section.into(),
        ])
    }
}

impl Default for YoleckEditorBottomPanelSections {
    fn default() -> Self {
        YoleckEditorBottomPanelSections {
            tabs: vec![YoleckEditorBottomPanelTab::new(
                "Console",
                crate::console::console_panel_section.into(),
            )],
            active_tab: 0,
        }
    }
}

impl Default for YoleckEditorLeftPanelSections {
    fn default() -> Self {
        YoleckEditorLeftPanelSections(vec![
            crate::editor::new_entity_section.into(),
            crate::editor::entity_selection_section.into(),
        ])
    }
}
