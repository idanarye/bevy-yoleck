use crate::bevy_egui::egui;
use crate::{
    YoleckDirective, YoleckEdit, YoleckEditorEvent, YoleckEditorState, YoleckPopulate,
    YoleckTypeHandlerFor,
};
use bevy::prelude::*;
use bevy_egui::EguiContext;
use bevy_mod_picking::{
    DefaultPickingPlugins, PickableBundle, PickingCameraBundle, PickingEvent, PickingPluginsState,
};
use bevy_transform_gizmo::{
    GizmoPickSource, GizmoTransformable, TransformGizmoEvent, TransformGizmoPlugin,
};
use smooth_bevy_cameras::controllers::orbit::OrbitCameraPlugin;
pub use smooth_bevy_cameras::controllers::orbit::{OrbitCameraBundle, OrbitCameraController};
use smooth_bevy_cameras::LookTransformPlugin;

pub struct YoleckTools3dPlugin;

impl Plugin for YoleckTools3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPickingPlugins);
        app.add_plugin(TransformGizmoPlugin::default());
        app.add_plugin(LookTransformPlugin);
        app.add_plugin(OrbitCameraPlugin::default());
        app.add_system_set({
            SystemSet::on_update(YoleckEditorState::EditorActive)
                .with_system(enable_disable)
                .with_system(process_picking_events)
                .with_system(process_events_from_yoleck)
                .with_system(process_gizmo_events)
        });
    }
}

fn enable_disable(
    mut prev: Local<Option<bool>>,
    yoleck_editor_state: Res<State<YoleckEditorState>>,
    mut egui_context: ResMut<EguiContext>,
    mut picking_plugins_state: ResMut<PickingPluginsState>,
    mut orbit_camera_controller_query: Query<&mut OrbitCameraController>,
) {
    let should_set_to = if matches!(
        *yoleck_editor_state.current(),
        YoleckEditorState::GameActive
    ) {
        false
    } else {
        !egui_context.ctx_mut().is_pointer_over_area()
    };
    if *prev == Some(should_set_to) {
        return;
    }
    *prev = Some(should_set_to);
    picking_plugins_state.enable_picking = should_set_to;
    picking_plugins_state.enable_highlighting = should_set_to;
    picking_plugins_state.enable_interacting = should_set_to;
    for mut orbit_camera_controller in orbit_camera_controller_query.iter_mut() {
        orbit_camera_controller.enabled = should_set_to;
    }
}

fn process_picking_events(
    mut picking_reader: EventReader<PickingEvent>,
    mut directives_writer: EventWriter<YoleckDirective>,
) {
    let mut select = None;
    let mut deselect = false;
    for event in picking_reader.iter() {
        let event = if let PickingEvent::Selection(event) = event {
            event
        } else {
            continue;
        };
        match event {
            bevy_mod_picking::SelectionEvent::JustSelected(entity) => {
                select = Some(entity);
            }
            bevy_mod_picking::SelectionEvent::JustDeselected(_) => {
                deselect = true;
            }
        }
    }
    if let Some(entity) = select {
        directives_writer.send(YoleckDirective::set_selected(Some(*entity)));
    } else if deselect {
        // only if nothing was selected this frame
        directives_writer.send(YoleckDirective::set_selected(None));
    }
}

fn process_events_from_yoleck(
    mut yoleck_reader: EventReader<YoleckEditorEvent>,
    mut selection_query: Query<(Entity, &mut bevy_mod_picking::Selection)>,
) {
    for event in yoleck_reader.iter() {
        match event {
            YoleckEditorEvent::EntitySelected(selected_entity) => {
                for (entity, mut selection) in selection_query.iter_mut() {
                    selection.set_selected(entity == *selected_entity);
                }
            }
            YoleckEditorEvent::EntityDeselected(deselected_entity) => {
                if let Ok((_, mut selection)) = selection_query.get_mut(*deselected_entity) {
                    selection.set_selected(false);
                }
            }
            YoleckEditorEvent::EditedEntityPopulated(repopulated_entity) => {
                if let Ok((_, mut selection)) = selection_query.get_mut(*repopulated_entity) {
                    selection.set_selected(true);
                }
            }
        }
    }
}

