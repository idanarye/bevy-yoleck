use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy_egui::{egui, EguiPlugin};

use bevy_yoleck::{YoleckPlugin, YoleckRawEntry, YoleckSelectable, YoleckSource, YoleckState};
use serde::{Deserialize, Serialize};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugin(EguiPlugin);
    app.add_plugin(YoleckPlugin);
    app.add_startup_system(|mut yoleck: ResMut<YoleckState>| {
        yoleck.add_handler::<ExampleBox>("ExampleBox".to_owned());
        yoleck.add_handler::<ExampleBox2>("ExampleBox2".to_owned());
    });
    app.add_startup_system(setup_camera);
    app.add_startup_system(setup_entities); // TODO: replace with entity setup from data;
    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn setup_entities(mut commands: Commands) {
    let data = r#"[
        [{"type": "ExampleBox"}, {
            "position": [0.0, -50.0],
            "color":{"Rgba":{"alpha":1.0,"blue":0.0,"green":0.0,"red":1.0}}
        }],
        [{"type": "ExampleBox", "name": "box2"}, {
            "position": [0.0, 50.0],
            "color":{"Rgba":{"alpha":1.0,"blue":1.0,"green":0.0,"red":0.0}}
        }],
        [{"type": "ExampleBox2"}, {
            "position": [0.0, 0.0]
        }]
    ]"#;
    for entry in serde_json::from_str::<Vec<YoleckRawEntry>>(data).unwrap() {
        commands.spawn().insert(entry);
    }
}

#[derive(Serialize, Deserialize)]
struct ExampleBox {
    #[serde(default)]
    position: Vec2,
    #[serde(default)]
    color: Color,
}

impl YoleckSource for ExampleBox {
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
        cmd.insert(YoleckSelectable::rect(20.0, 20.0));
    }

    fn edit(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(&mut self.position.x).prefix("X:"));
        self.color = self.color.as_rgba();
        if let Color::Rgba {
            red,
            green,
            blue,
            alpha,
        } = &mut self.color
        {
            let color32: egui::Color32 =
                egui::Rgba::from_rgba_unmultiplied(*red, *green, *blue, *alpha).into();
            let mut rgba: egui::Rgba = color32.into();
            egui::widgets::color_picker::color_edit_button_rgba(
                ui,
                &mut rgba,
                egui::color_picker::Alpha::OnlyBlend,
            );
            *red = rgba.r();
            *green = rgba.g();
            *blue = rgba.b();
            *alpha = rgba.a();
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ExampleBox2 {
    #[serde(default)]
    position: Vec2,
}

impl YoleckSource for ExampleBox2 {
    fn populate(&self, cmd: &mut EntityCommands) {
        cmd.insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::GREEN,
                custom_size: Some(Vec2::new(30.0, 30.0)),
                ..Default::default()
            },
            transform: Transform::from_translation(self.position.extend(0.0)),
            ..Default::default()
        });
        cmd.insert(YoleckSelectable::rect(30.0, 30.0));
    }

    fn edit(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(&mut self.position.x).prefix("X:"));
        ui.add(egui::DragValue::new(&mut self.position.y).prefix("Y:"));
    }
}
