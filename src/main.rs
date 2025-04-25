mod camera;
mod gizmos;

mod ball;
mod lights;
mod picking;
mod triangulate;

use crate::ball::spawn_ball;
use crate::camera::RotatingCameraPlugin;
use crate::gizmos::ControlGizmoPlugin;
use crate::lights::{spawn_light, spawn_overhead_lights};
use crate::picking::on_click_print_name;
use crate::triangulate::{ensure_ccw_winding, triangulate_polygon};
use bevy::asset::RenderAssetUsages;
use bevy::asset::io::memory::{Dir, MemoryAssetReader};
use bevy::asset::io::{AssetSource, AssetSourceId};
use bevy::color::palettes::basic::WHITE;
use bevy::core_pipeline::Skybox;
use bevy::pbr::wireframe::{Wireframe, WireframeConfig, WireframePlugin};
use bevy::pbr::{DirectionalLightShadowMap, PointLightShadowMap};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::settings::{RenderCreation, WgpuFeatures, WgpuSettings};
use bevy::render::view::NoFrustumCulling;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::process::ExitCode;
use vpin::vpx::gameitem::GameItemEnum;
use vpin::vpx::gameitem::dragpoint::DragPoint;
use vpin::vpx::gameitem::light::Light;
use vpin::vpx::gameitem::primitive::{Primitive, ReadMesh};
use vpin::vpx::gameitem::wall::Wall;
use vpin::vpx::material::MaterialType;
use vpin::vpx::model::Vertex3dNoTex2;
use vpin::vpx::{VPX, vpu_to_m};

#[derive(Component)]
struct Table;

#[derive(Resource)]
struct TableResource {
    vpx: VPX,
    table_width_m: f32,
    table_height_m: f32,
}

impl TableResource {
    fn table_size(&self) -> Vec2 {
        Vec2::new(self.table_width_m, self.table_height_m)
    }
}

pub struct MainPlugin;

impl Plugin for MainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_table);
        app.add_systems(Startup, setup);
    }
}

#[derive(Resource)]
struct MemoryDir {
    dir: Dir,
}

/// Renders a Visual Pinball table in Bevy
///
/// In Visual Pinball (right-handed Z up):
/// The X axis goes from left to right (+X points right).
/// The Y axis goes from far to near (+Y points towards you).
/// the Z axis goes from top to bottom (+Z points up).
///
/// In Bevy (right-handed Y up):
/// The X axis goes from left to right (+X points right).
/// The Y axis goes from bottom to top (+Y points up).
/// The Z axis goes from far to near (+Z points towards you, out of the screen).
///
/// https://bevy-cheatbook.github.io/fundamentals/coords.html
///
fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let table_path = args.get(1).expect("Expected a table name argument");
    let path = Path::new(table_path);
    if !path.exists() {
        eprintln!("Table file not found: {:?}", path);
        return ExitCode::FAILURE;
    }
    println!("Loading table from {:?}", table_path);
    let vpx = vpin::vpx::read(path).unwrap();
    let table_width_vpu = vpx.gamedata.right - vpx.gamedata.left;
    let table_height_vpu = vpx.gamedata.bottom - vpx.gamedata.top;
    let table_width_m = vpu_to_m(table_width_vpu);
    let table_height_m = vpu_to_m(table_height_vpu);

    println!(
        "Table size: {} x {} m, {} x {} vp units",
        table_width_m, table_height_m, table_width_vpu, table_height_vpu
    );

    let memory_dir = MemoryDir {
        dir: Dir::default(),
    };
    let reader = MemoryAssetReader {
        root: memory_dir.dir.clone(),
    };
    let app_exit = App::new()
        .register_asset_source(
            AssetSourceId::from_static("memory"),
            AssetSource::build().with_reader(move || Box::new(reader.clone())),
        )
        .insert_resource(memory_dir)
        .insert_resource(TableResource {
            vpx,
            table_width_m,
            table_height_m,
        })
        // Set the background color to light gray
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
        .insert_resource(AmbientLight {
            color: Color::BLACK,
            // TODO get this from the table
            // there is a color (black in the default table)
            // and a scene lighting scale? (daynight?)
            brightness: 1.0,
            //affects_lightmapped_meshes: true,
            ..default()
        })
        // Increase the shadow map resolution
        // .insert_resource(DirectionalLightShadowMap { size: 4096 })
        // .insert_resource(PointLightShadowMap { size: 4096 })
        .add_plugins((
            DefaultPlugins
                // // This plugin is needed to enable the wireframe rendering
                // .set(RenderPlugin {
                //     render_creation: RenderCreation::Automatic(WgpuSettings {
                //         // WARN this is a native only feature. It will not work with webgl or webgpu
                //         features: WgpuFeatures::POLYGON_MODE_LINE,
                //         ..default()
                //     }),
                //     ..default()
                // })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: format!(
                            "Pinviewer - {}",
                            path.file_name().unwrap().to_string_lossy()
                        ),
                        ..default()
                    }),
                    ..default()
                }),
            // You need to add this plugin to enable wireframe rendering
            //WireframePlugin,
        ))
        // Wireframes can be configured with this resource. This can be changed at runtime.
        // .insert_resource(WireframeConfig {
        //     // The global wireframe config enables drawing of wireframes on every mesh,
        //     // except those with `NoWireframe`. Meshes with `Wireframe` will always have a wireframe,
        //     // regardless of the global configuration.
        //     // FIXME when setting this to true our meshes are not rendered
        //     //   same when adding the Wireframe component to the mesh
        //     global: false,
        //     // Controls the default color of all wireframes. Used as the default color for global wireframes.
        //     // Can be changed per mesh using the `WireframeColor` component.
        //     default_color: WHITE.into(),
        // })
        .add_plugins(MainPlugin)
        .add_plugins(RotatingCameraPlugin)
        .add_plugins(ControlGizmoPlugin)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .add_plugins(WorldInspectorPlugin::default())
        .add_plugins(MeshPickingPlugin)
        .run();
    match app_exit {
        AppExit::Success => ExitCode::SUCCESS,
        AppExit::Error(err) => {
            eprintln!("Error: {:?}", err);
            ExitCode::FAILURE
        }
    }
}

