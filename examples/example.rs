use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_egui::{egui, EguiContext, EguiPlugin};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum AppMode {
    Editor,
}

fn main() {
    let mut app = App::new();
    app.add_state(AppMode::Editor);
    app.add_plugins(DefaultPlugins);
    app.add_plugin(EguiPlugin);
    app.add_startup_system(setup_camera);
    app.add_startup_system(setup_entities); // TODO: replace with entity setup from data;
    app.add_system_set(SystemSet::on_update(AppMode::Editor).with_system(yoleck_editor));
    app.insert_resource(YoleckState {
        entity_being_edited: None,
        entities: Default::default(),
    });
    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn setup_entities(mut yoleck: ResMut<YoleckState>, mut commands: Commands) {
    for example_box in [
        ExampleBox {
            position: Vec2::new(0.0, -50.0),
            color: Color::RED,
        },
        ExampleBox {
            position: Vec2::new(0.0, 50.0),
            color: Color::BLUE,
        },
    ] {
        let mut cmd = commands.spawn();
        cmd.insert(YoleckOwned);
        example_box.populate(&mut cmd);
        yoleck.entities.insert(cmd.id(), example_box);
    }
}

#[derive(Component)]
pub struct YoleckOwned;

struct YoleckState {
    entity_being_edited: Option<Entity>,
    entities: HashMap<Entity, ExampleBox>,
}

fn yoleck_editor(
    mut egui_context: ResMut<EguiContext>,
    mut yoleck: ResMut<YoleckState>,
    mut commands: Commands,
) {
    egui::Window::new("Level Editor").show(egui_context.ctx_mut(), |ui| {
        let yoleck = yoleck.as_mut();
        for entity in yoleck.entities.keys() {
            ui.selectable_value(
                &mut yoleck.entity_being_edited,
                Some(*entity),
                format!("{:?}", entity),
            );
        }
        if let Some(entity) = yoleck.entity_being_edited {
            if let Some(data) = yoleck.entities.get_mut(&entity) {
                data.edit(ui);
                data.populate(&mut commands.entity(entity));
            }
        }
    });
}

struct ExampleBox {
    position: Vec2,
    color: Color,
}

impl ExampleBox {
    fn populate(&self, cmd: &mut EntityCommands) {
        cmd.insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: self.color,
                custom_size: Some(Vec2::new(20.0, 20.0)),
                ..Default::default()
            },
            transform: Transform::from_translation(self.position.extend(0.0)),
            ..Default::default()
        });
    }

    fn edit(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Slider::new(&mut self.position.x, -100.0..=100.0).text("X Position"));
        self.color = self.color.as_rgba();
        if let Color::Rgba {
            red,
            green,
            blue,
            alpha,
        } = &mut self.color
        {
            let mut color32: egui::Color32 =
                egui::Rgba::from_rgba_unmultiplied(*red, *green, *blue, *alpha).into();
            egui::widgets::color_picker::color_picker_color32(
                ui,
                &mut color32,
                egui::color_picker::Alpha::OnlyBlend,
            );
            let rgba: egui::Rgba = color32.into();
            *red = rgba.r();
            *green = rgba.g();
            *blue = rgba.b();
            *alpha = rgba.a();
        }
    }
}
