//! # Viewport Editing Overlay for 3D games.
//!
//! Use this module to implement simple 3D editing for 3D games.
//!
//! To use add the egui and Yoleck plugins to the Bevy app, as well as the plugin of this module:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoleck::bevy_egui::EguiPlugin;
//! # use bevy_yoleck::YoleckPluginForEditor;
//! # use bevy_yoleck::vpeol_3d::YoleckVpeol3dPlugin;
//! # let mut app = App::new();
//! app.add_plugin(EguiPlugin);
//! app.add_plugin(YoleckPluginForEditor);
//! app.add_plugin(YoleckVpeol3dPlugin);
//! ```
//!
//! [`YoleckVpeol3dPlugin`] adds the orbit camera plugin from
//! [`bevy-orbit-controls`](https://github.com/iMplode-nZ/bevy-orbit-controls), but camera still
//! needs to be confiugred for it.
//!
//! Entity selection by clicking on it is supported by
//! [`bevy_mod_picking`](https://github.com/aevyrie/bevy_mod_picking). It's plugin is added by
//! [`YoleckVpeol3dPlugin`], but the camera and entities still need to be configured.
//!
//! [`YoleckVpeol3dCameraBundle`] can be used to configure the camera for both plugins.
//!
//! To use `vpeol_3d` for an entity type, it's edit system can get `Vec3` (and possibly `Quat`)
//! from the context with [`get_passed_data`](crate::YoleckEditContext::get_passed_data). It also
//! needs to add `PickableBundle` and
//! [`bevy_transform_gizmo`](https://github.com/ForesightMiningSoftwareCorporation/bevy_transform_gizmo)'s
//! `GizmoTransformable` (both are conveniently re-exported from this module):
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoleck::{YoleckTypeHandler, YoleckExtForApp, YoleckEdit, YoleckPopulate};
//! # use bevy_yoleck::vpeol_3d::{PickableBundle, GizmoTransformable};
//! # use serde::{Deserialize, Serialize};
//! # #[derive(Clone, PartialEq, Serialize, Deserialize)]
//! # struct Example {
//! #     position: Vec3,
//! #     rotation: Quat,
//! # }
//! # let mut app = App::new();
//! app.add_yoleck_handler({
//!     YoleckTypeHandler::<Example>::new("Example")
//!         .edit_with(edit_example)
//!         .populate_with(populate_example)
//! });
//!
//! fn edit_example(mut edit: YoleckEdit<Example>) {
//!     edit.edit(|ctx, data, _ui| {
//!         if let Some(pos) = ctx.get_passed_data::<Vec3>() {
//!             data.position = *pos;
//!         }
//!         if let Some(rot) = ctx.get_passed_data::<Quat>() {
//!             data.rotation = *rot;
//!         }
//!     });
//! }
//!
//! fn populate_example(mut populate: YoleckPopulate<Example>) {
//!     populate.populate(|_ctx, data, mut cmd| {
//!         cmd.insert_bundle(PbrBundle {
//!             transform: Transform::from_translation(data.position),
//!             // Actual PBR components
//!             ..Default::default()
//!         });
//!         cmd.insert_bundle(PickableBundle::default());
//!         cmd.insert(GizmoTransformable);
//!     });
//! }
//! ```
//!
//! 3D entities are often hierarchical (e.g. when created by `spawn_scene`), so `PickableBundle`
//! needs to be added to the relevant children - together with [`YoleckRouteClickTo`]. Since these
//! entities are usually added later, [`YoleckWillContainClickableChildren`] can be used:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoleck::{YoleckTypeHandler, YoleckExtForApp, YoleckEdit, YoleckPopulate};
//! # use bevy_yoleck::vpeol_3d::{PickableBundle, GizmoTransformable, YoleckWillContainClickableChildren};
//! # use serde::{Deserialize, Serialize};
//! # #[derive(Clone, PartialEq, Serialize, Deserialize)]
//! # struct Example {
//! #     position: Vec3,
//! #     rotation: Quat,
//! # }
//!
//! fn populate_example(mut populate: YoleckPopulate<Example>, asset_server: Res<AssetServer>) {
//!     populate.populate(|_ctx, data, mut cmd| {
//!         cmd.insert_bundle(TransformBundle::from_transform(Transform::from_translation(data.position)));
//!         cmd.with_children(|commands| {
//!             commands.spawn_scene(asset_server.load("models/my-model.glb#Scene0"));
//!         });
//!         cmd.insert(YoleckWillContainClickableChildren);
//!         cmd.insert(GizmoTransformable); // added on the parent entity, not the children
//!     });
//! }
//! ```
//!
//! Alternatively, use [`yoleck_vpeol_transform_edit_adapter`]. It'll apply the transform and add
//! [`PickableBundle`] and [`GizmoTransformable`], but not [`YoleckWillContainClickableChildren`] -
//! that one still needs to be added separately.

