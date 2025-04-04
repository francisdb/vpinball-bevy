mod camera;
mod gizmos;

use crate::camera::RotatingCameraPlugin;
use crate::gizmos::ControlGizmoPlugin;
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
        })
        // Increase the shadow map resolution
        // .insert_resource(DirectionalLightShadowMap { size: 4096 })
        // .insert_resource(PointLightShadowMap { size: 4096 })
        .add_plugins((
            DefaultPlugins
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        // WARN this is a native only feature. It will not work with webgl or webgpu
                        features: WgpuFeatures::POLYGON_MODE_LINE,
                        ..default()
                    }),
                    ..default()
                })
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
            WireframePlugin,
        ))
        // Wireframes can be configured with this resource. This can be changed at runtime.
        .insert_resource(WireframeConfig {
            // The global wireframe config enables drawing of wireframes on every mesh,
            // except those with `NoWireframe`. Meshes with `Wireframe` will always have a wireframe,
            // regardless of the global configuration.
            // FIXME when setting this to true our meshes are not rendered
            //   same when adding the Wireframe component to the mesh
            global: false,
            // Controls the default color of all wireframes. Used as the default color for global wireframes.
            // Can be changed per mesh using the `WireframeColor` component.
            default_color: WHITE.into(),
        })
        .add_plugins(HelloPlugin)
        .add_plugins(RotatingCameraPlugin)
        .add_plugins(ControlGizmoPlugin)
        .add_plugins(WorldInspectorPlugin::new())
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

fn spawn_light(commands: &mut Commands, light: &Light) {
    info!("Spawning light: {}", light.name);
    let color = Color::srgb_u8(light.color.r, light.color.g, light.color.b);
    let position = Vec3::new(
        vpu_to_m(light.center.x),
        vpu_to_m(light.height.unwrap_or(0.0)),
        vpu_to_m(light.center.y),
    );
    // TODO why do light sometimes have a .looking_at(Vec3::ZERO, Dir3::Y) on the transform?
    let transform = Transform::from_translation(position);

    // Some lights have a mesh, like the lights near the slingshots
    // TODO can we show these kinds of lights in bevy?

    let visibility = match light.visible {
        Some(false) => Visibility::Hidden,
        _ => Visibility::Visible,
    };

    commands.spawn((
        Name::new(light.name.to_string()),
        PointLight {
            // FIXME enabling shadows slows down the rendering
            // shadows_enabled: true,
            // shadow_depth_bias: 0.0,
            range: vpu_to_m(light.falloff_radius),
            intensity: light.intensity,
            color,
            ..default()
        },
        visibility,
        transform,
    ));
}

