use crate::TableResource;
use bevy::app::Plugin;
use bevy::input::mouse::MouseWheel;
use bevy::math::{Quat, Vec3};
use bevy::prelude::*;

pub(crate) struct RotatingCameraPlugin;

impl Plugin for RotatingCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera_system);
        //app.add_systems(Update, rotate_camera_system);;
        app.add_systems(Update, height_camera_system);
    }
}

fn spawn_camera_system(
    mut commands: Commands,
    table_resource: Res<TableResource>,
    asset_server: Res<AssetServer>,
    //mut images: ResMut<Assets<Image>>,
) {
    // camera
    // 0,0 is the top left of the table
    // 980, 2290 is the bottom right of the table
    // z is up negated
    // we want to be looking at the center of the table standing on the bottom center

    let default_player_eyes_height = 1.0; // meters from the playfield
    let table_size: Vec2 = Vec2::new(table_resource.table_width_m, table_resource.table_height_m);
    let table_center = Vec3::new(table_size.x / 2.0, table_size.y / 2.0, 0.0);
    let player_location = Vec3::new(
        table_center.x,
        table_size.y,
        -default_player_eyes_height, // height above the table
    );

    info!("Spawning camera at height {:?} m", player_location.z);

    let camera_transform =
        Transform::from_translation(player_location).looking_at(table_center, Vec3::NEG_Z);

    // TODO we should be loading the table environment map from the table resource
    commands
        .spawn((
            Camera3d::default(),
            Camera {
                hdr: true,
                ..default()
            },
            camera_transform,
        ))
        // .insert(Skybox {
        //     brightness: 2000.0,
        //     image: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        //     ..default()
        // })
        .insert(EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 2000.0,
            ..default()
        });
}

fn rotate_camera_system(mut query: Query<&mut Transform, With<Camera3d>>, time: Res<Time>) {
    let center = Vec3::ZERO;
    // one full rotation every 15 seconds
    let angle = time.delta_secs() * 2.0 * std::f32::consts::PI / 15.0;

    for mut transform in query.iter_mut() {
        let rotation = Quat::from_rotation_y(angle);
        let offset = transform.translation - center;
        let new_offset = rotation * offset;
        transform.translation = center + new_offset;
        transform.look_at(center, Vec3::Y);
    }
}

fn height_camera_system(
    mut query: Query<&mut Transform, With<Camera3d>>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    table_resource: Res<TableResource>,
) {
    let table_center = Vec3::new(
        table_resource.table_width_m / 2.0,
        table_resource.table_height_m / 2.0,
        0.0,
    );

    for mut transform in query.iter_mut() {
        for event in mouse_wheel_events.read() {
            let move_amount = event.y * 0.05; // Adjust the zoom sensitivity as needed
            transform.translation.z += move_amount;
            transform.look_at(table_center, Vec3::NEG_Z);
        }
    }
}
