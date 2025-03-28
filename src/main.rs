mod camera;
mod gizmos;

use crate::camera::RotatingCameraPlugin;
use crate::gizmos::ControlGizmoPlugin;
use bevy::asset::RenderAssetUsages;
use bevy::asset::io::memory::{Dir, MemoryAssetReader};
use bevy::asset::io::{AssetSource, AssetSourceId};
use bevy::core_pipeline::Skybox;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::Extent3d;
use bevy::utils::info;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::process::ExitCode;
use vpin::vpx::gameitem::GameItemEnum;
use vpin::vpx::gameitem::primitive::{Primitive, ReadMesh};
use vpin::vpx::gameitem::wall::Wall;
use vpin::vpx::material::MaterialType;
use vpin::vpx::model::Vertex3dNoTex2;
use vpin::vpx::{VPX, vpu_to_m};

#[derive(Component)]
struct Table;

#[derive(Component)]
struct Name(String);

#[derive(Resource)]
struct TableResource {
    vpx: VPX,
    table_width_m: f32,
    table_height_m: f32,
}

pub struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_table);
        app.add_systems(Startup, setup);
    }
}

#[derive(Resource)]
struct MemoryDir {
    dir: Dir,
}

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
            color: Color::WHITE,
            brightness: 50.0,
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(HelloPlugin)
        .add_plugins(RotatingCameraPlugin)
        .add_plugins(ControlGizmoPlugin)
        .add_systems(Startup, setup_gizmo_config)
        .run();
    match app_exit {
        AppExit::Success => ExitCode::SUCCESS,
        AppExit::Error(err) => {
            eprintln!("Error: {:?}", err);
            ExitCode::FAILURE
        }
    }
}

fn setup_gizmo_config(mut gizmo_config_store: ResMut<GizmoConfigStore>) {
    gizmo_config_store
        .config_mut::<LightGizmoConfigGroup>()
        .1
        .draw_all = true;
}

fn load_table(mut commands: Commands, table: Res<TableResource>) {
    commands.spawn((Table, Name(table.vpx.gamedata.name.to_string())));
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
    if let Some(env_image) = &table.vpx.gamedata.env_image {
        // TODO Add environment map for reflections
    }

    // TODO create vpinball lights
    // TODO use textures
    // TODO scale everything to real world units in meters
    // TODO rotate everything so that the table is almost parallel the ground

    let table_center = Vec3::new(table.table_width_m / 2.0, table.table_height_m / 2.0, 0.0);

    // // for reference a 1 cm cube at the center of the table
    // let color_light_blue: Color = Color::srgb_u8(124, 144, 255);
    // commands.spawn((
    //     Mesh3d(meshes.add(Cuboid::new(0.01, 0.01, 0.01))),
    //     MeshMaterial3d(materials.add(color_light_blue)),
    //     Transform::from_xyz(table_center.x, table_center.y, table_center.z - 0.005),
    // ));

    // Chrome ball at the center of the table (1.0625 inches standard pinball size)
    // default vpinball ball diameter is 50 VPU

    // TODO find out what the difference between ball_image and ball_image_front is
    //   and what the audit about old ball image is.
    // This image is probably static if spherical map is disabled
    // On the table I tested it was just an environment map
    //let ball_image_name = &table.vpx.gamedata.ball_image;
    let ball_image_name = &table.vpx.gamedata.ball_image_front;

    let mut ball_image_handle: Option<Handle<Image>> = None;
    for image in &table.vpx.images {
        if image.name == *ball_image_name {
            info!("Ball image found: {:?}", image.name);
            // TODO load the ball texture
            // TODO apply the ball texture to the ball material
            if let Some(jpg) = &image.jpeg {
                // use the image data to create a texture

                let file_name = format!("{}.{}", image.name, image.ext());
                mem_dir
                    .dir
                    .insert_asset(Path::new(&file_name), jpg.data.to_owned());

                info!("Ball texture: {:?}", file_name);
                ball_image_handle = Some(asset_server.load(format!("memory://{}", file_name)));

                // TODO https://github.com/bevyengine/bevy/discussions/13602#discussioncomment-12089441
                // TODO https://www.reddit.com/r/bevy/comments/1i83wv5/tutorial_how_to_load_inmemory_assets_in_bevy/
            } else {
                info!("No image data found for ball image");
            }
        }
    }

    let chrome_material = materials.add(StandardMaterial {
        //base_color: Color::srgb(0.8, 0.8, 0.8),
        base_color: Color::WHITE,
        metallic: 1.0,
        perceptual_roughness: 0.05,
        reflectance: 0.9,
        base_color_texture: ball_image_handle,
        ..default()
    });

    let ball_radius_m = vpu_to_m(50.0 / 2.0);
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(ball_radius_m))),
        MeshMaterial3d(chrome_material),
        Transform::from_xyz(table_center.x, table_center.y, -ball_radius_m),
        Name("ChromeBall".to_string()),
    ));

    // create materials
    // new map of material name to material handle

    let material_map = create_materials(&table, &mut materials);

    // render all vpinball primitives
    for item in &table.vpx.gameitems {
        match item {
            GameItemEnum::Primitive(primitive) => {
                // make a mesh from the primitive
                let mesh = primitive.read_mesh().unwrap();
                if let Some(ReadMesh { vertices, indices }) = mesh {
                    spawn_primitive(
                        &mut commands,
                        &mut meshes,
                        &material_map,
                        primitive,
                        vertices,
                        indices,
                    );
                } else {
                    info!("No mesh found for primitive {}", primitive.name);
                }
            }
            GameItemEnum::Light(light) => {
                let color = Color::srgb_u8(light.color.r, light.color.g, light.color.b);
                let position = Vec3::new(
                    vpu_to_m(light.center.x),
                    vpu_to_m(light.center.y),
                    vpu_to_m(light.height.unwrap_or(0.0)),
                );
                let transform = Transform::from_translation(position);
                commands.spawn((
                    PointLight {
                        shadows_enabled: false,
                        range: vpu_to_m(light.falloff_radius),
                        intensity: light.intensity,
                        color,
                        ..default()
                    },
                    transform,
                ));
            }
            GameItemEnum::Wall(wall) => spawn_wall(wall),
            _other => {}
        }
    }

    // Room light
    let light_position = table_center;
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            range: 1.0,
            intensity: 1_000.0,
            ..default()
        },
        Transform::from_xyz(light_position.x, light_position.y, light_position.z - 0.5),
    ));
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
            info!("roughness: {:?}", material.roughness);
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