/// Chrome ball at the center of the table (1.0625 inches standard pinball size)
/// default vpinball ball diameter is 50 VPU
fn spawn_ball(
    mem_dir: ResMut<MemoryDir>,
    asset_server: Res<AssetServer>,
    commands: &mut Commands,
    mut meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    table: &Res<TableResource>,
) {
    let table_center = Vec3::new(table.table_width_m / 2.0, table.table_height_m / 2.0, 0.0);

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
        Name::new("ChromeBall".to_string()),
        Mesh3d(meshes.add(Sphere::new(ball_radius_m))),
        MeshMaterial3d(chrome_material),
        Transform::from_xyz(table_center.x, ball_radius_m, table_center.y),
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
    let overhead_lights_range = overhead_lights_height + 2.0;
    // In lumens
    let overhead_lights_intensity = 500_000.0;
    info!(
        "Placing 2 overhead lights at height {:?}m and range {}m",
        overhead_lights_height, overhead_lights_range
    );
    let color = Color::srgb_u8(
        table.vpx.gamedata.light_ambient.r,
        table.vpx.gamedata.light_ambient.g,
        table.vpx.gamedata.light_ambient.b,
    );

    let overhead_light_1_pos = Vec3::new(
        vpu_to_m(table.vpx.gamedata.right * 0.5),
        overhead_lights_height,
        vpu_to_m(table.vpx.gamedata.bottom * (1.0 / 3.0)),
    );
    commands.spawn((
        Name::new("Overhead light back".to_string()),
        PointLight {
            color,
            shadows_enabled: true,
            // without this we have no shadows for small objects
            shadow_depth_bias: 0.0,
            range: overhead_lights_range,
            intensity: overhead_lights_intensity,
            ..default()
        },
        Transform::from_translation(overhead_light_1_pos), //.looking_at(Vec3::ZERO, Dir3::Y),
    ));

    let overhead_light_2_pos = Vec3::new(
        vpu_to_m(table.vpx.gamedata.right * 0.5),
        overhead_lights_height,
        vpu_to_m(table.vpx.gamedata.bottom * (2.0 / 3.0)),
    );
    commands.spawn((
        Name::new("Overhead light front".to_string()),
        PointLight {
            color,
            shadows_enabled: true,
            // without this we have no shadows for small objects
            shadow_depth_bias: 0.0,
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

    commands.spawn((
        Name::new(format!("Wall_{}", wall.name)),
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle.clone()),
        Transform::default(),
        NoFrustumCulling,
    ));
}

/// Primitives are stored with inverted z axis
fn spawn_primitive(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material_map: &HashMap<String, Handle<StandardMaterial>>,
    primitive: &Primitive,
    table: &VPX,
) {
    info!("Spawning primitive {}", primitive.name);
    let mesh = primitive.read_mesh().unwrap();
    if let Some(ReadMesh { vertices, indices }) = mesh {
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
        info!("  rot: {:?}, {:?}, {:?}", rot_x, rot_y, rot_z);
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

        // apply world rotation to the mesh
        let rotation_transform = Transform::from_rotation(
            Quat::from_rotation_x(-rot_x.to_radians())
                * Quat::from_rotation_y(-rot_z.to_radians())
                * Quat::from_rotation_z(rot_y.to_radians()),
        );

        // apply world translation to the mesh
        let translation_transform = Transform::from_translation(Vec3::new(
            vpu_to_m(position.x),
            vpu_to_m(position.z),
            vpu_to_m(position.y),
        ));

        let transform = translation_transform * rotation_transform;

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

        let visibility = match primitive.is_visible {
            true => Visibility::Visible,
            false => Visibility::Hidden,
        };

        commands.spawn((
            Name::new(primitive.name.to_string()),
            Mesh3d(meshes.add(bevy_mesh)),
            MeshMaterial3d(material_handle.clone()),
            transform,
            visibility, //Wireframe,
        ));
    } else {
        info!("No mesh found for primitive {}", primitive.name);
    }
}

/// Determines if the points are in the correct winding order (counterclockwise)
/// and reverses them if not.
fn ensure_ccw_winding(drag_points: &mut Vec<DragPoint>) -> bool {
    if drag_points.len() < 3 {
        return false; // Not enough points to form a proper polygon
    }

    // Calculate the signed area of the polygon
    // Positive area means counterclockwise winding
    // Negative area means clockwise winding
    let mut signed_area = 0.0;

    for i in 0..drag_points.len() {
        let j = (i + 1) % drag_points.len();
        signed_area +=
            (drag_points[i].x * drag_points[j].y) - (drag_points[j].x * drag_points[i].y);
    }

    // If signed area is negative, we have clockwise winding
    // We need to reverse the points to get counterclockwise winding
    if signed_area < 0.0 {
        drag_points.reverse();
        return true; // Points were reversed
    }

    false // Points were already in correct order
}

fn triangulate_polygon(vertices: &[Vec2]) -> Vec<usize> {
    if vertices.len() < 3 {
        return vec![];
    }

    // Initialize the list of indices
    let mut indices = Vec::new();

    // Create a mutable array of available vertices
    let mut remaining: Vec<usize> = (0..vertices.len()).collect();

    // Continue until we've used all vertices except the last 2
    let mut attempts = 0;
    let max_attempts = vertices.len() * vertices.len(); // Safety limit

    while remaining.len() > 2 && attempts < max_attempts {
        let n = remaining.len();

        // Find an ear
        for i in 0..n {
            let prev = (i + n - 1) % n;
            let curr = i;
            let next = (i + 1) % n;

            let prev_idx = remaining[prev];
            let curr_idx = remaining[curr];
            let next_idx = remaining[next];

            let p0 = vertices[prev_idx];
            let p1 = vertices[curr_idx];
            let p2 = vertices[next_idx];

            // Check if vertex forms an ear (internal angle < 180°)
            if is_ear(vertices, &remaining, prev, curr, next) {
                // Add triangle
                indices.push(prev_idx as u32);
                indices.push(curr_idx as u32);
                indices.push(next_idx as u32);

                // Remove the ear tip from remaining vertices
                remaining.remove(curr);
                break;
            }
        }

        attempts += 1;
    }

    // Convert to the expected format
    indices.iter().map(|&i| i as usize).collect()
}

fn is_ear(vertices: &[Vec2], remaining: &[usize], prev: usize, curr: usize, next: usize) -> bool {
    let prev_idx = remaining[prev];
    let curr_idx = remaining[curr];
    let next_idx = remaining[next];

    let p0 = vertices[prev_idx];
    let p1 = vertices[curr_idx];
    let p2 = vertices[next_idx];

    // First, check if this is a convex corner
    if !is_convex(p0, p1, p2) {
        return false;
    }

    // Then check if any remaining vertex is inside this triangle
    for &i in remaining {
        if i == prev_idx || i == curr_idx || i == next_idx {
            continue;
        }

        if point_in_triangle(vertices[i], p0, p1, p2) {
            return false;
        }
    }

    true
}

fn is_convex(p0: Vec2, p1: Vec2, p2: Vec2) -> bool {
    // Calculate the cross product to determine convexity
    let v1 = Vec2::new(p1.x - p0.x, p1.y - p0.y);
    let v2 = Vec2::new(p2.x - p1.x, p2.y - p1.y);
    let cross = v1.x * v2.y - v1.y * v2.x;

    // Positive cross product means counter-clockwise, which is what we want
    cross > 0.0
}

fn point_in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    // Barycentric coordinate method
    let area = 0.5 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y)).abs();

    // Calculate areas of three triangles made by point p and vertices of the triangle
    let alpha = 0.5 * ((b.y - c.y) * (p.x - c.x) + (c.x - b.x) * (p.y - c.y)) / area;
    let beta = 0.5 * ((c.y - a.y) * (p.x - c.x) + (a.x - c.x) * (p.y - c.y)) / area;
    let gamma = 1.0 - alpha - beta;

    // If all coordinates are between 0 and 1, point is inside triangle
    alpha >= 0.0 && beta >= 0.0 && gamma >= 0.0
}
