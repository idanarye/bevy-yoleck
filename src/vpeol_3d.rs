use crate::bevy_egui::egui;
use crate::vpeol::{
    handle_clickable_children_system, VpeolBasePlugin, VpeolCameraState, VpeolDragPlane,
    VpeolRootResolver, VpeolSystemSet,
};
use crate::{prelude::*, YoleckDirective, YoleckPopulateBaseSet};
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
        app.add_yoleck_edit_system(vpeol_3d_edit_third_axis_with_knob);
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
        } else {
            return None;
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

            camera_state.consider(
                root_resolver.resolve_root(entity),
                -distance_to_aabb,
                || cursor_ray.get_point(distance_to_aabb),
            );

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

#[derive(Component)]
pub struct Vpeol3dThirdAxisWithKnob {
    pub knob_distance: f32,
    pub knob_scale: f32,
}

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

fn vpeol_3d_edit_third_axis_with_knob(
    mut edit: YoleckEdit<(
        Entity,
        &GlobalTransform,
        &Vpeol3dThirdAxisWithKnob,
        Option<&VpeolDragPlane>,
    )>,
    global_drag_plane: Res<VpeolDragPlane>,
    mut knobs: YoleckKnobs,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut mesh_and_material: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
    mut directives_writer: EventWriter<YoleckDirective>,
) {
    let Ok((entity, global_transform, third_axis_with_knob, drag_plane)) = edit.get_single_mut() else { return };

    let (mesh, material) = mesh_and_material.get_or_insert_with(|| {
        (
            mesh_assets.add(Mesh::from(shape::Cylinder {
                radius: 0.5,
                height: 1.0,
                resolution: 10,
                segments: 10,
            })),
            material_assets.add(Color::ORANGE_RED.into()),
        )
    });

    let drag_plane = drag_plane.unwrap_or(global_drag_plane.as_ref());
    let entity_position = global_transform.translation();

    for (knob_name, drag_plane_normal) in [
        ("vpeol-3d-third-axis-knob-positive", drag_plane.normal),
        ("vpeol-3d-third-axis-knob-negative", -drag_plane.normal),
    ] {
        let mut knob = knobs.knob(knob_name);
        let knob_offset = third_axis_with_knob.knob_distance * drag_plane_normal;
        let knob_transform = Transform {
            translation: entity_position + knob_offset,
            rotation: Quat::from_rotation_arc(Vec3::Y, drag_plane_normal),
            scale: third_axis_with_knob.knob_scale * Vec3::ONE,
        };
        knob.cmd.insert(VpeolDragPlane {
            normal: {
                let normal = drag_plane.normal.cross(Vec3::X).normalize_or_zero();
                if normal == Vec3::ZERO {
                    Vec3::Y
                } else {
                    normal
                }
            },
        });
        knob.cmd.insert(PbrBundle {
            mesh: mesh.clone(),
            material: material.clone(),
            transform: knob_transform,
            global_transform: knob_transform.into(),
            ..Default::default()
        });
        if let Some(pos) = knob.get_passed_data::<Vec3>() {
            let vector_from_entity = *pos - knob_offset - entity_position;
            let along_drag_normal = vector_from_entity.dot(drag_plane.normal);
            let vector_along_drag_normal = along_drag_normal * drag_plane.normal;
            let position_along_drag_normal = entity_position + vector_along_drag_normal;
            directives_writer.send(YoleckDirective::pass_to_entity(
                entity,
                position_along_drag_normal,
            ));
        }
    }
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
