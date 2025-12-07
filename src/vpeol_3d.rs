//! # Viewport Editing Overlay for 3D games.
//!
//! Use this module to implement simple 3D editing for 3D games.
//!
//! To use add the egui and Yoleck plugins to the Bevy app, as well as the plugin of this module:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoleck::bevy_egui::EguiPlugin;
//! # use bevy_yoleck::prelude::*;
//! # use bevy_yoleck::vpeol::prelude::*;
//! # let mut app = App::new();
//! app.add_plugins(EguiPlugin::default());
//! app.add_plugins(YoleckPluginForEditor);
//! // - Use `Vpeol3dPluginForGame` instead when setting up for game.
//! // - Use topdown is for games that utilize the XZ plane. There is also
//! //   `Vpeol3dPluginForEditor::sidescroller` for games that mainly need the XY plane.
//! app.add_plugins(Vpeol3dPluginForEditor::topdown());
//! ```
//!
//! Add the following components to the camera entity:
//! * [`VpeolCameraState`] in order to select and drag entities.
//! * [`Vpeol3dCameraControl`] in order to control the camera with the mouse. This one can be
//!   skipped if there are other means to control the camera inside the editor, or if no camera
//!   control inside the editor is desired.
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoleck::vpeol::prelude::*;
//! # let commands: Commands = panic!();
//! commands
//!     .spawn(Camera3d::default())
//!     .insert(VpeolCameraState::default())
//!     // Use a variant of the camera controls that fit the choice of editor plugin.
//!     .insert(Vpeol3dCameraControl::topdown());
//! ```
//!
//! ## Custom Camera Modes
//!
//! You can customize available camera modes using [`YoleckCameraChoices`]:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_yoleck::vpeol::prelude::*;
//! # let mut app = App::new();
//! app.insert_resource(
//!     YoleckCameraChoices::default()
//!         .choice_with_transform(
//!             "Custom Camera",
//!             || {
//!                 let mut control = Vpeol3dCameraControl::fps();
//!                 control.mode_name = "Custom Camera".to_string();
//!                 control
//!             },
//!             Vec3::new(10.0, 10.0, 10.0),
//!             Vec3::ZERO,
//!             Vec3::Y,
//!         )
//! );
//!
//! // Implement custom camera movement
//! app.add_systems(PostUpdate, custom_camera_movement);
//!
//! fn custom_camera_movement(
//!     mut cameras: Query<(&mut Transform, &Vpeol3dCameraControl)>,
//! ) {
//!     for (mut transform, control) in cameras.iter_mut() {
//!         if control.mode_name == "Custom Camera" {
//!             // Your custom camera logic here
//!         }
//!     }
//! }
//! ```
//!
//! Entity selection by clicking on it is supported by just adding the plugin. To implement
//! dragging, there are two options:
//!
//! 1. Add the [`Vpeol3dPosition`] Yoleck component and use it as the source of position. Axis knobs
//!    (X, Y, Z) are automatically added to all entities with `Vpeol3dPosition`. Configure them using
//!    the [`Vpeol3dKnobsConfig`] resource. Optionally add [`Vpeol3dRotation`] (edited with Euler
//!    angles) and [`Vpeol3dScale`] (edited with X, Y, Z values) for rotation and scale support.
//!     ```no_run
//!     # use bevy::prelude::*;
//!     # use bevy_yoleck::prelude::*;
//!     # use bevy_yoleck::vpeol::prelude::*;
//!     # use serde::{Deserialize, Serialize};
//!     # #[derive(Clone, PartialEq, Serialize, Deserialize, Component, Default, YoleckComponent)]
//!     # struct Example;
//!     # let mut app = App::new();
//!     app.add_yoleck_entity_type({
//!         YoleckEntityType::new("Example")
//!             .with::<Vpeol3dPosition>() // vpeol_3d dragging with axis knobs
//!             .with::<Vpeol3dRotation>() // optional: rotation with egui (Euler angles)
//!             .with::<Vpeol3dScale>() // optional: scale with egui
//!             .with::<Example>() // entity's specific data and systems
//!     });
//!     ```
//! 2. Use data passing. vpeol_3d will pass a `Vec3` to the entity being dragged:
//!     ```no_run
//!     # use bevy::prelude::*;
//!     # use bevy_yoleck::prelude::*;
//!     # use serde::{Deserialize, Serialize};
//!     # #[derive(Clone, PartialEq, Serialize, Deserialize, Component, Default, YoleckComponent)]
//!     # struct Example {
//!     #     position: Vec3,
//!     # }
//!     # let mut app = App::new();
//!     fn edit_example(mut edit: YoleckEdit<(Entity, &mut Example)>, passed_data: Res<YoleckPassedData>) {
//!         let Ok((entity, mut example)) = edit.single_mut() else { return };
//!         if let Some(pos) = passed_data.get::<Vec3>(entity) {
//!             example.position = *pos;
//!         }
//!     }
//!
//!     fn populate_example(
//!         mut populate: YoleckPopulate<&Example>,
//!         asset_server: Res<AssetServer>
//!     ) {
//!         populate.populate(|_ctx, mut cmd, example| {
//!             cmd.insert(Transform::from_translation(example.position));
//!             cmd.insert(SceneRoot(asset_server.load("scene.glb#Scene0")));
//!         });
//!     }
//!     ```

use std::any::TypeId;

use crate::bevy_egui::egui;
use crate::exclusive_systems::{
    YoleckEntityCreationExclusiveSystems, YoleckExclusiveSystemDirective,
};
use crate::vpeol::{
    handle_clickable_children_system, ray_intersection_with_mesh, VpeolBasePlugin,
    VpeolCameraState, VpeolClicksOnObjectsState, VpeolDragPlane, VpeolRepositionLevel,
    VpeolRootResolver, VpeolSystems,
};
use crate::{prelude::*, YoleckDirective, YoleckSchedule, YoleckEditMarker, YoleckState, YoleckBelongsToLevel, YoleckEditorTopPanelSections};
use crate::editor::YoleckEditorEvent;
use crate::entity_management::YoleckRawEntry;
use crate::{YoleckManaged, YoleckEntityConstructionSpecs};
use bevy::camera::visibility::VisibleEntities;
use bevy::color::palettes::css;
use bevy::ecs::system::SystemState;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_egui::EguiContexts;
use serde::{Deserialize, Serialize};

/// Add the systems required for loading levels that use vpeol_3d components
pub struct Vpeol3dPluginForGame;

impl Plugin for Vpeol3dPluginForGame {
    fn build(&self, app: &mut App) {
        app.add_systems(
            YoleckSchedule::OverrideCommonComponents,
            vpeol_3d_populate_transform,
        );
        #[cfg(feature = "bevy_reflect")]
        register_reflect_types(app);
    }
}

#[cfg(feature = "bevy_reflect")]
fn register_reflect_types(app: &mut App) {
    app.register_type::<Vpeol3dPosition>();
    app.register_type::<Vpeol3dRotation>();
    app.register_type::<Vpeol3dScale>();
    app.register_type::<Vpeol3dCameraControl>();
}