fn load_table(mut commands: Commands, table: Res<TableResource>) {
    commands.spawn((Table, Name::new(table.vpx.gamedata.name.to_string())));
}

/// set up a simple 3D scene
fn setup(
    mem_dir: ResMut<MemoryDir>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    table: Res<TableResource>,
) {
    // // circular base
    // commands.spawn((
    //     Mesh3d(meshes.add(Circle::new(4.0))),
    //     MeshMaterial3d(materials.add(Color::WHITE)),
    //     Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    // ));
    // // cube
    // let cube_width = 0.04;
    // commands.spawn((
    //     Mesh3d(meshes.add(Cuboid::new(cube_width, cube_width, cube_width))),
    //     MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
    //     Transform::from_xyz(0.0, cube_width / 2.0, 0.0),
    // ));
    // // cube
    // let cube_width = 0.06;
    // commands.spawn((
    //     Mesh3d(meshes.add(Cuboid::new(cube_width, cube_width, cube_width))),
    //     MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
    //     Transform::from_xyz(0.2, cube_width / 2.0, 0.0),
    // ));
    // // cube
    // let cube_width = 0.08;
    // commands.spawn((
    //     Mesh3d(meshes.add(Cuboid::new(cube_width, cube_width, cube_width))),
    //     MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
    //     Transform::from_xyz(0.4, cube_width / 2.0, 0.0),
    // ));
    // // cube
    // let cube_width = 0.1;
    // commands.spawn((
    //     Mesh3d(meshes.add(Cuboid::new(cube_width, cube_width, cube_width))),
    //     MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
    //     Transform::from_xyz(0.6, cube_width / 2.0, 0.0),
    // ));
    // commands.spawn((
    //     PointLight {
    //         shadows_enabled: true,
    //         shadow_depth_bias: 0.0,
    //         range: 100.0,
    //         intensity: 100000.0,
    //         ..default()
    //     },
    //     Transform::from_xyz(1.0, 2.0, 1.0),
    // ));

    //spawn_reference_plane(&mut commands, &mut meshes, &mut materials);

    if let Some(_env_image) = &table.vpx.gamedata.env_image {
        // TODO Add environment map for reflections
    }

    // TODO use textures

    // // for reference a 1 cm cube at the center of the table
    // let table_center = Vec3::new(table.table_width_m / 2.0, table.table_height_m / 2.0, 0.0);
    // let color_light_blue: Color = Color::srgb_u8(124, 144, 255);
    // commands.spawn((
    //     Mesh3d(meshes.add(Cuboid::new(0.01, 0.01, 0.01))),
    //     MeshMaterial3d(materials.add(color_light_blue)),
    //     Transform::from_xyz(table_center.x, table_center.y, table_center.z - 0.005),
    // ));

    spawn_ball(
        mem_dir,
        asset_server,
        &mut commands,
        &mut meshes,
        &mut materials,
        &table,
    );

    let material_map = create_materials(&table, &mut materials);

    spawn_game_items(
        &mut commands,
        &mut meshes,
        &mut materials,
        &table,
        &material_map,
    );

    spawn_overhead_lights(&mut commands, table);
}

