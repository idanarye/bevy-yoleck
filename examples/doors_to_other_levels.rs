use std::path::Path;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_yoleck::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    app.add_plugins(EguiPlugin);
    app.add_plugins(YoleckPluginForEditor);

    app.insert_resource(bevy_yoleck::YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels_doors"),
    ));

    app.run();
}