/// Add the systems required for 3D editing.
///
/// * 3D camera control (for cameras with [`Vpeol3dCameraControl`])
/// * Entity selection.
/// * Entity dragging.
/// * Connecting nested entities.
pub struct Vpeol3dPluginForEditor {
    /// The plane to configure the global [`VpeolDragPlane`] resource with.
    ///
    /// Indiviual entities can override this with their own [`VpeolDragPlane`] component.
    ///
    /// It is a good idea to match this to [`Vpeol3dCameraControl::plane`].
    pub drag_plane: InfinitePlane3d,
}

impl Vpeol3dPluginForEditor {
    /// For sidescroller games - drag entities along the XY plane.
    ///
    /// Indiviual entities can override this with a [`VpeolDragPlane`] component.
    ///
    /// Adding [`Vpeol3dThirdAxisWithKnob`] can be used to allow Z axis manipulation.
    ///
    /// This combines well with [`Vpeol3dCameraControl::sidescroller`].
    pub fn sidescroller() -> Self {
        Self {
            drag_plane: InfinitePlane3d { normal: Dir3::Z },
        }
    }

    /// For games that are not sidescrollers - drag entities along the XZ plane.
    ///
    /// Indiviual entities can override this with a [`VpeolDragPlane`] component.
    ///
    /// Adding [`Vpeol3dThirdAxisWithKnob`] can be used to allow Y axis manipulation.
    ///
    /// This combines well with [`Vpeol3dCameraControl::topdown`].
    pub fn topdown() -> Self {
        Self {
            drag_plane: InfinitePlane3d { normal: Dir3::Y },
        }
    }
}

impl Plugin for Vpeol3dPluginForEditor {
    fn build(&self, app: &mut App) {
        app.add_plugins(VpeolBasePlugin);
        app.add_plugins(Vpeol3dPluginForGame);
        app.insert_resource(VpeolDragPlane(self.drag_plane));
        app.init_resource::<Vpeol3dKnobsConfig>();
        app.init_resource::<YoleckCameraChoices>();

        app.world_mut()
            .resource_mut::<YoleckEditorTopPanelSections>()
            .0
            .push(vpeol_3d_camera_mode_selector.into());
        app.world_mut()
            .resource_mut::<YoleckEditorTopPanelSections>()
            .0
            .push(vpeol_3d_knobs_mode_selector.into());

        app.add_systems(
            Update,
            (update_camera_status_for_models,).in_set(VpeolSystems::UpdateCameraState),
        );
        app.add_systems(
            PostUpdate,
            (
                camera_3d_wasd_movement,
                camera_3d_move_along_plane_normal,
                camera_3d_rotate,
            )
                .run_if(in_state(YoleckEditorState::EditorActive)),
        );
        app.add_systems(
            Update,
            (handle_delete_entity_key, handle_copy_entity_key, handle_paste_entity_key)
                .run_if(in_state(YoleckEditorState::EditorActive)),
        );
        app.add_systems(
            Update,
            draw_scene_gizmo.run_if(in_state(YoleckEditorState::EditorActive)),
        );
        app.add_systems(
            Update,
            (
                ApplyDeferred,
                handle_clickable_children_system::<With<Mesh3d>, ()>,
                ApplyDeferred,
            )
                .chain()
                .run_if(in_state(YoleckEditorState::EditorActive)),
        );
        app.add_yoleck_edit_system(vpeol_3d_edit_transform_group);
        app.world_mut()
            .resource_mut::<YoleckEntityCreationExclusiveSystems>()
            .on_entity_creation(|queue| queue.push_back(vpeol_3d_init_position));
        app.add_yoleck_edit_system(vpeol_3d_edit_axis_knobs);
    }
}

fn update_camera_status_for_models(
    mut cameras_query: Query<(&mut VpeolCameraState, &VisibleEntities)>,
    entities_query: Query<(Entity, &GlobalTransform, &Mesh3d)>,
    mesh_assets: Res<Assets<Mesh>>,
    root_resolver: VpeolRootResolver,
) {
    for (mut camera_state, visible_entities) in cameras_query.iter_mut() {
        let Some(cursor_ray) = camera_state.cursor_ray else {
            continue;
        };
        for (entity, global_transform, mesh) in
            entities_query.iter_many(visible_entities.iter(TypeId::of::<Mesh3d>()))
        {
            let Some(mesh) = mesh_assets.get(&mesh.0) else {
                continue;
            };

            let inverse_transform = global_transform.to_matrix().inverse();

            // Note: the transform may change the ray's length, which Bevy no longer supports
            // (since version 0.13), so we keep the ray length separately and apply it later to the
            // distance.
            let ray_origin = inverse_transform.transform_point3(cursor_ray.origin);
            let ray_vector = inverse_transform.transform_vector3(*cursor_ray.direction);
            let Ok((ray_direction, ray_length_factor)) = Dir3::new_and_length(ray_vector) else {
                continue;
            };

            let ray_in_object_coords = Ray3d {
                origin: ray_origin,
                direction: ray_direction,
            };

            let Some(distance) = ray_intersection_with_mesh(ray_in_object_coords, mesh) else {
                continue;
            };

            let distance = distance / ray_length_factor;

            let Some(root_entity) = root_resolver.resolve_root(entity) else {
                continue;
            };
            camera_state.consider(root_entity, -distance, || cursor_ray.get_point(distance));
        }
    }
}

/// A single camera mode choice with its constructor and optional initial transform.
pub struct YoleckCameraChoice {
    pub name: String,
    pub constructor: Box<dyn Fn() -> Vpeol3dCameraControl + Send + Sync>,
    pub initial_transform: Option<(Vec3, Vec3, Vec3)>,
}

/// Resource that defines available camera modes in the editor.
///
/// This allows users to customize which camera modes are available and add custom modes.
///
/// # Example
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::vpeol::prelude::*;
/// # let mut app = App::new();
/// app.insert_resource(
///     YoleckCameraChoices::default()
///         .choice("Custom FPS", || Vpeol3dCameraControl::fps())
///         .choice_with_transform(
///             "Isometric",
///             || {
///                 let mut control = Vpeol3dCameraControl::fps();
///                 control.mode_name = "Isometric".to_string();
///                 control.allow_rotation_while_maintaining_up = None;
///                 control
///             },
///             Vec3::new(10.0, 10.0, 10.0),
///             Vec3::ZERO,
///             Vec3::Y,
///         )
/// );
/// ```
#[derive(Resource)]
pub struct YoleckCameraChoices {
    pub choices: Vec<YoleckCameraChoice>,
}

impl YoleckCameraChoices {
    pub fn new() -> Self {
        Self {
            choices: Vec::new(),
        }
    }