fn spawn_wall(wall: &Wall) {
    //info!("Spawning wall {}", wall.name);
    // TODO we have to create a mesh for the wall
    // A wall defines a polygon in 2D space and a bottom and top height
    // These are for example used for the plastics on top of the table features

    // https://bevyengine.org/examples/3d-rendering/generate-custom-mesh/
}

fn spawn_primitive(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material_map: &HashMap<String, Handle<StandardMaterial>>,
    primitive: &Primitive,
    vertices: Vec<([u8; 32], Vertex3dNoTex2)>,
    indices: Vec<i64>,
) {
    info!("Spawning primitive {}", primitive.name);

    let mut bevy_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    let positions: Vec<[f32; 3]> = vertices.iter().map(|(_, v)| [v.x, v.y, v.z]).collect();
    bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

    let normals: Vec<[f32; 3]> = vertices.iter().map(|(_, v)| [v.nx, v.ny, v.nz]).collect();
    bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

    let uvs: Vec<[f32; 2]> = vertices.iter().map(|(_, v)| [v.tu, v.tv]).collect();
    bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    let indices: Vec<u32> = indices.iter().map(|i| *i as u32).collect();
    bevy_mesh.insert_indices(Indices::U32(indices));

    let position = primitive.position;
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
    // info!("  rot: {:?}, {:?}, {:?}", rot_x, rot_y, rot_z);
    // info!("  tra: {:?}, {:?}, {:?}", tra_x, tra_y, tra_z);
    // info!(
    //     "  obj_rot: {:?}, {:?}, {:?}",
    //     obj_rot_x, obj_rot_y, obj_rot_z
    // );

    // scale to visual pinball units to meters
    bevy_mesh.scale_by(Vec3::new(vpu_to_m(1.0), vpu_to_m(1.0), vpu_to_m(1.0)));

    // apply the object rotation to the mesh
    bevy_mesh.rotate_by(
        Quat::from_rotation_x(obj_rot_x.to_radians())
            * Quat::from_rotation_y(obj_rot_y.to_radians())
            * Quat::from_rotation_z(obj_rot_z.to_radians()),
    );

    // apply user-defined scale to the mesh
    bevy_mesh.scale_by(Vec3::new(
        primitive.size.x,
        primitive.size.y,
        primitive.size.z,
    ));

    // apply world rotation to the mesh
    let rotation_transform = Transform::from_rotation(
        Quat::from_rotation_x(rot_x.to_radians())
            * Quat::from_rotation_y(rot_y.to_radians())
            * Quat::from_rotation_z(rot_z.to_radians()),
    );

    // apply world translation to the mesh
    let translation_transform = Transform::from_translation(Vec3::new(
        vpu_to_m(position.x),
        vpu_to_m(position.y),
        vpu_to_m(-position.z),
    ));

    let transform = translation_transform * rotation_transform;

    let material_name = &primitive.material;
    // TODO look up the material

    let material_handle = material_map.get(material_name).unwrap_or_else(|| {
        warn!(
            "Material for primitive {} not found: {}",
            primitive.name, material_name
        );
        &material_map["default"]
    });

    commands.spawn((
        Name(primitive.name.to_string()),
        Mesh3d(meshes.add(bevy_mesh)),
        MeshMaterial3d(material_handle.clone()),
        transform,
    ));
}
