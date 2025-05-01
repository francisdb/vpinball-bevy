use crate::picking::on_click_print_name;
use crate::triangulate::{ensure_ccw_winding, triangulate_polygon};
use bevy::asset::{Assets, Handle, RenderAssetUsages};
use bevy::log::{info, warn};
use bevy::math::Vec2;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::{Commands, Mesh, Mesh3d, Name, ResMut, Transform};
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::view::NoFrustumCulling;
use std::collections::HashMap;
use vpin::vpx::gameitem::wall::Wall;
use vpin::vpx::vpu_to_m;

pub(crate) fn spawn_wall(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    material_map: &HashMap<String, Handle<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    wall: &Wall,
) {
    info!("Spawning wall {}", wall.name);

    // TODO apply CatmullCurve if the dragpoints are set to smooth
    //   how is that curve then sampled for the mesh?

    // for now we just spawn the top and not the walls. There is no bottom face.

    // A wall defines a polygon in 2D space and a bottom and top height
    // These are for example used for the plastics on top of the table features

    // https://bevyengine.org/examples/3d-rendering/generate-custom-mesh/

    // wall contains dragpoints. These define a polygon in 2D space
    // TODO we need to create a mesh from the wall dragpoints

    // first we need to check if the wall.drag_points are in the right order, otherwise we need to reverse them

    // print all drag points
    for (i, drag_point) in wall.drag_points.iter().enumerate() {
        info!("  drag point {}: {:?}", i, drag_point);
    }

    let mut drag_points = wall.drag_points.clone();

    let reversed = ensure_ccw_winding(&mut drag_points);
    if reversed {
        info!("  drag points were in clockwise order, reversed");
    } else {
        info!("  drag points were in counterclockwise order");
    }

    // then we need to create a mesh from the dragpoints

    // Create a mesh builder
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    let top_height = vpu_to_m(wall.height_top);
    let bottom_height = vpu_to_m(wall.height_bottom);

    // Generate vertices for top face (all with the same height)
    let num_points = drag_points.len();
    let mut positions = Vec::with_capacity(num_points);
    let mut normals = Vec::with_capacity(num_points);
    let mut uvs = Vec::with_capacity(num_points);

    for point in &drag_points {
        // Position (x, top_height, y) -> Bevy uses y-up
        positions.push([vpu_to_m(point.x), top_height, vpu_to_m(point.y)]);

        // Normal points up for the top face
        normals.push([0.0, 1.0, 0.0]);

        // Simple UV mapping (could be improved)
        uvs.push([point.x, point.y]);
    }

    // Triangulate the polygon using ear clipping (works for any polygon)
    let positions_2d: Vec<Vec2> = positions
        .iter()
        .map(|p| Vec2::new(p[0], p[2])) // Use x,z as 2D coordinates
        .collect();

    let mut indices = triangulate_polygon(&positions_2d);
    indices.reverse();
    // info!("  indices: {:?}", indices);

    // TODO we need to create the side walls
    //   * we can't use Extrusion as that requires a Primitive2d

    // Insert attributes
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices.iter().map(|&i| i as u32).collect()));

    let material_handle = material_map
        .get(wall.top_material.as_str())
        .unwrap_or_else(|| {
            warn!(
                "Material for wall {} not found: {}. Using default material",
                wall.name, wall.top_material
            );
            &material_map["default"]
        });

    let mesh_handle = meshes.add(mesh);

    commands
        .spawn((
            Name::new(format!("Wall_{}", wall.name)),
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle.clone()),
            Transform::default(),
            NoFrustumCulling,
        ))
        .observe(on_click_print_name);
}