    pub fn choice(
        mut self,
        name: impl Into<String>,
        constructor: impl Fn() -> Vpeol3dCameraControl + Send + Sync + 'static,
    ) -> Self {
        self.choices.push(YoleckCameraChoice {
            name: name.into(),
            constructor: Box::new(constructor),
            initial_transform: None,
        });
        self
    }

    pub fn choice_with_transform(
        mut self,
        name: impl Into<String>,
        constructor: impl Fn() -> Vpeol3dCameraControl + Send + Sync + 'static,
        position: Vec3,
        look_at: Vec3,
        up: Vec3,
    ) -> Self {
        self.choices.push(YoleckCameraChoice {
            name: name.into(),
            constructor: Box::new(constructor),
            initial_transform: Some((position, look_at, up)),
        });
        self
    }
}

impl Default for YoleckCameraChoices {
    fn default() -> Self {
        Self::new()
            .choice_with_transform(
                "FPS",
                Vpeol3dCameraControl::fps,
                Vec3::ZERO,
                Vec3::NEG_Z,
                Vec3::Y,
            )
            .choice_with_transform(
                "Sidescroller",
                Vpeol3dCameraControl::sidescroller,
                Vec3::new(0.0, 0.0, 10.0),
                Vec3::ZERO,
                Vec3::Y,
            )
            .choice_with_transform(
                "Topdown",
                Vpeol3dCameraControl::topdown,
                Vec3::new(0.0, 10.0, 0.0),
                Vec3::ZERO,
                Vec3::NEG_Z,
            )
    }
}

/// Move and rotate a camera entity with the mouse while inside the editor.
#[derive(Component)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy::reflect::Reflect))]
pub struct Vpeol3dCameraControl {
    /// Name of the camera mode, used to identify which mode is currently active.
    pub mode_name: String,
    /// Defines the plane normal for mouse wheel zoom movement.
    pub plane: InfinitePlane3d,
    /// If `Some`, enable mouse rotation. The up direction of the camera will be the specific
    /// direction.
    ///
    /// It is a good idea to match this to [`Vpeol3dPluginForEditor::drag_plane`].
    pub allow_rotation_while_maintaining_up: Option<Dir3>,
    /// How much to change the proximity to the plane when receiving scroll event in
    /// `MouseScrollUnit::Line` units.
    pub proximity_per_scroll_line: f32,
    /// How much to change the proximity to the plane when receiving scroll event in
    /// `MouseScrollUnit::Pixel` units.
    pub proximity_per_scroll_pixel: f32,
    /// Movement speed for WASD controls (units per second).
    pub wasd_movement_speed: f32,
    /// Mouse sensitivity for camera rotation.
    pub mouse_sensitivity: f32,
}

impl Vpeol3dCameraControl {
    /// Preset for FPS-style camera control with full rotation freedom.
    ///
    /// This mode allows complete free-look rotation with mouse and WASD movement.
    pub fn fps() -> Self {
        Self {
            mode_name: "FPS".to_string(),
            plane: InfinitePlane3d { normal: Dir3::Y },
            allow_rotation_while_maintaining_up: Some(Dir3::Y),
            proximity_per_scroll_line: 2.0,
            proximity_per_scroll_pixel: 0.01,
            wasd_movement_speed: 10.0,
            mouse_sensitivity: 0.003,
        }
    }

    /// Preset for sidescroller games, where the the game world is on the XY plane.
    ///
    /// With this preset, the camera stays fixed looking at the scene from the side.
    ///
    /// This combines well with [`Vpeol3dPluginForEditor::sidescroller`].
    pub fn sidescroller() -> Self {
        Self {
            mode_name: "Sidescroller".to_string(),
            plane: InfinitePlane3d {
                normal: Dir3::NEG_Z,
            },
            allow_rotation_while_maintaining_up: None,
            proximity_per_scroll_line: 2.0,
            proximity_per_scroll_pixel: 0.01,
            wasd_movement_speed: 10.0,
            mouse_sensitivity: 0.003,
        }
    }

    /// Preset for games where the the game world is mainly on the XZ plane (though there can still
    /// be verticality)
    ///
    /// This combines well with [`Vpeol3dPluginForEditor::topdown`].
    pub fn topdown() -> Self {
        Self {
            mode_name: "Topdown".to_string(),
            plane: InfinitePlane3d { normal: Dir3::NEG_Y },
            allow_rotation_while_maintaining_up: None,
            proximity_per_scroll_line: 2.0,
            proximity_per_scroll_pixel: 0.01,
            wasd_movement_speed: 10.0,
            mouse_sensitivity: 0.003,
        }
    }
}

fn camera_3d_wasd_movement(
    mut egui_context: EguiContexts,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cameras_query: Query<(&mut Transform, &Vpeol3dCameraControl)>,
) -> Result {
    if egui_context.ctx_mut()?.wants_keyboard_input() {
        return Ok(());
    }

    let mut direction = Vec3::ZERO;

    if keyboard_input.pressed(KeyCode::KeyW) {
        direction += Vec3::NEG_Z;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        direction += Vec3::Z;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        direction += Vec3::NEG_X;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        direction += Vec3::X;
    }
    if keyboard_input.pressed(KeyCode::KeyE) {
        direction += Vec3::Y;
    }
    if keyboard_input.pressed(KeyCode::KeyQ) {
        direction += Vec3::NEG_Y;
    }

    if direction == Vec3::ZERO {
        return Ok(());
    }

    direction = direction.normalize_or_zero();

    let speed_multiplier = if keyboard_input.pressed(KeyCode::ShiftLeft) {
        2.0
    } else {
        1.0
    };

    for (mut camera_transform, camera_control) in cameras_query.iter_mut() {
        let movement = if camera_control.mode_name == "Sidescroller" {
            let mut world_direction = Vec3::ZERO;
            if keyboard_input.pressed(KeyCode::KeyW) {
                world_direction.y += 1.0;
            }
            if keyboard_input.pressed(KeyCode::KeyS) {
                world_direction.y -= 1.0;
            }
            if keyboard_input.pressed(KeyCode::KeyA) {
                world_direction.x -= 1.0;
            }
            if keyboard_input.pressed(KeyCode::KeyD) {
                world_direction.x += 1.0;
            }
            world_direction.normalize_or_zero()
                * camera_control.wasd_movement_speed
                * speed_multiplier
                * time.delta_secs()
        } else if camera_control.mode_name == "Topdown" {
            let mut world_direction = Vec3::ZERO;
            if keyboard_input.pressed(KeyCode::KeyW) {
                world_direction.z -= 1.0;
            }
            if keyboard_input.pressed(KeyCode::KeyS) {
                world_direction.z += 1.0;
            }
            if keyboard_input.pressed(KeyCode::KeyA) {
                world_direction.x -= 1.0;
            }
            if keyboard_input.pressed(KeyCode::KeyD) {
                world_direction.x += 1.0;
            }
            world_direction.normalize_or_zero()
                * camera_control.wasd_movement_speed
                * speed_multiplier
                * time.delta_secs()
        } else {
            camera_transform.rotation
                * direction
                * camera_control.wasd_movement_speed
                * speed_multiplier
                * time.delta_secs()
        };

        camera_transform.translation += movement;
    }
    Ok(())
}

