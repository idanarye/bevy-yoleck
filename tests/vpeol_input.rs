#![cfg(feature = "vpeol")]

use bevy::{
    camera::RenderTarget,
    prelude::*,
    state::app::StatesPlugin,
    window::{PrimaryWindow, WindowRef},
};
use bevy_yoleck::{
    YoleckDirective, YoleckEditMarker, YoleckEditorViewportRect,
    bevy_egui::{EguiContext, EguiUserTextures, PrimaryEguiContext, egui},
    prelude::YoleckPluginForEditor,
    vpeol::{VpeolBasePlugin, VpeolCameraState, VpeolDragPlane},
};

#[test]
fn clicking_outside_the_editor_viewport_does_not_change_selection() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin))
        .add_plugins(YoleckPluginForEditor)
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<EguiUserTextures>()
        .insert_resource(VpeolDragPlane::XY)
        .insert_resource(YoleckEditorViewportRect {
            rect: Some(egui::Rect::from_min_max(
                egui::pos2(300.0, 40.0),
                egui::pos2(1000.0, 700.0),
            )),
        })
        .add_plugins(VpeolBasePlugin);

    let mut window = Window::default();
    window.set_cursor_position(Some(Vec2::new(1100.0, 300.0)));
    app.world_mut().spawn((window, PrimaryWindow));
    app.world_mut()
        .spawn((EguiContext::default(), PrimaryEguiContext));
    app.world_mut().spawn((
        RenderTarget::Window(WindowRef::Primary),
        VpeolCameraState {
            cursor_ray: Some(Ray3d::new(Vec3::ZERO, Dir3::Z)),
            ..default()
        },
    ));
    app.world_mut().spawn(YoleckEditMarker);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);

    app.update();

    assert!(
        app.world()
            .resource::<Messages<YoleckDirective>>()
            .is_empty()
    );
}
