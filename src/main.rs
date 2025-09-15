mod camera;
mod gizmos;

mod ball;
mod lights;
mod picking;
mod primitives;
mod triangulate;
mod walls;

use crate::ball::spawn_ball;
use crate::camera::RotatingCameraPlugin;
use crate::gizmos::ControlGizmoPlugin;
use crate::lights::{ControlLightsPlugin, spawn_light, spawn_overhead_lights};
use crate::primitives::spawn_primitive;
use crate::walls::spawn_wall;
use bevy::asset::io::memory::{Dir, MemoryAssetReader};
use bevy::asset::io::{AssetSource, AssetSourceId};

use bevy::prelude::*;

use bevy::render::mesh::PrimitiveTopology;
use bevy_dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin};
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::process::ExitCode;
use vpin::vpx::gameitem::GameItemEnum;

use vpin::vpx::material::MaterialType;
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
        .insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.2)))
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
        .add_plugins(DebugPickingPlugin)
        .add_plugins(ControlLightsPlugin)
        .insert_resource(DebugPickingMode::Normal)
        // A system that cycles the debugging state when you press F3:
        .add_systems(
            PreUpdate,
            (|mut mode: ResMut<DebugPickingMode>| {
                *mode = match *mode {
                    DebugPickingMode::Disabled => DebugPickingMode::Normal,
                    DebugPickingMode::Normal => DebugPickingMode::Noisy,
                    DebugPickingMode::Noisy => DebugPickingMode::Disabled,
                };
                info!("Debug picking mode: {:?}", *mode);
            })
            .distributive_run_if(bevy::input::common_conditions::input_just_pressed(
                KeyCode::F3,
            )),
        )
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
        // if item.name() != "playfield_mesh" {
        //     continue;
        // }
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