fn camera_3d_move_along_plane_normal(
    mut egui_context: EguiContexts,
    mut cameras_query: Query<(&mut Transform, &Vpeol3dCameraControl)>,
    mut wheel_events_reader: MessageReader<MouseWheel>,
) -> Result {
    if egui_context.ctx_mut()?.is_pointer_over_area() {
        return Ok(());
    }

    for (mut camera_transform, camera_control) in cameras_query.iter_mut() {
        let zoom_amount: f32 = wheel_events_reader
            .read()
            .map(|wheel_event| match wheel_event.unit {
                bevy::input::mouse::MouseScrollUnit::Line => {
                    wheel_event.y * camera_control.proximity_per_scroll_line
                }
                bevy::input::mouse::MouseScrollUnit::Pixel => {
                    wheel_event.y * camera_control.proximity_per_scroll_pixel
                }
            })
            .sum();

        if zoom_amount == 0.0 {
            continue;
        }

        camera_transform.translation += zoom_amount * *camera_control.plane.normal;
    }
    Ok(())
}

fn handle_delete_entity_key(
    mut egui_context: EguiContexts,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut yoleck_state: ResMut<YoleckState>,
    query: Query<Entity, With<YoleckEditMarker>>,
    mut commands: Commands,
    mut writer: MessageWriter<YoleckEditorEvent>,
) -> Result {
    if egui_context.ctx_mut()?.wants_keyboard_input() {
        return Ok(());
    }

    if keyboard_input.just_pressed(KeyCode::Delete) {
        for entity in query.iter() {
            commands.entity(entity).despawn();
            writer.write(YoleckEditorEvent::EntityDeselected(entity));
        }
        if !query.is_empty() {
            yoleck_state.level_needs_saving = true;
        }
    }

    Ok(())
}

fn handle_copy_entity_key(
    mut egui_context: EguiContexts,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    query: Query<&YoleckManaged, With<YoleckEditMarker>>,
    construction_specs: Res<YoleckEntityConstructionSpecs>,
) -> Result {
    if egui_context.ctx_mut()?.wants_keyboard_input() {
        return Ok(());
    }

    let ctrl_pressed = keyboard_input.pressed(KeyCode::ControlLeft) 
        || keyboard_input.pressed(KeyCode::ControlRight);

    if ctrl_pressed && keyboard_input.just_pressed(KeyCode::KeyC) {
        let entities: Vec<YoleckRawEntry> = query
            .iter()
            .filter_map(|yoleck_managed| {
                let entity_type = construction_specs.get_entity_type_info(&yoleck_managed.type_name)?;
                
                let data: serde_json::Map<String, serde_json::Value> = entity_type
                    .components
                    .iter()
                    .filter_map(|component| {
                        let component_data = yoleck_managed.components_data.get(component)?;
                        let handler = &construction_specs.component_handlers[component];
                        Some((
                            handler.key().to_string(),
                            handler.serialize(component_data.as_ref()),
                        ))
                    })
                    .collect();

                Some(YoleckRawEntry {
                    header: crate::entity_management::YoleckEntryHeader {
                        type_name: yoleck_managed.type_name.clone(),
                        name: yoleck_managed.name.clone(),
                        uuid: None,
                    },
                    data: serde_json::Value::Object(data),
                })
            })
            .collect();

        if !entities.is_empty() {
            if let Ok(json) = serde_json::to_string(&entities) {
                let mut clipboard = arboard::Clipboard::new()?;
                clipboard.set_text(json)?;
            }
        }
    }

    Ok(())
}

fn handle_paste_entity_key(
    mut egui_context: EguiContexts,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut yoleck_state: ResMut<YoleckState>,
    mut commands: Commands,
    mut writer: MessageWriter<YoleckEditorEvent>,
    query: Query<Entity, With<YoleckEditMarker>>,
) -> Result {
    if egui_context.ctx_mut()?.wants_keyboard_input() {
        return Ok(());
    }

    let ctrl_pressed = keyboard_input.pressed(KeyCode::ControlLeft) 
        || keyboard_input.pressed(KeyCode::ControlRight);

    if ctrl_pressed && keyboard_input.just_pressed(KeyCode::KeyV) {
        let mut clipboard = arboard::Clipboard::new()?;
        if let Ok(text) = clipboard.get_text() {
            if let Ok(entities) = serde_json::from_str::<Vec<YoleckRawEntry>>(&text) {
                if !entities.is_empty() {
                    for prev_selected in query.iter() {
                        commands.entity(prev_selected).remove::<YoleckEditMarker>();
                        writer.write(YoleckEditorEvent::EntityDeselected(prev_selected));
                    }
                    
                    let level_being_edited = yoleck_state.level_being_edited;
                    
                    for entry in entities {
                        let entity_id = commands.spawn((
                            entry,
                            YoleckBelongsToLevel {
                                level: level_being_edited,
                            },
                            YoleckEditMarker,
                        )).id();
                        
                        writer.write(YoleckEditorEvent::EntitySelected(entity_id));
                    }
                    
                    yoleck_state.level_needs_saving = true;
                }
            }
        }
    }

    Ok(())
}

