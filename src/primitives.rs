use crate::picking::on_click_print_name;
use bevy::asset::{Assets, Handle, RenderAssetUsages};
use bevy::log::{info, warn};
use bevy::math::{Quat, Vec3};
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::{Commands, Cylinder, Mesh, Mesh3d, Name, ResMut, Transform, Visibility};
use bevy::render::mesh::{Indices, MeshAabb, PrimitiveTopology};
use bevy_picking::Pickable;
use std::collections::HashMap;
use vpin::vpx::gameitem::primitive::{Primitive, ReadMesh};
use vpin::vpx::{VPX, vpu_to_m};

pub(crate) fn spawn_primitive(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material_map: &HashMap<String, Handle<StandardMaterial>>,
    primitive: &Primitive,
    table: &VPX,
) {
    info!("Spawning primitive {}", primitive.name);

    let material_handle = if primitive.name == "playfield_mesh" {
        if !primitive.material.is_empty() {
            warn!(
                "Expected empty primitive material for playfield_mesh but got {}",
                primitive.material
            );
        }
        if table.gamedata.playfield_material.is_empty() {
            warn!("Gamedata playfield material is empty");
        }
        // get the material from the table
        info!("Playfield material: {}", &table.gamedata.playfield_material);
        &material_map[&table.gamedata.playfield_material]
    } else {
        let material_name = &primitive.material;
        // TODO look up the material
        if material_name.is_empty() {
            warn!(
                "Material name is empty for primitive {}, using default material",
                primitive.name
            );
            &material_map["default"]
        } else {
            material_map.get(material_name).unwrap_or_else(|| {
                warn!(
                    "Material for primitive {} not found: {}. Using default material",
                    primitive.name, material_name
                );
                &material_map["default"]
            })
        }
    };

    let [
        rot_x,
        rot_y,
        rot_z,
        tra_x,
        tra_y,
        tra_z,
        obj_rot_x,
        obj_rot_y,
        obj_rot_z,
    ] = primitive.rot_and_tra;

    let mesh = primitive.read_mesh().unwrap();
    let bevy_mesh = if let Some(ReadMesh { vertices, indices }) = mesh {
        let mut bevy_mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );

        let positions: Vec<[f32; 3]> = vertices.iter().map(|(_, v)| [v.x, -v.z, v.y]).collect();
        bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

        let normals: Vec<[f32; 3]> = vertices.iter().map(|(_, v)| [v.nx, -v.nz, v.ny]).collect();
        bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

        let uvs: Vec<[f32; 2]> = vertices.iter().map(|(_, v)| [v.tu, v.tv]).collect();
        bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

        let indices: Vec<u32> = indices.iter().map(|i| *i as u32).collect();
        bevy_mesh.insert_indices(Indices::U32(indices));

        let [
            rot_x,
            rot_y,
            rot_z,
            tra_x,
            tra_y,
            tra_z,
            obj_rot_x,
            obj_rot_y,
            obj_rot_z,
        ] = primitive.rot_and_tra;

        // info!(
        //     "  Size: {:?}, {:?}, {:?}",
        //     primitive.size.x, primitive.size.y, primitive.size.z
        // );
        // info!(
        //     "  position: {:?}, {:?}, {:?}",
        //     position.x, position.y, position.z
        // );

        // info!("  tra: {:?}, {:?}, {:?}", tra_x, tra_y, tra_z);
        info!(
            "  obj_rot: {:?}, {:?}, {:?}",
            obj_rot_x, obj_rot_y, obj_rot_z
        );

        // scale to visual pinball units to meters
        bevy_mesh.scale_by(Vec3::new(vpu_to_m(1.0), vpu_to_m(1.0), vpu_to_m(1.0)));

        // apply the object rotation to the mesh
        bevy_mesh.rotate_by(
            Quat::from_rotation_x(-obj_rot_x.to_radians())
                * Quat::from_rotation_y(-obj_rot_z.to_radians())
                * Quat::from_rotation_z(obj_rot_y.to_radians()),
        );

        // apply user-defined scale to the mesh
        bevy_mesh.scale_by(Vec3::new(
            primitive.size.x,
            primitive.size.z,
            primitive.size.y,
        ));

        bevy_mesh
    } else {
        // TODO there is a property that indicates there is no mesh
        //  in that case it's a cylinder
        //  the number of sides indicates how many segments the circle has

        // we need to create a cylinder
        // TODO how do we control the number of faces?
        // TODO how do we fix the size correctly?
        let cylinder = Cylinder {
            radius: 0.01,
            half_height: 0.1,
        };
        //let cylinder = Extrusion::new(Circle { radius: 0.01 }, 0.1);
        cylinder.into()
    };

    // apply world rotation to the mesh
    info!("  rot: {:?}, {:?}, {:?}", rot_x, rot_y, rot_z);
    let rotation_transform = Transform::from_rotation(
        Quat::from_rotation_x(-rot_x.to_radians())
            * Quat::from_rotation_y(-rot_z.to_radians())
            * Quat::from_rotation_z(rot_y.to_radians()),
    );

    let position = primitive.position;

    // apply world translation to the mesh
    let translation_transform = Transform::from_translation(Vec3::new(
        vpu_to_m(position.x),
        vpu_to_m(position.z),
        vpu_to_m(position.y),
    ));

    let transform = translation_transform * rotation_transform;

    let visibility = match primitive.is_visible {
        true => Visibility::Visible,
        false => Visibility::Hidden,
    };

    commands
        .spawn((
            Name::new(primitive.name.to_string()),
            Mesh3d(meshes.add(bevy_mesh)),
            MeshMaterial3d(material_handle.clone()),
            transform,
            visibility, //Wireframe,
            Pickable::default(),
        ))
        .observe(on_click_print_name);
}
