use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

pub struct YoleckPlugin;

impl Plugin for YoleckPlugin {
    fn build(&self, app: &mut App) {
        app.add_state(YoleckEditorState::EditorActive);
        app.add_system_set(
            SystemSet::on_update(YoleckEditorState::EditorActive).with_system(yoleck_editor),
        );
        app.insert_resource(YoleckState {
            entity_being_edited: None,
        });
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum YoleckEditorState {
    EditorActive,
}

pub trait YoleckSource: Send + Sync {
    fn populate(&self, cmd: &mut EntityCommands);
    fn edit(&mut self, ui: &mut egui::Ui);
}

#[derive(Component)]
pub struct YoleckManaged {
    pub name: String,
    pub source: Box<dyn YoleckSource>,
}

struct YoleckState {
    entity_being_edited: Option<Entity>,
}

fn yoleck_editor(
    mut egui_context: ResMut<EguiContext>,
    mut yoleck: ResMut<YoleckState>,
    mut yoleck_managed_query: Query<(Entity, &mut YoleckManaged)>,
    mut commands: Commands,
) {
    egui::Window::new("Level Editor").show(egui_context.ctx_mut(), |ui| {
        let yoleck = yoleck.as_mut();
        for (entity, yoleck_managed) in yoleck_managed_query.iter() {
            ui.selectable_value(
                &mut yoleck.entity_being_edited,
                Some(entity),
                format!("{} {:?}", yoleck_managed.name, entity),
            );
        }
        if let Some(entity) = yoleck.entity_being_edited {
            if let Ok((_, mut yoleck_managed)) = yoleck_managed_query.get_mut(entity) {
                ui.text_edit_singleline(&mut yoleck_managed.name);
                yoleck_managed.source.edit(ui);
                yoleck_managed.source.populate(&mut commands.entity(entity));
            }
        }
    });
}