fn draw_scene_gizmo(
    mut egui_context: EguiContexts,
    mut cameras_query: Query<&mut Transform, With<VpeolCameraState>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut first_frame_skipped: Local<bool>,
    editor_viewport: Res<crate::editor_window::YoleckEditorViewportRect>,
) -> Result {
    if !*first_frame_skipped {
        *first_frame_skipped = true;
        return Ok(());
    }

    let ctx = egui_context.ctx_mut()?;

    if !ctx.is_using_pointer() && ctx.input(|i| i.viewport_rect().width() == 0.0) {
        return Ok(());
    }

    let Ok(mut camera_transform) = cameras_query.single_mut() else {
        return Ok(());
    };

    let screen_rect = editor_viewport.rect.unwrap_or_else(|| ctx.input(|i| i.viewport_rect()));

    if screen_rect.width() == 0.0 || screen_rect.height() == 0.0 {
        return Ok(());
    }

    let gizmo_size = 60.0;
    let axis_length = 25.0;
    let margin = 20.0;
    let click_radius = 10.0;

    let center = egui::Pos2::new(
        screen_rect.max.x - margin - gizmo_size / 2.0,
        screen_rect.min.y + margin + gizmo_size / 2.0,
    );

    let camera_rotation = camera_transform.rotation;
    let inv_rotation = camera_rotation.inverse();

    let world_x = inv_rotation * Vec3::X;
    let world_y = inv_rotation * Vec3::Y;
    let world_z = inv_rotation * Vec3::Z;

    let to_screen = |v: Vec3| -> egui::Pos2 {
        let perspective_scale = 1.0 / (1.0 - v.z * 0.3);
        let screen_x = v.x * axis_length * perspective_scale;
        let screen_y = v.y * axis_length * perspective_scale;

        let len = (screen_x * screen_x + screen_y * screen_y).sqrt();
        let min_len = 8.0;
        let (screen_x, screen_y) = if len < min_len && len > 0.001 {
            let scale = min_len / len;
            (screen_x * scale, screen_y * scale)
        } else {
            (screen_x, screen_y)
        };

        egui::Pos2::new(center.x + screen_x, center.y - screen_y)
    };

    let x_pos = to_screen(world_x);
    let x_neg = to_screen(-world_x);
    let y_pos = to_screen(world_y);
    let y_neg = to_screen(-world_y);
    let z_pos = to_screen(world_z);
    let z_neg = to_screen(-world_z);

    let cursor_pos = ctx.input(|i| i.pointer.hover_pos());
    let gizmo_rect = egui::Rect::from_center_size(center, egui::Vec2::splat(gizmo_size));

    if let Some(cursor) = cursor_pos {
        if mouse_buttons.just_pressed(MouseButton::Left) {
            if gizmo_rect.contains(cursor) {
                let distances = [
                    (cursor.distance(x_pos), Vec3::NEG_X, Vec3::Y),
                    (cursor.distance(x_neg), Vec3::X, Vec3::Y),
                    (cursor.distance(y_pos), Vec3::NEG_Y, Vec3::Z),
                    (cursor.distance(y_neg), Vec3::Y, Vec3::Z),
                    (cursor.distance(z_pos), Vec3::NEG_Z, Vec3::Y),
                    (cursor.distance(z_neg), Vec3::Z, Vec3::Y),
                ];

                if let Some((_, forward, up)) = distances
                    .iter()
                    .filter(|(d, _, _)| *d < click_radius)
                    .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
                {
                    camera_transform.look_to(*forward, *up);
                }
            }
        }
    }

    #[derive(Clone, Copy)]
    struct AxisData {
        depth: f32,
        color_bright: egui::Color32,
        color_dim: egui::Color32,
        pos_end: egui::Pos2,
        neg_end: egui::Pos2,
        world_dir: Vec3,
    }

    let mut axes = vec![
        AxisData {
            depth: world_x.z.abs(),
            color_bright: egui::Color32::from_rgb(230, 60, 60),
            color_dim: egui::Color32::from_rgb(120, 50, 50),
            pos_end: x_pos,
            neg_end: x_neg,
            world_dir: world_x,
        },
        AxisData {
            depth: world_y.z.abs(),
            color_bright: egui::Color32::from_rgb(60, 230, 60),
            color_dim: egui::Color32::from_rgb(50, 120, 50),
            pos_end: y_pos,
            neg_end: y_neg,
            world_dir: world_y,
        },
        AxisData {
            depth: world_z.z.abs(),
            color_bright: egui::Color32::from_rgb(60, 120, 230),
            color_dim: egui::Color32::from_rgb(50, 70, 120),
            pos_end: z_pos,
            neg_end: z_neg,
            world_dir: world_z,
        },
    ];
    axes.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap());

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("scene_gizmo"),
    ));

    painter.circle_filled(
        center,
        gizmo_size / 2.0,
        egui::Color32::from_rgba_unmultiplied(40, 40, 40, 200),
    );

    let stroke_bright = 3.0;
    let stroke_dim = 2.0;
    let cone_radius_bright = 5.0;
    let cone_radius_dim = 3.5;
    let cone_length = 8.0;

    for axis in &axes {
        let (front_end, front_color, back_end, back_color) = if axis.world_dir.z >= 0.0 {
            (
                axis.pos_end,
                axis.color_bright,
                axis.neg_end,
                axis.color_dim,
            )
        } else {
            (
                axis.neg_end,
                axis.color_dim,
                axis.pos_end,
                axis.color_bright,
            )
        };

        let back_dir = (back_end - center).normalized();
        let back_line_end = back_end - back_dir * cone_length;
        painter.line_segment(
            [center, back_line_end],
            egui::Stroke::new(stroke_dim, back_color),
        );

        let back_perp = egui::Vec2::new(-back_dir.y, back_dir.x);
        let back_cone_base = back_end - back_dir * cone_length;
        let back_cone = vec![
            back_end,
            back_cone_base + back_perp * cone_radius_dim,
            back_cone_base - back_perp * cone_radius_dim,
        ];
        painter.add(egui::Shape::convex_polygon(
            back_cone,
            back_color,
            egui::Stroke::NONE,
        ));

        let front_dir = (front_end - center).normalized();
        let front_line_end = front_end - front_dir * cone_length;
        painter.line_segment(
            [center, front_line_end],
            egui::Stroke::new(stroke_bright, front_color),
        );

        let front_perp = egui::Vec2::new(-front_dir.y, front_dir.x);
        let front_cone_base = front_end - front_dir * cone_length;
        let front_cone = vec![
            front_end,
            front_cone_base + front_perp * cone_radius_bright,
            front_cone_base - front_perp * cone_radius_bright,
        ];
        painter.add(egui::Shape::convex_polygon(
            front_cone,
            front_color,
            egui::Stroke::NONE,
        ));
    }

    let label_offset = 12.0;
    let font_id = egui::FontId::proportional(12.0);

    let axis_labels = [
        ("X", x_pos, egui::Color32::from_rgb(230, 60, 60), world_x.z),
        ("Y", y_pos, egui::Color32::from_rgb(60, 230, 60), world_y.z),
        ("Z", z_pos, egui::Color32::from_rgb(60, 120, 230), world_z.z),
    ];

    for (label, pos, color, depth) in axis_labels {
        let dir = (pos - center).normalized();
        let label_pos = pos + dir * label_offset;
        let alpha = if depth >= 0.0 { 255 } else { 120 };
        let label_color = egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
        painter.text(
            label_pos,
            egui::Align2::CENTER_CENTER,
            label,
            font_id.clone(),
            label_color,
        );
    }

    Ok(())
}

