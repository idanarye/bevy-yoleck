use std::path::Path;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_egui::{egui, EguiPlugin};

use bevy_yoleck::{
    YoleckEditContext, YoleckEditorState, YoleckLoadingCommand, YoleckPluginForEditor,
    YoleckPluginForGame, YoleckPopulateContext, YoleckSource, YoleckTypeHandlers,
};
use serde::{Deserialize, Serialize};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    let level = std::env::args().nth(1);
    if let Some(level) = level {
        app.add_plugin(YoleckPluginForGame);
        app.add_startup_system(
            move |asset_server: Res<AssetServer>,
                  mut yoleck_loading_command: ResMut<YoleckLoadingCommand>| {
                *yoleck_loading_command = YoleckLoadingCommand::FromAsset(
                    asset_server.load(Path::new("levels").join(&level)),
                );
            },
        );
    } else {
        app.add_plugin(EguiPlugin);
        app.add_plugin(YoleckPluginForEditor);
        app.add_plugin(bevy_yoleck::tools_2d::YoleckTools2dPlugin);
    }
    app.insert_resource(YoleckTypeHandlers::new([
        ExampleBox::handler(),
        ExampleBox2::handler(),
    ]));
    app.add_startup_system(setup_camera);
    app.add_system_set(
        SystemSet::on_update(YoleckEditorState::GameActive).with_system(move_the_boxes),
    );
    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Serialize, Deserialize)]
struct ExampleBox {
    #[serde(default)]
    position: Vec2,
    #[serde(default)]
    color: Color,
}

impl YoleckSource for ExampleBox {
    const NAME: &'static str = "ExampleBox";

    fn populate(&self, _ctx: &YoleckPopulateContext, cmd: &mut EntityCommands) {
        cmd.insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: self.color,
                custom_size: Some(Vec2::new(20.0, 20.0)),
                anchor: Anchor::BottomLeft,
                ..Default::default()
            },
            transform: Transform::from_translation(self.position.extend(0.0)),
            ..Default::default()
        })
        .insert(Velocity(Vec2::new(1.0, 0.0)));
    }

    fn edit(&mut self, ctx: &YoleckEditContext, ui: &mut egui::Ui) {
        if let Some(pos) = ctx.get_passed_data::<Vec2>() {
            *self.position = **pos;
        }
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
    const NAME: &'static str = "ExampleBox2";

    fn populate(&self, _ctx: &YoleckPopulateContext, cmd: &mut EntityCommands) {
        cmd.insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::GREEN,
                anchor: Anchor::TopRight,
                custom_size: Some(Vec2::new(30.0, 30.0)),
                ..Default::default()
            },
            transform: Transform::from_translation(self.position.extend(0.0)),
            ..Default::default()
        });
    }

    fn edit(&mut self, ctx: &YoleckEditContext, ui: &mut egui::Ui) {
        if let Some(pos) = ctx.get_passed_data::<Vec2>() {
            *self.position = **pos;
        }
        ui.add(egui::DragValue::new(&mut self.position.x).prefix("X:"));
        ui.add(egui::DragValue::new(&mut self.position.y).prefix("Y:"));
    }
}

fn move_the_boxes(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in query.iter_mut() {
        transform.translation += velocity.0.extend(0.0);
    }
}
