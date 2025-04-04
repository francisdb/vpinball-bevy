mod camera;
mod gizmos;
//mod surface_mesh_generator;

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
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    // WARN this is a native only feature. It will not work with webgl or webgpu
                    features: WgpuFeatures::POLYGON_MODE_LINE,
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

    spawn_game_items(&mut commands, &mut meshes, &table, &material_map);

    spawn_overhead_lights(&mut commands, table);
}

fn spawn_game_items(
    mut commands: &mut Commands,
    mut meshes: &mut ResMut<Assets<Mesh>>,
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
            GameItemEnum::Wall(wall) => spawn_wall(wall),
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
            shadows_enabled: true,

            shadow_depth_bias: 0.0,
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

/// Draws a 2x2 reference plane in the X-Z plane but 1mm below the table to avoid z-fighting
/// TODO make this a gizmo reference grid
fn spawn_reference_plane(
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
        Name::new("ReferenceGrid".to_string()),
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
    let overhead_lights_range = overhead_lights_height + 2.0;
    // In lumens
    let overhead_lights_intensity = 50_000.0;
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

fn spawn_wall(_wall: &Wall) {
    //info!("Spawning wall {}", wall.name);
    // TODO we have to create a mesh for the wall
    // A wall defines a polygon in 2D space and a bottom and top height
    // These are for example used for the plastics on top of the table features

    // https://bevyengine.org/examples/3d-rendering/generate-custom-mesh/
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
