use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::utils::HashMap;

use crate::{YoleckEditorState, YoleckState};

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
        screen: Vec2,
        #[allow(dead_code)]
        world: Vec2,
    },
}

fn yoleck_clicks_on_objects(
    windows: Res<Windows>,
    buttons: Res<Input<MouseButton>>,
    cameras_query: Query<(Entity, &GlobalTransform, &Camera), With<OrthographicProjection>>,
    yolek_targets_query: Query<(Entity, &GlobalTransform, &YoleckSelectable)>,
    mut yoleck: ResMut<YoleckState>,
    mut state_by_camera: Local<HashMap<Entity, YoleckClicksOnObjectsState>>,
) {
    enum MouseButtonOp {
        JustPressed,
        BeingPressed,
        JustReleased,
    }

    let mouse_button_op = if buttons.just_pressed(MouseButton::Left) {
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

            let is_entity_still_pointed_at = |entity: Entity| -> bool {
                if let Ok((_, entity_transform, entity_selectable)) =
                    yolek_targets_query.get(entity)
                {
                    entity_selectable.is_world_pos_in(entity_transform, world_pos)
                } else {
                    false
                }
            };

            match (&mouse_button_op, &state) {
                (MouseButtonOp::JustPressed, YoleckClicksOnObjectsState::Empty) => {
                    let entity_under_cursor = yolek_targets_query.iter().find_map(
                        |(entity, entity_transform, entity_selectable)| {
                            entity_selectable
                                .is_world_pos_in(entity_transform, world_pos)
                                .then(|| entity)
                        },
                    );
                    if let Some(entity) = entity_under_cursor {
                        *state = YoleckClicksOnObjectsState::PendingSelection(entity);
                    } else {
                        *state = YoleckClicksOnObjectsState::PendingMidair {
                            screen: screen_pos,
                            world: world_pos,
                        };
                    }
                }
                (
                    MouseButtonOp::JustReleased,
                    YoleckClicksOnObjectsState::PendingSelection(start_entity),
                ) => {
                    if is_entity_still_pointed_at(*start_entity) {
                        yoleck.entity_being_edited = Some(*start_entity);
                    }
                    *state = YoleckClicksOnObjectsState::Empty;
                }
                (_, YoleckClicksOnObjectsState::PendingSelection(start_entity)) => {
                    if !is_entity_still_pointed_at(*start_entity) {
                        *state = YoleckClicksOnObjectsState::Empty;
                    }
                }
                (
                    MouseButtonOp::BeingPressed,
                    YoleckClicksOnObjectsState::PendingMidair { screen, world: _ },
                ) => {
                    if 0.1 <= (*screen - screen_pos).length_squared() {
                        *state = YoleckClicksOnObjectsState::Empty;
                    }
                }
                (
                    MouseButtonOp::JustReleased,
                    YoleckClicksOnObjectsState::PendingMidair { screen, world: _ },
                ) => {
                    if (*screen - screen_pos).length_squared() < 0.1 {
                        yoleck.entity_being_edited = None;
                    }
                    *state = YoleckClicksOnObjectsState::Empty;
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
        let topleft = transform.mul_vec3(Vec3::new(self.0.left, self.0.top, 0.0));
        let botright = transform.mul_vec3(Vec3::new(self.0.right, self.0.bottom, 0.0));
        topleft.x <= cursor_in_world_pos.x
            && topleft.y <= cursor_in_world_pos.y
            && cursor_in_world_pos.x <= botright.x
            && cursor_in_world_pos.y <= botright.y
    }
}

fn screen_pos_to_world_pos(
    screen_pos: Vec2,
    wnd: &Window,
    camera_transform: &GlobalTransform,
    camera: &Camera,
) -> Vec2 {
    // Taken from https://bevy-cheatbook.github.io/cookbook/cursor2world.html
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
