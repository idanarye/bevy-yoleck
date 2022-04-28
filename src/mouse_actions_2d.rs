use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::utils::HashMap;
use bevy_egui::EguiContext;

use crate::{YoleckDirective, YoleckEditorState, YoleckState};

pub struct YoleckMouseActions2dPlugin;

impl Plugin for YoleckMouseActions2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(YoleckEditorState::EditorActive)
                .with_system(yoleck_clicks_on_objects),
        );
    }
}

enum YoleckClicksOnObjectsState {
    Empty,
    PendingSelection(Entity),
    PendingMidair {
        orig_screen_pos: Vec2,
        #[allow(dead_code)]
        world: Vec2,
    },
    BeingDragged {
        entity: Entity,
        prev_screen_pos: Vec2,
        offset: Vec2,
    },
}

#[allow(clippy::too_many_arguments)]
fn yoleck_clicks_on_objects(
    mut egui_context: ResMut<EguiContext>,
    windows: Res<Windows>,
    buttons: Res<Input<MouseButton>>,
    cameras_query: Query<(Entity, &GlobalTransform, &Camera), With<OrthographicProjection>>,
    yolek_targets_query: Query<(Entity, &GlobalTransform, &YoleckSelectable)>,
    mut yoleck: ResMut<YoleckState>,
    mut state_by_camera: Local<HashMap<Entity, YoleckClicksOnObjectsState>>,
    mut directives_writer: EventWriter<YoleckDirective>,
) {
    enum MouseButtonOp {
        JustPressed,
        BeingPressed,
        JustReleased,
    }

    let mouse_button_op = if buttons.just_pressed(MouseButton::Left) {
        if egui_context.ctx_mut().is_pointer_over_area() {
            return;
        }
        MouseButtonOp::JustPressed
    } else if buttons.just_released(MouseButton::Left) {
        MouseButtonOp::JustReleased
    } else if buttons.pressed(MouseButton::Left) {
        MouseButtonOp::BeingPressed
    } else {
        state_by_camera.clear();
        return;
    };

    for (camera_entity, camera_transform, camera) in cameras_query.iter() {
        let window = if let RenderTarget::Window(window_id) = camera.target {
            windows.get(window_id).unwrap()
        } else {
            continue;
        };
        if let Some(screen_pos) = window.cursor_position() {
            let world_pos = screen_pos_to_world_pos(screen_pos, window, camera_transform, camera);

            let state = state_by_camera
                .entry(camera_entity)
                .or_insert(YoleckClicksOnObjectsState::Empty);

            let is_entity_still_pointed_at = |entity: Entity| {
                if let Ok((_, entity_transform, entity_selectable)) =
                    yolek_targets_query.get(entity)
                {
                    if entity_selectable.is_world_pos_in(entity_transform, world_pos) {
                        Some(entity_transform)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            match (&mouse_button_op, &state) {
                (MouseButtonOp::JustPressed, YoleckClicksOnObjectsState::Empty) => {
                    *state = if let Some((entity, entity_transform)) = yoleck
                        .entity_being_edited
                        .and_then(|entity| Some((entity, is_entity_still_pointed_at(entity)?)))
                    {
                        YoleckClicksOnObjectsState::BeingDragged {
                            entity,
                            prev_screen_pos: screen_pos,
                            offset: world_pos - entity_transform.translation.truncate(),
                        }
                    } else {
                        let entity_under_cursor = yolek_targets_query.iter().find_map(
                            |(entity, entity_transform, entity_selectable)| {
                                entity_selectable
                                    .is_world_pos_in(entity_transform, world_pos)
                                    .then(|| entity)
                            },
                        );
                        if let Some(entity) = entity_under_cursor {
                            YoleckClicksOnObjectsState::PendingSelection(entity)
                        } else {
                            YoleckClicksOnObjectsState::PendingMidair {
                                orig_screen_pos: screen_pos,
                                world: world_pos,
                            }
                        }
                    }
                }
                (
                    MouseButtonOp::JustReleased,
                    YoleckClicksOnObjectsState::PendingSelection(start_entity),
                ) => {
                    if is_entity_still_pointed_at(*start_entity).is_some() {
                        yoleck.entity_being_edited = Some(*start_entity);
                    }
                    *state = YoleckClicksOnObjectsState::Empty;
                }
                (_, YoleckClicksOnObjectsState::PendingSelection(start_entity)) => {
                    if is_entity_still_pointed_at(*start_entity).is_none() {
                        *state = YoleckClicksOnObjectsState::Empty;
                    }
                }
                (
                    MouseButtonOp::BeingPressed,
                    YoleckClicksOnObjectsState::PendingMidair {
                        orig_screen_pos,
                        world: _,
                    },
                ) => {
                    if 0.1 <= orig_screen_pos.distance_squared(screen_pos) {
                        *state = YoleckClicksOnObjectsState::Empty;
                    }
                }
                (
                    MouseButtonOp::JustReleased,
                    YoleckClicksOnObjectsState::PendingMidair {
                        orig_screen_pos,
                        world: _,
                    },
                ) => {
                    if orig_screen_pos.distance_squared(screen_pos) < 0.1 {
                        yoleck.entity_being_edited = None;
                    }
                    *state = YoleckClicksOnObjectsState::Empty;
                }
                (
                    MouseButtonOp::BeingPressed,
                    YoleckClicksOnObjectsState::BeingDragged {
                        entity,
                        prev_screen_pos,
                        offset,
                    },
                ) => {
                    if 0.1 <= prev_screen_pos.distance_squared(screen_pos) {
                        directives_writer.send(YoleckDirective::pass_to_entity(
                            *entity,
                            world_pos - *offset,
                        ));
                        *state = YoleckClicksOnObjectsState::BeingDragged {
                            entity: *entity,
                            prev_screen_pos: screen_pos,
                            offset: *offset,
                        };
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Component)]
pub struct YoleckSelectable(Rect<f32>);

impl YoleckSelectable {
    pub fn rect(width: f32, height: f32) -> Self {
        Self(Rect {
            left: -width * 0.5,
            right: width * 0.5,
            top: -height * 0.5,
            bottom: height * 0.5,
        })
    }

    fn is_world_pos_in(&self, transform: &GlobalTransform, cursor_in_world_pos: Vec2) -> bool {
        let [x, y, _] = transform
            .compute_matrix()
            .inverse()
            .project_point3(cursor_in_world_pos.extend(0.0))
            .to_array();
        self.0.left <= x && x <= self.0.right && self.0.top <= y && y <= self.0.bottom
    }
}

fn screen_pos_to_world_pos(
    screen_pos: Vec2,
    wnd: &Window,
    camera_transform: &GlobalTransform,
    camera: &Camera,
) -> Vec2 {
    // Code stolen from https://bevy-cheatbook.github.io/cookbook/cursor2world.html

    // get the size of the window
    let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

    // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
    let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

    // matrix for undoing the projection and camera transform
    let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix.inverse();

    // use it to convert ndc to world-space coordinates
    let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

    // reduce it to a 2D value
    world_pos.truncate()
}
