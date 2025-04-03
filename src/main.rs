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

impl TableResource {
    fn table_size(&self) -> Vec2 {
        Vec2::new(self.table_width_m, self.table_height_m)
    }
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

/// Renders a Visual Pinball table in Bevy
///
/// In Visual Pinball (right handed Z up):
/// The X axis goes from left to right (+X points right).
/// The Y axis goes from far to near (+Y points towards you).
/// the Z axis goes from top to bottom (+Z points up).
///
/// In Bevy (right handed Y up):
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
            brightness: 2.0,
            affects_lightmapped_meshes: true,
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
    draw_reference_plane(&mut commands, &mut meshes, &mut materials);

    if let Some(_env_image) = &table.vpx.gamedata.env_image {
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

    spawn_ball(
        mem_dir,
        asset_server,
        &mut commands,
        &mut meshes,
        &mut materials,
        &table,
        table_center,
    );

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
                    vpu_to_m(light.height.unwrap_or(0.0)),
                    vpu_to_m(light.center.y),
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

    spawn_overhead_lights(&mut commands, table);
}

fn spawn_ball(
    mem_dir: ResMut<MemoryDir>,
    asset_server: Res<AssetServer>,
    commands: &mut Commands,
    mut meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    table: &Res<TableResource>,
    table_center: Vec3,
) {
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
        Name("ChromeBall".to_string()),
        Mesh3d(meshes.add(Sphere::new(ball_radius_m))),
        MeshMaterial3d(chrome_material),
        Transform::from_xyz(table_center.x, ball_radius_m, table_center.y),
    ));
}

/// Draws a 2x2 reference plane in the X-Z plane but 1mm below the table to avoid z-fighting
/// TODO make this a gizmo reference grid
fn draw_reference_plane(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let half_width = 1.0;
    let reference_plane = meshes.add(Rectangle::new(half_width * 2.0, half_width * 2.0));
    let grid_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.8, 0.8),
        metallic: 0.0,
        perceptual_roughness: 0.9,
        reflectance: 0.1,
        // Optional grid texture
        // base_color_texture: Some(asset_server.load("textures/grid.png")),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.spawn((
        Name("ReferenceGrid".to_string()),
        Mesh3d(reference_plane),
        MeshMaterial3d(grid_material),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_translation(Vec3::new(0.0, -0.001, 0.0)),
    ));
}

fn spawn_overhead_lights(commands: &mut Commands, table: Res<TableResource>) {
    // Visual Pinball defines two overhead lights positioned at 1/3 and 2/3 of the table length

    // 2 overhead lights with power, height, color and range
    //    m_table->m_Light[0].pos.x = m_table->m_right * 0.5f;
    //    m_table->m_Light[1].pos.x = m_table->m_right * 0.5f;
    //    m_table->m_Light[0].pos.y = m_table->m_bottom * (float)(1.0 / 3.0);
    //    m_table->m_Light[1].pos.y = m_table->m_bottom * (float)(2.0 / 3.0);
    //    m_table->m_Light[0].pos.z = m_table->m_lightHeight;
    //    m_table->m_Light[1].pos.z = m_table->m_lightHeight;
    //
    // LZDI
    //    vec4 emission = convertColor(m_table->m_Light[0].emission, 1.f);
    //    emission.x *= m_table->m_lightEmissionScale * m_globalEmissionScale;
    //    emission.y *= m_table->m_lightEmissionScale * m_globalEmissionScale;
    //    emission.z *= m_table->m_lightEmissionScale * m_globalEmissionScale;
    // LZRA
    //   range

    //       PropRGB("Light Em. Color", m_table, is_live, &(m_table->m_Light[0].emission), m_live_table ? &(m_live_table->m_Light[0].emission) : nullptr);
    //       PropFloat("Light Em. Scale", m_table, is_live, &(m_table->m_lightEmissionScale), m_live_table ? &(m_live_table->m_lightEmissionScale) : nullptr, 20000.0f, 100000.0f, "%.0f", ImGuiInputTextFlags_CharsDecimal, reinit_lights);
    //       PropFloat("Light Height", m_table, is_live, &(m_table->m_lightHeight), m_live_table ? &(m_live_table->m_lightHeight) : nullptr, 20.0f, 100.0f, "%.0f");
    //       PropFloat("Light Range", m_table, is_live, &(m_table->m_lightRange), m_live_table ? &(m_live_table->m_lightRange) : nullptr, 200.0f, 1000.0f, "%.0f");
    //

    let overhead_lights_height = vpu_to_m(table.vpx.gamedata.light_height);
    // TODO what units are these? vpu_to_m(table.vpx.gamedata.light_range)
    let overhead_lights_range = overhead_lights_height + 0.5;
    // In lumens
    let overhead_lights_intensity = 20_000.0;
    info!(
        "Placing 2 overhead lights at height {:?}m and range {}m",
        overhead_lights_height, overhead_lights_range
    );

    let overhead_light_1_pos = Vec3::new(
        vpu_to_m(table.vpx.gamedata.right * 0.5),
        overhead_lights_height,
        vpu_to_m(table.vpx.gamedata.bottom * (1.0 / 3.0)),
    );
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            range: overhead_lights_range,
            intensity: overhead_lights_intensity,
            ..default()
        },
        Transform::from_translation(overhead_light_1_pos),
    ));

    let overhead_light_2_pos = Vec3::new(
        vpu_to_m(table.vpx.gamedata.right * 0.5),
        overhead_lights_height,
        vpu_to_m(table.vpx.gamedata.bottom * (2.0 / 3.0)),
    );
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            range: overhead_lights_range,
            intensity: overhead_lights_intensity,
            ..default()
        },
        Transform::from_translation(overhead_light_2_pos),
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

fn spawn_wall(_wall: &Wall) {
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

    let positions: Vec<[f32; 3]> = vertices.iter().map(|(_, v)| [v.x, v.z, v.y]).collect();
    bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

    let normals: Vec<[f32; 3]> = vertices.iter().map(|(_, v)| [v.nx, v.nz, v.ny]).collect();
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