use crate::bevy_egui::egui;
pub use crate::vpeol::YoleckWillContainClickableChildren;
use crate::vpeol::{handle_clickable_children_system, YoleckRouteClickTo};
use crate::{
    YoleckDirective, YoleckEdit, YoleckEditorEvent, YoleckEditorState, YoleckPopulate,
    YoleckTypeHandler,
};
use bevy::prelude::*;
use bevy_egui::EguiContext;
/// Reexported from [`bevy_mod_picking`](https://github.com/aevyrie/bevy_mod_picking).
pub use bevy_mod_picking::PickableBundle;
use bevy_mod_picking::{
    DefaultPickingPlugins, PickingCameraBundle, PickingEvent, PickingPluginsState,
};
/// Reexported from [`bevy_transform_gizmo`](https://github.com/ForesightMiningSoftwareCorporation/bevy_transform_gizmo).
pub use bevy_transform_gizmo::GizmoTransformable;
use bevy_transform_gizmo::{GizmoPickSource, TransformGizmoEvent, TransformGizmoPlugin};
use smooth_bevy_cameras::controllers::orbit::OrbitCameraPlugin;
/// Reexported from [`bevy-orbit-controls`](https://github.com/iMplode-nZ/bevy-orbit-controls).
pub use smooth_bevy_cameras::controllers::orbit::{OrbitCameraBundle, OrbitCameraController};
use smooth_bevy_cameras::LookTransformPlugin;

/// Add the systems required for 2D editing.
///
/// * Camera control using [`bevy-orbit-controls`](https://github.com/iMplode-nZ/bevy-orbit-controls)
///   * Needs to be installed in the camera entity separately.
/// * Entity selection using [`bevy_mod_picking`](https://github.com/aevyrie/bevy_mod_picking)
///   * Needs to be installed in the camera entity separately.
///   * Needs to be installed in the pickable entities separately.
/// * Entity dragging and rotating using
///   [`bevy_transform_gizmo`](https://github.com/ForesightMiningSoftwareCorporation/bevy_transform_gizmo).
///   * Needs to be installed in the pickable entities separately.
/// * Connecting nested entities.
pub struct YoleckVpeol3dPlugin;

impl Plugin for YoleckVpeol3dPlugin {
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
                .with_system(handle_clickable_children_system::<With<Handle<Mesh>>, PickableBundle>)
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
    mut selection_query: Query<&mut bevy_mod_picking::Selection>,
    root_resolver: Query<&YoleckRouteClickTo>,
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
        let entity = if let Ok(YoleckRouteClickTo(root_entity)) = root_resolver.get(*entity) {
            if let Ok(mut selection) = selection_query.get_mut(*entity) {
                selection.set_selected(false);
            }
            if let Ok(mut selection) = selection_query.get_mut(*root_entity) {
                selection.set_selected(true);
            }
            root_entity
        } else {
            entity
        };
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

/// Helper for installing
/// [`bevy-orbit-controls`](https://github.com/iMplode-nZ/bevy-orbit-controls) and
/// [`bevy_mod_picking`](https://github.com/aevyrie/bevy_mod_picking) in a camera entity.
#[derive(Bundle)]
pub struct YoleckVpeol3dCameraBundle {
    #[bundle]
    pub orbit_camera_bundle: OrbitCameraBundle,
    #[bundle]
    pub picking_camera_bundle: PickingCameraBundle,
    pub gizmo_pick_source: GizmoPickSource,
}

impl YoleckVpeol3dCameraBundle {
    pub fn new(orbit_camera_bundle: OrbitCameraBundle) -> Self {
        Self {
            orbit_camera_bundle,
            picking_camera_bundle: Default::default(),
            gizmo_pick_source: Default::default(),
        }
    }
}

/// See [`yoleck_vpeol_transform_edit_adapter`].
pub struct YoleckVpeolTransform3dProjection<'a> {
    pub translation: &'a mut Vec3,
    pub rotation: Option<&'a mut Quat>,
}

/// Implement parts of the 3D editing for the entity:
///
/// * Add [`PickableBundle`] and [`GizmoTransformable`] to install entity selection and transform gizmo.
/// * Edit a `Vec3` position field of an entity with the gizmo.
/// * Edit a `Quat` rotation field of an entity with the gizmo.
///
/// Note that this does not populate the `Transform` component - this needs be done with a manually
/// written populate system. Also, if the children of the entity entity are the ones using for
/// picking [`YoleckWillContainClickableChildren`] needs to be added to it - this adapter will not
/// add it.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::{YoleckTypeHandler, YoleckExtForApp, YoleckPopulate};
/// # use bevy_yoleck::vpeol_3d::{yoleck_vpeol_transform_edit_adapter, YoleckVpeolTransform3dProjection};
/// # use serde::{Deserialize, Serialize};
/// # #[derive(Clone, PartialEq, Serialize, Deserialize)]
/// # struct Example {
/// #     position: Vec3,
/// #     rotation: Quat,
/// # }
/// # let mut app = App::new();
/// app.add_yoleck_handler({
///     YoleckTypeHandler::<Example>::new("Example")
///         .with(yoleck_vpeol_transform_edit_adapter(
///             |data: &mut Example| {
///                 YoleckVpeolTransform3dProjection {
///                     translation: &mut data.position,
///                     rotation: Some(&mut data.rotation),
///                 }
///             }
///         ))
///         .populate_with(populate_example)
/// });
///
/// fn populate_example(mut populate: YoleckPopulate<Example>) {
///     populate.populate(|_ctx, data, mut cmd| {
///         cmd.insert_bundle(PbrBundle {
///             transform: Transform::from_translation(data.position),
///             // Actual PBR components
///             ..Default::default()
///         });
///         // No need to add PickableBundle and GizmoTransformable - the adapter already did so.
///     });
/// }
/// ```
pub fn yoleck_vpeol_transform_edit_adapter<T: 'static>(
    projection: impl 'static
        + Clone
        + Send
        + Sync
        + for<'a> Fn(&'a mut T) -> YoleckVpeolTransform3dProjection<'a>,
) -> impl FnOnce(YoleckTypeHandler<T>) -> YoleckTypeHandler<T> {
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
                    let YoleckVpeolTransform3dProjection {
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
