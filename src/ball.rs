use crate::picking::on_click_print_name;
use crate::{MemoryDir, TableResource};
use bevy::asset::{AssetServer, Assets, Handle};
use bevy::color::Color;
use bevy::image::Image;
use bevy::log::info;
use bevy::math::Vec3;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::{Commands, Mesh, Mesh3d, Name, Res, ResMut, Sphere, Transform, default};
use std::path::Path;
use vpin::vpx::vpu_to_m;

/// Chrome ball at the center of the table (1.0625 inches standard pinball size)
/// default vpinball ball diameter is 50 VPU
pub(crate) fn spawn_ball(
    mem_dir: ResMut<MemoryDir>,
    asset_server: Res<AssetServer>,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
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
    commands
        .spawn((
            Name::new("ChromeBall".to_string()),
            Mesh3d(meshes.add(Sphere::new(ball_radius_m))),
            MeshMaterial3d(chrome_material),
            Transform::from_xyz(table_center.x, ball_radius_m, table_center.y),
        ))
        .observe(on_click_print_name);
}
