use std::path::Path;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_mod_picking::PickingCameraBundle;
use bevy_transform_gizmo::GizmoPickSource;
use bevy_yoleck::tools_3d::transform_edit_adapter;
use bevy_yoleck::{
    YoleckEdit, YoleckEditorLevelsDirectoryPath, YoleckExtForApp, YoleckLoadingCommand,
    YoleckPluginForEditor, YoleckPluginForGame, YoleckPopulate, YoleckTypeHandlerFor,
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
                    asset_server.load(Path::new("levels3d").join(&level)),
                );
            },
        );
    } else {
        app.add_plugin(EguiPlugin);
        app.add_plugin(YoleckPluginForEditor);
        app.insert_resource(YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels3d"),
        ));
        app.add_plugin(bevy_yoleck::tools_3d::YoleckTools3dPlugin);
    }
    app.add_yoleck_handler({
        YoleckTypeHandlerFor::<ExampleBox>::new("ExampleBox")
            .populate_with(populate_box)
            .with(transform_edit_adapter(|example_box: &mut ExampleBox| {
                &mut example_box.position
            }))
            .edit_with(edit_box)
    });
    app.init_resource::<ExampleAssets>();
    app.add_startup_system(setup_camera);
    app.add_startup_system(setup_meshes);
    app.run();
}

fn setup_camera(mut commands: Commands) {
    let mut camera = PerspectiveCameraBundle::new_3d();
    camera.transform.translation.z = 100.0;
    commands
        .spawn_bundle(camera)
        .insert_bundle(PickingCameraBundle::default())
        .insert(GizmoPickSource::default());
}

#[derive(Default)]
struct ExampleAssets {
    cube_mesh: Handle<Mesh>,
}

fn setup_meshes(mut example_assets: ResMut<ExampleAssets>, mut mesh_assets: ResMut<Assets<Mesh>>) {
    example_assets.cube_mesh = mesh_assets.add(shape::Cube::new(10.0).into());
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct ExampleBox {
    #[serde(default)]
    position: Vec3,
}

fn populate_box(mut populate: YoleckPopulate<ExampleBox>, example_assets: Res<ExampleAssets>) {
    populate.populate(|_ctx, data, mut cmd| {
        cmd.insert_bundle(PbrBundle {
            mesh: example_assets.cube_mesh.clone(),
            // material: todo!(),
            transform: Transform::from_translation(data.position),
            // global_transform: todo!(),
            // visibility: todo!(),
            // computed_visibility: todo!(),
            ..Default::default()
        });
    });
}

fn edit_box(_edit: YoleckEdit<ExampleBox>) {}
