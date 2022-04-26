use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy_egui::{egui, EguiPlugin};

use bevy_yoleck::{YoleckPlugin, YoleckRaw, YoleckSource, YoleckState};
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
        ["ExampleBox", {
            "position": [0.0, -50.0],
            "color":{"Rgba":{"alpha":1.0,"blue":0.0,"green":0.0,"red":1.0}}
        }],
        ["ExampleBox", {
            "position": [0.0, 50.0],
            "color":{"Rgba":{"alpha":1.0,"blue":1.0,"green":0.0,"red":0.0}}
        }],
        ["ExampleBox2", {
            "position": [0.0, 0.0]
        }]
    ]"#;
    for (type_name, data) in serde_json::from_str::<Vec<(String, serde_json::Value)>>(data).unwrap()
    {
        commands.spawn().insert(YoleckRaw { type_name, data });
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

#[derive(Serialize, Deserialize)]
struct ExampleBox2 {
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
    }

    fn edit(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Slider::new(&mut self.position.x, -100.0..=100.0).text("X Position"));
        ui.add(egui::Slider::new(&mut self.position.y, -100.0..=100.0).text("Y Position"));
    }
}
