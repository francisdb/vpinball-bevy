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
        app.add_systems(Update, (height_camera_system, move_camera_system));
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

    // The 0.0 y (height) is the location of the playfield?
    let default_player_eyes_height = 0.6; // meters from the playfield + playfield height
    let table_size = table_resource.table_size();
    let table_center = get_table_center(table_size);
    let player_location = Vec3::new(
        table_center.x,
        default_player_eyes_height, // height above the table
        table_size.y,               // up
    );

    info!("Spawning camera at height {:?} m", player_location.z);

    let camera_transform =
        Transform::from_translation(player_location).looking_at(table_center, Dir3::Y);

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
        // TODO vpinball has an Environment Lighting (hdr file) and a Power value (int)
        .insert(EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 200.0,
            ..default()
        });
}

fn get_table_center(table_size: Vec2) -> Vec3 {
    Vec3::new(table_size.x / 2.0, 0.0, table_size.y / 2.0)
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
    let table_center = get_table_center(table_resource.table_size());

    for mut transform in query.iter_mut() {
        for event in mouse_wheel_events.read() {
            let move_amount = event.y * 0.05; // Adjust the zoom sensitivity as needed
            transform.translation.y += move_amount;
            transform.look_at(table_center, Dir3::Y);
        }
    }
}

fn move_camera_system(
    mut query: Query<&mut Transform, With<Camera3d>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    table_resource: Res<TableResource>,
) {
    let table_center = get_table_center(table_resource.table_size());
    let move_speed = 0.5; // in meters per second
    let delta_time = time.delta_secs();

    for mut transform in query.iter_mut() {
        let mut movement = Vec3::ZERO;

        // Get forward and right directions from camera's current orientation
        // Using camera's transform but keeping movement in the X-Z plane
        let forward = Vec3::new(0.0, 0.0, -1.0).normalize();
        let right = Vec3::new(1.0, 0.0, 0.0).normalize();

        // Calculate movement based on input
        if keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW) {
            movement += forward;
        }
        if keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS) {
            movement -= forward;
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD) {
            movement += right;
        }
        if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
            movement -= right;
        }

        // Apply movement if any keys were pressed
        if movement != Vec3::ZERO {
            let normalized_movement = movement.normalize() * move_speed * delta_time;
            transform.translation += normalized_movement;

            // Keep looking at the table center
            transform.look_at(table_center, Dir3::Y);
        }
    }
}