fn process_gizmo_events(
    mut gizmo_reader: EventReader<TransformGizmoEvent>,
    mut directives_writer: EventWriter<YoleckDirective>,
    selection_query: Query<(Entity, &bevy_mod_picking::Selection)>,
    global_transform_query: Query<&GlobalTransform>,
) {
    let selected_entity = || {
        selection_query.iter().find_map(|(entity, selection)| {
            if selection.selected() {
                Some(entity)
            } else {
                None
            }
        })
    };
    for event in gizmo_reader.iter() {
        match event.interaction {
            bevy_transform_gizmo::TransformGizmoInteraction::TranslateAxis { .. } => {
                if let Some(entity) = selected_entity() {
                    directives_writer.send(YoleckDirective::pass_to_entity(
                        entity,
                        event.to.translation,
                    ));
                }
            }
            bevy_transform_gizmo::TransformGizmoInteraction::TranslateOrigin => {}
            bevy_transform_gizmo::TransformGizmoInteraction::RotateAxis { .. } => {
                if let Some(entity) = selected_entity() {
                    // For some reason event.to.rotation doesn't work, so we need to grab it from
                    // the component...
                    if let Ok(global_transform) = global_transform_query.get(entity) {
                        directives_writer.send(YoleckDirective::pass_to_entity(
                            entity,
                            global_transform.rotation,
                        ));
                    }
                }
            }
            bevy_transform_gizmo::TransformGizmoInteraction::ScaleAxis { .. } => {}
        }
    }
}

#[derive(Bundle)]
pub struct Tools3DCameraBundle {
    #[bundle]
    pub orbit_camera_bundle: OrbitCameraBundle,
    #[bundle]
    pub picking_camera_bundle: PickingCameraBundle,
    pub gizmo_pick_source: GizmoPickSource,
}

impl Tools3DCameraBundle {
    pub fn new(orbit_camera_bundle: OrbitCameraBundle) -> Self {
        Self {
            orbit_camera_bundle,
            picking_camera_bundle: Default::default(),
            gizmo_pick_source: Default::default(),
        }
    }
}

pub struct Transform3dProjection<'a> {
    pub translation: &'a mut Vec3,
    pub rotation: Option<&'a mut Quat>,
}

pub fn transform_edit_adapter<T: 'static>(
    projection: impl 'static + Clone + Send + Sync + for<'a> Fn(&'a mut T) -> Transform3dProjection<'a>,
) -> impl FnOnce(YoleckTypeHandlerFor<T>) -> YoleckTypeHandlerFor<T> {
    move |handler| {
        handler
            .populate_with(move |mut populate: YoleckPopulate<T>| {
                populate.populate(|_ctx, _data, mut cmd| {
                    cmd.insert_bundle(PickableBundle::default());
                    cmd.insert(GizmoTransformable);
                });
            })
            .edit_with(move |mut edit: YoleckEdit<T>| {
                edit.edit(|ctx, data, ui| {
                    let Transform3dProjection {
                        translation,
                        rotation,
                    } = projection(data);
                    if let Some(pos) = ctx.get_passed_data::<Vec3>() {
                        *translation = *pos;
                    }
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut translation.x).prefix("X:"));
                        ui.add(egui::DragValue::new(&mut translation.y).prefix("Y:"));
                        ui.add(egui::DragValue::new(&mut translation.z).prefix("Z:"));
                    });
                    if let Some(rotation) = rotation {
                        if let Some(rot) = ctx.get_passed_data::<Quat>() {
                            *rotation = *rot;
                        }
                    }
                });
            })
    }
}