fn spawn_game_items(
    mut commands: &mut Commands,
    mut meshes: &mut ResMut<Assets<Mesh>>,
    mut materials: &mut ResMut<Assets<StandardMaterial>>,
    table: &Res<TableResource>,
    material_map: &HashMap<String, Handle<StandardMaterial>>,
) {
    for item in &table.vpx.gameitems {
        match item {
            GameItemEnum::Primitive(primitive) => {
                spawn_primitive(
                    &mut commands,
                    &mut meshes,
                    &material_map,
                    primitive,
                    &table.vpx,
                );
            }
            GameItemEnum::Light(light) => {
                spawn_light(&mut commands, light);
            }
            GameItemEnum::Wall(wall) => spawn_wall(
                &mut commands,
                &mut materials,
                material_map,
                &mut meshes,
                wall,
            ),
            _other => {}
        }
    }
}

fn create_materials(
    table: &TableResource,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> HashMap<String, Handle<StandardMaterial>> {
    let mut material_map: HashMap<String, Handle<StandardMaterial>> = HashMap::new();

    // Vpinball dummy material color
    // https://github.com/vpinball/vpinball/blob/ac2108b4acd7b052e8b9fa3d7f03b02e10b5a09e/src/parts/Material.h#L47
    let default_material_color: Color = Color::srgb_u8(180, 105, 255);
    let default_material = materials.add(default_material_color);
    material_map.insert("default".to_string(), default_material);

    if let Some(vpx_materials) = &table.vpx.gamedata.materials {
        for material in vpx_materials {
            info!("Material: {:?}", material.name);
            // PBR material
            let base_color = Color::srgb_u8(
                material.base_color.r,
                material.base_color.g,
                material.base_color.b,
            );
            let metallic = match material.type_ {
                MaterialType::Metal => 1.0,
                MaterialType::Basic => 0.0,
                MaterialType::Unknown => 0.0,
            };
            info!("  roughness: {:?}", material.roughness);
            info!("  base color: {:?}", material.base_color);
            info!("  metallic: {:?}", metallic);
            info!("  thickness: {:?}", material.thickness);
            // TODO enable pbr_multi_layer_material_textures feature and set clear coat
            let material_handle = materials.add(StandardMaterial {
                base_color,
                metallic,
                thickness: material.thickness,
                //reflectance: material.glossy_image_lerp,
                perceptual_roughness: 1.0 - material.roughness,
                reflectance: 0.9,
                alpha_mode: AlphaMode::Opaque,
                ..default()
            });
            material_map.insert(material.name.clone(), material_handle);
        }
    } else {
        info!("No materials found");
    }

    material_map
}

fn spawn_wall(
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
    info!("  indices: {:?}", indices);

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

/// Spawns a 3D primitive in the Bevy engine using the provided data.
///
/// This function creates a Bevy-compatible mesh object from the given primitive data (e.g., mesh
/// vertices, normals, UV coordinates, indices), applies the appropriate transformations and
/// scaling based on its attributes, associates a material with the mesh, and spawns it in the ECS
/// (Entity Component System) world. This is intended for use within a custom visual pinball game
/// development environment.
///
/// # Parameters
/// - `commands`: A mutable reference to the Bevy `Commands` object for spawning entities.
/// - `meshes`: A mutable reference to Bevy's `Assets<Mesh>` for storing and managing mesh assets.
/// - `material_map`: A reference to a `HashMap` mapping material names to their material handles.
/// - `primitive`: A reference to the primitive to be spawned, which contains metadata like mesh
///                geometry, size, rotation, position, and material.
/// - `table`: A reference to the `VPX` structure associated with the current table, used to
///            retrieve additional metadata such as playfield material.
///
/// # Details
/// - Reads mesh data from the `primitive`, including vertices and indices.
/// - Converts raw geometry into a Bevy `Mesh` by:
///     - Mapping positions, normals, and texture coordinates (UVs) to Bevy's attributes.
///     - Converting vertex and index formats to Bevy-compatible formats.
/// - Applies scaling to convert visual pinball units (VPU) to meters.
/// - Applies rotation and translation transformations (object-level and world space).
/// - Retrieves the appropriate material from the `material_map`:
///     - Uses the material defined in `primitive.material` (if present).
///     - For the `playfield_mesh`, retrieves the material from the `table`'s playfield metadata.
///     - Falls back to a default material if the material is not found or is empty.
/// - Sets the visibility based on the `primitive.is_visible` property.
/// - Spawns the
fn spawn_primitive(
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
            RenderAssetUsages::RENDER_WORLD,
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
        ))
        .observe(on_click_print_name);
}