fn camera_3d_rotate(
    mut egui_context: EguiContexts,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut cameras_query: Query<(&mut Transform, &Vpeol3dCameraControl)>,
    mut mouse_motion_reader: MessageReader<MouseMotion>,
    mut cursor_options: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut is_rotating: Local<bool>,
) -> Result {
    let Ok(mut cursor) = cursor_options.single_mut() else {
        return Ok(());
    };

    if mouse_buttons.just_pressed(MouseButton::Right) {
        if egui_context.ctx_mut()?.is_pointer_over_area() {
            return Ok(());
        }
        
        let has_rotatable_camera = cameras_query
            .iter()
            .any(|(_, control)| control.allow_rotation_while_maintaining_up.is_some());
        
        if has_rotatable_camera {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
            *is_rotating = true;
        }
    }

    if mouse_buttons.just_released(MouseButton::Right) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
        *is_rotating = false;
    }

    if !*is_rotating {
        return Ok(());
    }

    let mut delta = Vec2::ZERO;
    for motion in mouse_motion_reader.read() {
        delta += motion.delta;
    }

    if delta == Vec2::ZERO {
        return Ok(());
    }

    for (mut camera_transform, camera_control) in cameras_query.iter_mut() {
        let Some(maintaining_up) = camera_control.allow_rotation_while_maintaining_up else {
            continue;
        };

        let yaw = -delta.x * camera_control.mouse_sensitivity;
        let pitch = -delta.y * camera_control.mouse_sensitivity;

        let yaw_rotation = Quat::from_axis_angle(*maintaining_up, yaw);
        camera_transform.rotation = yaw_rotation * camera_transform.rotation;

        let right = camera_transform.right();
        let pitch_rotation = Quat::from_axis_angle(*right, pitch);
        camera_transform.rotation = pitch_rotation * camera_transform.rotation;

        let new_forward = camera_transform.forward();
        camera_transform.look_to(*new_forward, *maintaining_up);
    }
    Ok(())
}

pub fn vpeol_3d_knobs_mode_selector(
    world: &mut World,
) -> impl FnMut(&mut World, &mut egui::Ui) -> Result {
    let mut system_state = SystemState::<ResMut<Vpeol3dKnobsConfig>>::new(world);

    move |world, ui: &mut egui::Ui| {
        let mut config = system_state.get_mut(world);

        ui.add_space(ui.available_width());

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.radio_value(&mut config.mode, Vpeol3dKnobsMode::Local, "Local");
            ui.radio_value(&mut config.mode, Vpeol3dKnobsMode::World, "World");
            ui.label("Knobs:");
        });
        
        Ok(())
    }
}

