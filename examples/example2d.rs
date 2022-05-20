use std::path::Path;

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_egui::{egui, EguiPlugin};

use bevy_yoleck::tools_2d::{handle_position_adjustable_z, handle_position_fixed_z};
use bevy_yoleck::{
    YoleckEdit, YoleckEditorState, YoleckExtForApp, YoleckLoadingCommand, YoleckPluginForEditor,
    YoleckPluginForGame, YoleckPopulate, YoleckTypeHandlerFor,
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
    app.add_yoleck_handler({
        YoleckTypeHandlerFor::<ExampleBox>::new("ExampleBox")
            .populate_with(populate_box)
            .with(handle_position_adjustable_z(|data: &mut ExampleBox| {
                &mut data.position
            }))
            .edit_with(edit_box)
    });
    app.add_yoleck_handler({
        YoleckTypeHandlerFor::<ExampleBox2>::new("ExampleBox2")
            .populate_with(populate_box2)
            .with(handle_position_fixed_z(
                |data: &mut ExampleBox2| &mut data.position,
                0.0,
            ))
    });
    app.add_startup_system(setup_camera);
    if true {
        app.add_system(move_the_boxes);
    } else {
        app.add_system_set(
            SystemSet::on_update(YoleckEditorState::GameActive).with_system(move_the_boxes),
        );
    }
    app.run();
}

fn setup_camera(mut commands: Commands) {
    let mut camera = OrthographicCameraBundle::new_2d();
    camera.transform.translation.z = 100.0;
    commands.spawn_bundle(camera);
}

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct ExampleBox {
    #[serde(default)]
    position: Vec3,
    #[serde(default)]
    color: Color,
}

fn populate_box(mut populate: YoleckPopulate<ExampleBox>) {
    populate.populate(|ctx, data, mut cmd| {
        cmd.insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: data.color,
                custom_size: Some(Vec2::new(20.0, 20.0)),
                anchor: Anchor::BottomLeft,
                ..Default::default()
            },
            ..Default::default()
        });
        if !ctx.is_in_editor() {
            cmd.insert(Velocity(Vec2::new(1.0, 0.0)));
        }
    });
}

fn edit_box(mut edit: YoleckEdit<ExampleBox>) {
    edit.edit(|_ctx, data, ui| {
        data.color = data.color.as_rgba();
        if let Color::Rgba {
            red,
            green,
            blue,
            alpha,
        } = &mut data.color
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
    });
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct ExampleBox2 {
    #[serde(default)]
    position: Vec2,
}

fn populate_box2(mut populate: YoleckPopulate<ExampleBox2>) {
    populate.populate(|_ctx, _data, mut cmd| {
        cmd.insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::GREEN,
                anchor: Anchor::TopRight,
                custom_size: Some(Vec2::new(30.0, 30.0)),
                ..Default::default()
            },
            ..Default::default()
        });
    });
}

fn move_the_boxes(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in query.iter_mut() {
        transform.translation += velocity.0.extend(0.0);
    }
}
