use crate::bevy_egui::egui;
use crate::vpeol::{
    handle_clickable_children_system, VpeolBasePlugin, VpeolCameraState, VpeolDragPlane,
    VpeolRootResolver, VpeolSystemSet,
};
use crate::{prelude::*, YoleckPopulateBaseSet};
use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::render::view::VisibleEntities;
use serde::{Deserialize, Serialize};

/// Add the systems required for loading levels that use vpeol_3d components
pub struct Vpeol3dPluginForGame;

impl Plugin for Vpeol3dPluginForGame {
    fn build(&self, app: &mut App) {
        app.yoleck_populate_schedule_mut().add_system(
            vpeol_3d_populate_transform
                .in_base_set(YoleckPopulateBaseSet::OverrideCommonComponents),
        );
    }
}

/// Add the systems required for 3D editing.
///
/// * Entity selection.
/// * Entity dragging.
/// * Connecting nested entities.
pub struct Vpeol3dPluginForEditor {
    pub drag_plane_normal: Vec3,
}

impl Vpeol3dPluginForEditor {
    pub fn sidescroller() -> Self {
        Self {
            drag_plane_normal: Vec3::Z,
        }
    }

    pub fn topdown() -> Self {
        Self {
            drag_plane_normal: Vec3::Y,
        }
    }
}

impl Plugin for Vpeol3dPluginForEditor {
    fn build(&self, app: &mut App) {
        app.add_plugin(VpeolBasePlugin);
        app.add_plugin(Vpeol3dPluginForGame);
        app.insert_resource(VpeolDragPlane {
            normal: self.drag_plane_normal,
        });

        app.add_systems(
            (update_camera_status_for_models,).in_set(VpeolSystemSet::UpdateCameraState),
        );
        //app.add_systems(
        //(camera_3d_pan, camera_3d_zoom).in_set(OnUpdate(YoleckEditorState::EditorActive)),
        //);
        app.add_systems(
            (
                apply_system_buffers,
                handle_clickable_children_system::<With<Handle<Mesh>>, ()>,
                apply_system_buffers,
            )
                .chain()
                .in_set(OnUpdate(YoleckEditorState::EditorActive)),
        );
        app.add_yoleck_edit_system(vpeol_3d_edit_position);
    }
}

fn ray_intersection_with_aabb(ray: Ray, aabb: Aabb) -> Option<f32> {
    let center: Vec3 = aabb.center.into();
    let mut max_low = f32::NEG_INFINITY;
    let mut min_high = f32::INFINITY;
    for (axis, half_extent) in [
        (Vec3::X, aabb.half_extents.x),
        (Vec3::Y, aabb.half_extents.y),
        (Vec3::Z, aabb.half_extents.z),
    ] {
        let low = ray.intersect_plane(center - half_extent * axis, axis);
        let high = ray.intersect_plane(center + half_extent * axis, axis);
        let (low, high) = if 0.0 <= ray.direction.dot(axis) {
            (low, high)
        } else {
            (high, low)
        };
        if let Some(low) = low {
            max_low = max_low.max(low);
        }
        if let Some(high) = high {
            min_high = min_high.min(high);
        }
    }
    if max_low <= min_high {
        Some(max_low)
    } else {
        None
    }
}

fn update_camera_status_for_models(
    mut cameras_query: Query<(&mut VpeolCameraState, &VisibleEntities)>,
    entities_query: Query<(Entity, &GlobalTransform, &Handle<Mesh>)>,
    mesh_assets: Res<Assets<Mesh>>,
    root_resolver: VpeolRootResolver,
) {
    for (mut camera_state, visible_entities) in cameras_query.iter_mut() {
        let Some(cursor_ray) = camera_state.cursor_ray else { continue };
        for (entity, global_transform, mesh) in entities_query.iter_many(&visible_entities.entities)
        {
            let Some(mesh) = mesh_assets.get(mesh) else { continue };
            if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
                continue;
            }
            let Some(aabb) = mesh.compute_aabb() else { continue };

            let inverse_transform = global_transform.compute_matrix().inverse();

            let ray_in_object_coords = Ray {
                origin: inverse_transform.transform_point3(cursor_ray.origin),
                direction: inverse_transform.transform_vector3(cursor_ray.direction),
            };

            let Some(distance_to_aabb) = ray_intersection_with_aabb(ray_in_object_coords, aabb) else { continue };

            camera_state.consider(root_resolver.resolve_root(entity), distance_to_aabb, || {
                cursor_ray.get_point(distance_to_aabb)
            });

            // TODO: Check the triangles for ray intersection
            /*
            let Some(indices) = mesh.indices() else { continue };
            let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { continue };
            let mut it = indices.iter();
            let mut next_triangle =
                || {
                    Some([it.next()?, it.next()?, it.next()?].map(|idx| {
                        global_transform.transform_point(Vec3::from_array(positions[idx]))
                    }))
                };
            while let Some(triangle) = next_triangle() {
                let triangle_origin = triangle[0];
                let vec1 = triangle[1] - triangle[0];
                let vec2 = triangle[2] - triangle[0];
                let triangle_normal = vec1.cross(vec2).normalize_or_zero();
                let Some(distance) = cursor_ray.intersect_plane(triangle_origin, triangle_normal) else { continue };
                let intersection = cursor_ray.get_point(distance);
                let _ = intersection;
                //info!("intersection {:?}", intersection);
            }
            */
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Component, Default, YoleckComponent)]
#[serde(transparent)]
pub struct Vpeol3dPosition(pub Vec3);

#[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
#[serde(transparent)]
pub struct Vpeol3dRotatation(pub Quat);

#[derive(Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
#[serde(transparent)]
pub struct Vpeol3dScale(pub Vec3);

impl Default for Vpeol3dScale {
    fn default() -> Self {
        Self(Vec3::ONE)
    }
}

fn vpeol_3d_edit_position(
    mut ui: ResMut<YoleckUi>,
    mut edit: YoleckEdit<(Entity, &mut Vpeol3dPosition)>,
    passed_data: Res<YoleckPassedData>,
) {
    let Ok((entity, mut position)) = edit.get_single_mut() else { return };
    if let Some(pos) = passed_data.get::<Vec3>(entity) {
        position.0 = *pos;
    }
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut position.0.x).prefix("X:"));
        ui.add(egui::DragValue::new(&mut position.0.y).prefix("Y:"));
        ui.add(egui::DragValue::new(&mut position.0.z).prefix("Z:"));
    });
}

fn vpeol_3d_populate_transform(
    mut populate: YoleckPopulate<(
        &Vpeol3dPosition,
        Option<&Vpeol3dRotatation>,
        Option<&Vpeol3dScale>,
    )>,
) {
    populate.populate(|_ctx, mut cmd, (position, rotation, scale)| {
        let mut transform = Transform::from_translation(position.0);
        if let Some(Vpeol3dRotatation(rotation)) = rotation {
            transform = transform.with_rotation(*rotation);
        }
        if let Some(Vpeol3dScale(scale)) = scale {
            transform = transform.with_scale(*scale);
        }
        cmd.insert(TransformBundle {
            local: transform,
            global: transform.into(),
        });
    })
}