pub fn vpeol_3d_camera_mode_selector(
    world: &mut World,
) -> impl FnMut(&mut World, &mut egui::Ui) -> Result {
    let mut system_state = SystemState::<(
        Query<(&mut Vpeol3dCameraControl, &mut Transform)>,
        Res<YoleckCameraChoices>,
    )>::new(world);

    move |world, ui: &mut egui::Ui| {
        let (mut query, choices) = system_state.get_mut(world);
        
        if let Ok((mut camera_control, mut camera_transform)) = query.single_mut() {
            let old_mode_name = camera_control.mode_name.clone();

            egui::ComboBox::from_id_salt("camera_mode_selector")
                .selected_text(&camera_control.mode_name)
                .show_ui(ui, |ui| {
                    for choice in choices.choices.iter() {
                        ui.selectable_value(&mut camera_control.mode_name, choice.name.clone(), &choice.name);
                    }
                });
            
            if old_mode_name != camera_control.mode_name {
                if let Some(choice) = choices.choices.iter().find(|c| c.name == camera_control.mode_name) {
                    let new_control = (choice.constructor)();
                    *camera_control = new_control;
                    
                    if let Some((position, look_at, up)) = choice.initial_transform {
                        camera_transform.translation = position;
                        camera_transform.look_at(look_at, up);
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// A position component that's edited and populated by vpeol_3d.
///
/// Editing is done with egui, or by dragging the entity on a [`VpeolDragPlane`]  that passes
/// through the entity. To support dragging perpendicular to that plane, use
/// [`Vpeol3dThirdAxisWithKnob`].
#[derive(Clone, PartialEq, Serialize, Deserialize, Component, Default, YoleckComponent)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy::reflect::Reflect))]
pub struct Vpeol3dPosition(pub Vec3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vpeol3dKnobsMode {
    World,
    Local,
}

#[derive(Resource)]
pub struct Vpeol3dKnobsConfig {
    pub knob_distance: f32,
    pub knob_scale: f32,
    pub mode: Vpeol3dKnobsMode,
}

impl Default for Vpeol3dKnobsConfig {
    fn default() -> Self {
        Self {
            knob_distance: 2.0,
            knob_scale: 0.5,
            mode: Vpeol3dKnobsMode::World,
        }
    }
}

/// A rotation component that's edited and populated by vpeol_3d.
///
/// Editing is done with egui using Euler angles (X, Y, Z in degrees).
#[derive(Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy::reflect::Reflect))]
pub struct Vpeol3dRotation(pub Vec3);

impl Default for Vpeol3dRotation {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}

/// A scale component that's edited and populated by vpeol_3d.
///
/// Editing is done with egui using separate drag values for X, Y, Z axes.
#[derive(Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy::reflect::Reflect))]
pub struct Vpeol3dScale(pub Vec3);

impl Default for Vpeol3dScale {
    fn default() -> Self {
        Self(Vec3::ONE)
    }
}

enum CommonDragPlane {
    NotDecidedYet,
    WithNormal(Vec3),
    NoSharedPlane,
}

impl CommonDragPlane {
    fn consider(&mut self, normal: Vec3) {
        *self = match self {
            CommonDragPlane::NotDecidedYet => CommonDragPlane::WithNormal(normal),
            CommonDragPlane::WithNormal(current_normal) => {
                if *current_normal == normal {
                    CommonDragPlane::WithNormal(normal)
                } else {
                    CommonDragPlane::NoSharedPlane
                }
            }
            CommonDragPlane::NoSharedPlane => CommonDragPlane::NoSharedPlane,
        }
    }

    fn shared_normal(&self) -> Option<Vec3> {
        if let CommonDragPlane::WithNormal(normal) = self {
            Some(*normal)
        } else {
            None
        }
    }
}

fn vpeol_3d_edit_transform_group(
    mut ui: ResMut<YoleckUi>,
    position_edit: YoleckEdit<(Entity, &mut Vpeol3dPosition, Option<&VpeolDragPlane>)>,
    rotation_edit: YoleckEdit<&mut Vpeol3dRotation>,
    scale_edit: YoleckEdit<&mut Vpeol3dScale>,
    global_drag_plane: Res<VpeolDragPlane>,
    passed_data: Res<YoleckPassedData>,
) {
    let has_any = !position_edit.is_empty() || !rotation_edit.is_empty() || !scale_edit.is_empty();
    if !has_any {
        return;
    }

    ui.group(|ui| {
        ui.label(egui::RichText::new("Transform").strong());
        ui.separator();
        
        vpeol_3d_edit_position_impl(ui, position_edit, &global_drag_plane, &passed_data);
        vpeol_3d_edit_rotation_impl(ui, rotation_edit);
        vpeol_3d_edit_scale_impl(ui, scale_edit);
    });
}

fn vpeol_3d_edit_position_impl(
    ui: &mut egui::Ui,
    mut edit: YoleckEdit<(Entity, &mut Vpeol3dPosition, Option<&VpeolDragPlane>)>,
    global_drag_plane: &VpeolDragPlane,
    passed_data: &YoleckPassedData,
) {
    if edit.is_empty() || edit.has_nonmatching() {
        return;
    }
    let mut average = DVec3::ZERO;
    let mut num_entities = 0;
    let mut transition = Vec3::ZERO;

    let mut common_drag_plane = CommonDragPlane::NotDecidedYet;

    for (entity, position, drag_plane) in edit.iter_matching() {
        let VpeolDragPlane(drag_plane) = drag_plane.unwrap_or(global_drag_plane);
        common_drag_plane.consider(*drag_plane.normal);

        if let Some(pos) = passed_data.get::<Vec3>(entity) {
            transition = *pos - position.0;
        }
        average += position.0.as_dvec3();
        num_entities += 1;
    }
    average /= num_entities as f64;

    if common_drag_plane.shared_normal().is_none() {
        transition = Vec3::ZERO;
        ui.label(
            egui::RichText::new("Drag plane differs - cannot drag together")
                .color(egui::Color32::RED),
        );
    }
    ui.horizontal(|ui| {
        let mut new_average = average;

        ui.add(egui::Label::new("Position"));
        ui.add(egui::DragValue::new(&mut new_average.x).prefix("X:"));
        ui.add(egui::DragValue::new(&mut new_average.y).prefix("Y:"));
        ui.add(egui::DragValue::new(&mut new_average.z).prefix("Z:"));

        transition += (new_average - average).as_vec3();
    });

    if transition.is_finite() && transition != Vec3::ZERO {
        for (_, mut position, _) in edit.iter_matching_mut() {
            position.0 += transition;
        }
    }
}

fn vpeol_3d_edit_rotation_impl(
    ui: &mut egui::Ui,
    mut edit: YoleckEdit<&mut Vpeol3dRotation>,
) {
    if edit.is_empty() || edit.has_nonmatching() {
        return;
    }
    
    let mut average_euler = Vec3::ZERO;
    let mut num_entities = 0;

    for rotation in edit.iter_matching() {
        average_euler += rotation.0;
        num_entities += 1;
    }
    average_euler /= num_entities as f32;

    ui.horizontal(|ui| {
        let mut new_euler = average_euler;
        let mut x_deg = new_euler.x.to_degrees();
        let mut y_deg = new_euler.y.to_degrees();
        let mut z_deg = new_euler.z.to_degrees();
        
        ui.add(egui::Label::new("Rotation"));
        ui.add(egui::DragValue::new(&mut x_deg).prefix("X:").speed(1.0).suffix("°"));
        ui.add(egui::DragValue::new(&mut y_deg).prefix("Y:").speed(1.0).suffix("°"));
        ui.add(egui::DragValue::new(&mut z_deg).prefix("Z:").speed(1.0).suffix("°"));
        
        new_euler.x = x_deg.to_radians();
        new_euler.y = y_deg.to_radians();
        new_euler.z = z_deg.to_radians();
        
        let transition = new_euler - average_euler;
        
        if transition.is_finite() && transition != Vec3::ZERO {
            for mut rotation in edit.iter_matching_mut() {
                rotation.0 += transition;
            }
        }
    });
}

fn vpeol_3d_edit_scale_impl(
    ui: &mut egui::Ui,
    mut edit: YoleckEdit<&mut Vpeol3dScale>,
) {
    if edit.is_empty() || edit.has_nonmatching() {
        return;
    }
    let mut average = DVec3::ZERO;
    let mut num_entities = 0;

    for scale in edit.iter_matching() {
        average += scale.0.as_dvec3();
        num_entities += 1;
    }
    average /= num_entities as f64;

    ui.horizontal(|ui| {
        let mut new_average = average;

        ui.add(egui::Label::new("Scale"));
        ui.add(egui::DragValue::new(&mut new_average.x).prefix("X:").speed(0.01));
        ui.add(egui::DragValue::new(&mut new_average.y).prefix("Y:").speed(0.01));
        ui.add(egui::DragValue::new(&mut new_average.z).prefix("Z:").speed(0.01));

        let transition = (new_average - average).as_vec3();
        
        if transition.is_finite() && transition != Vec3::ZERO {
            for mut scale in edit.iter_matching_mut() {
                scale.0 += transition;
            }
        }
    });
}

fn vpeol_3d_init_position(
    mut egui_context: EguiContexts,
    ui: Res<YoleckUi>,
    mut edit: YoleckEdit<(&mut Vpeol3dPosition, Option<&VpeolDragPlane>)>,
    global_drag_plane: Res<VpeolDragPlane>,
    cameras_query: Query<&VpeolCameraState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
) -> YoleckExclusiveSystemDirective {
    let Ok((mut position, drag_plane)) = edit.single_mut() else {
        return YoleckExclusiveSystemDirective::Finished;
    };

    let Some(cursor_ray) = cameras_query
        .iter()
        .find_map(|camera_state| camera_state.cursor_ray)
    else {
        return YoleckExclusiveSystemDirective::Listening;
    };

    let VpeolDragPlane(drag_plane) = drag_plane.unwrap_or(global_drag_plane.as_ref());
    if let Some(distance_to_plane) =
        cursor_ray.intersect_plane(position.0, InfinitePlane3d::new(*drag_plane.normal))
    {
        position.0 = cursor_ray.get_point(distance_to_plane);
    };

    if egui_context.ctx_mut().unwrap().is_pointer_over_area() || ui.ctx().is_pointer_over_area() {
        return YoleckExclusiveSystemDirective::Listening;
    }

    if mouse_buttons.just_released(MouseButton::Left) {
        return YoleckExclusiveSystemDirective::Finished;
    }

    YoleckExclusiveSystemDirective::Listening
}

#[derive(Clone, Copy)]
struct AxisKnobData {
    axis: Vec3,
    drag_plane_normal: Dir3,
}

fn vpeol_3d_edit_axis_knobs(
    mut edit: YoleckEdit<(Entity, &GlobalTransform, &Vpeol3dPosition, Option<&Vpeol3dRotation>)>,
    knobs_config: Res<Vpeol3dKnobsConfig>,
    mut knobs: YoleckKnobs,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    #[allow(clippy::type_complexity)] mut cached_assets: Local<
        Option<(
            Handle<Mesh>,
            Handle<Mesh>,
            [Handle<StandardMaterial>; 3],
            [Handle<StandardMaterial>; 3],
        )>,
    >,
    mut directives_writer: MessageWriter<YoleckDirective>,
    cameras_query: Query<(&GlobalTransform, &VpeolCameraState)>,
) {
    if edit.is_empty() || edit.has_nonmatching() {
        return;
    }

    let (camera_position, dragged_entity) = cameras_query
        .iter()
        .next()
        .map(|(t, state)| {
            let dragged = match &state.clicks_on_objects_state {
                VpeolClicksOnObjectsState::BeingDragged { entity, .. } => Some(*entity),
                _ => None,
            };
            (t.translation(), dragged)
        })
        .unwrap_or((Vec3::ZERO, None));

    let (cone_mesh, line_mesh, materials, materials_active) = cached_assets.get_or_insert_with(|| {
        (
            mesh_assets.add(Mesh::from(Cone {
                radius: 0.5,
                height: 1.0,
            })),
            mesh_assets.add(Mesh::from(Cylinder {
                radius: 0.15,
                half_height: 0.5,
            })),
            [
                material_assets.add(StandardMaterial {
                    base_color: Color::from(css::RED),
                    unlit: true,
                    ..default()
                }),
                material_assets.add(StandardMaterial {
                    base_color: Color::from(css::GREEN),
                    unlit: true,
                    ..default()
                }),
                material_assets.add(StandardMaterial {
                    base_color: Color::from(css::BLUE),
                    unlit: true,
                    ..default()
                }),
            ],
            [
                material_assets.add(StandardMaterial {
                    base_color: Color::linear_rgb(1.0, 0.5, 0.5),
                    unlit: true,
                    ..default()
                }),
                material_assets.add(StandardMaterial {
                    base_color: Color::linear_rgb(0.5, 1.0, 0.5),
                    unlit: true,
                    ..default()
                }),
                material_assets.add(StandardMaterial {
                    base_color: Color::linear_rgb(0.5, 0.5, 1.0),
                    unlit: true,
                    ..default()
                }),
            ],
        )
    });

    let world_axes = [
        AxisKnobData {
            axis: Vec3::X,
            drag_plane_normal: Dir3::Z,
        },
        AxisKnobData {
            axis: Vec3::Y,
            drag_plane_normal: Dir3::X,
        },
        AxisKnobData {
            axis: Vec3::Z,
            drag_plane_normal: Dir3::Y,
        },
    ];

    for (entity, global_transform, _, rotation) in edit.iter_matching() {
        let entity_position = global_transform.translation();
        let entity_scale = global_transform.to_scale_rotation_translation().0;
        let entity_radius = entity_scale.max_element();

        let distance_to_camera = (camera_position - entity_position).length();
        let distance_scale = (distance_to_camera / 40.0).max(1.0);

        let axes = match knobs_config.mode {
            Vpeol3dKnobsMode::World => world_axes,
            Vpeol3dKnobsMode::Local => {
                let rot = if let Some(Vpeol3dRotation(euler_angles)) = rotation {
                    Quat::from_euler(EulerRot::XYZ, euler_angles.x, euler_angles.y, euler_angles.z)
                } else {
                    Quat::IDENTITY
                };
                
                let local_x = (rot * Vec3::X).normalize();
                let local_y = (rot * Vec3::Y).normalize();
                let local_z = (rot * Vec3::Z).normalize();
                
                [
                    AxisKnobData {
                        axis: local_x,
                        drag_plane_normal: Dir3::new_unchecked(local_z),
                    },
                    AxisKnobData {
                        axis: local_y,
                        drag_plane_normal: Dir3::new_unchecked(local_x),
                    },
                    AxisKnobData {
                        axis: local_z,
                        drag_plane_normal: Dir3::new_unchecked(local_y),
                    },
                ]
            }
        };

        for (axis_idx, axis_data) in axes.iter().enumerate() {
            let knob_name = match axis_idx {
                0 => "vpeol-3d-axis-knob-x",
                1 => "vpeol-3d-axis-knob-y",
                _ => "vpeol-3d-axis-knob-z",
            };

            let line_name = match axis_idx {
                0 => "vpeol-3d-axis-line-x",
                1 => "vpeol-3d-axis-line-y",
                _ => "vpeol-3d-axis-line-z",
            };

            let scaled_knob_scale = knobs_config.knob_scale * distance_scale;
            let base_distance = knobs_config.knob_distance + entity_radius;
            let scaled_distance = base_distance * (1.0 + (distance_scale - 1.0) * 0.3);

            let knob_offset = scaled_distance * axis_data.axis;
            let knob_position = entity_position + knob_offset;
            let knob_transform = Transform {
                translation: knob_position,
                rotation: Quat::from_rotation_arc(Vec3::Y, axis_data.axis),
                scale: scaled_knob_scale * Vec3::ONE,
            };

            let line_length = scaled_distance - scaled_knob_scale * 0.5;
            let line_center = entity_position + axis_data.axis * line_length * 0.5;
            let line_transform = Transform {
                translation: line_center,
                rotation: Quat::from_rotation_arc(Vec3::Y, axis_data.axis),
                scale: Vec3::new(scaled_knob_scale, line_length, scaled_knob_scale),
            };

            let line_knob = knobs.knob((entity, line_name));
            let line_knob_id = line_knob.cmd.id();
            drop(line_knob);
            
            let knob = knobs.knob((entity, knob_name));
            let knob_id = knob.cmd.id();
            let passed_pos = knob.get_passed_data::<Vec3>().copied();
            drop(knob);
            
            let is_active = dragged_entity == Some(line_knob_id) || dragged_entity == Some(knob_id);
            
            let material = if is_active {
                &materials_active[axis_idx]
            } else {
                &materials[axis_idx]
            };

            let mut line_knob = knobs.knob((entity, line_name));
            line_knob.cmd.insert((
                Mesh3d(line_mesh.clone()),
                MeshMaterial3d(material.clone()),
                line_transform,
                GlobalTransform::from(line_transform),
            ));

            let mut knob = knobs.knob((entity, knob_name));
            knob.cmd.insert(VpeolDragPlane(InfinitePlane3d {
                normal: axis_data.drag_plane_normal,
            }));
            knob.cmd.insert((
                Mesh3d(cone_mesh.clone()),
                MeshMaterial3d(material.clone()),
                knob_transform,
                GlobalTransform::from(knob_transform),
            ));

            if let Some(pos) = passed_pos {
                let vector_from_entity = pos - knob_offset - entity_position;
                let along_axis = vector_from_entity.dot(axis_data.axis);
                let new_position = entity_position + along_axis * axis_data.axis;
                directives_writer.write(YoleckDirective::pass_to_entity(entity, new_position));
            }
        }
    }
}

fn vpeol_3d_populate_transform(
    mut populate: YoleckPopulate<(
        &Vpeol3dPosition,
        Option<&Vpeol3dRotation>,
        Option<&Vpeol3dScale>,
        &YoleckBelongsToLevel,
    )>,
    levels_query: Query<&VpeolRepositionLevel>,
) {
    populate.populate(
        |_ctx, mut cmd, (position, rotation, scale, belongs_to_level)| {
            let mut transform = Transform::from_translation(position.0);
            if let Some(Vpeol3dRotation(euler_angles)) = rotation {
                let quat = Quat::from_euler(EulerRot::XYZ, euler_angles.x, euler_angles.y, euler_angles.z);
                transform = transform.with_rotation(quat);
            }
            if let Some(Vpeol3dScale(scale)) = scale {
                transform = transform.with_scale(*scale);
            }

            if let Ok(VpeolRepositionLevel(level_transform)) =
                levels_query.get(belongs_to_level.level)
            {
                transform = *level_transform * transform;
            }

            cmd.insert((transform, GlobalTransform::from(transform)));
        },
    )
}
