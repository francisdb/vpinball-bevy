use bevy::input::ButtonInput;
use bevy::log::info;
use bevy::prelude::*;

pub(crate) struct ControlGizmoPlugin;

impl Plugin for ControlGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_gizmo_config)
            .add_systems(Update, (toggle_light_gizmos, draw_coordinate_axes));
    }
}

fn setup_gizmo_config(mut gizmo_config_store: ResMut<GizmoConfigStore>) {
    gizmo_config_store
        .config_mut::<LightGizmoConfigGroup>()
        .1
        .draw_all = true;

    gizmo_config_store
        .config_mut::<DefaultGizmoConfigGroup>()
        .0
        .depth_bias = -0.1;
}

fn toggle_light_gizmos(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut gizmo_config_store: ResMut<GizmoConfigStore>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyG) {
        let config = gizmo_config_store.config_mut::<LightGizmoConfigGroup>();
        config.1.draw_all = !config.1.draw_all;
        info!(
            "Light gizmos: {}",
            if config.1.draw_all { "ON" } else { "OFF" }
        );
    }
}

// Add this system to your HelloPlugin or directly in main()
fn draw_coordinate_axes(mut gizmos: Gizmos) {
    let world_center = Vec3::ZERO;
    let axis_length = 0.5; // Half meter long axes

    let red = Color::srgb(1.0, 0.0, 0.0);
    let green = Color::srgb(0.0, 1.0, 0.0);
    let blue = Color::srgb(0.0, 0.0, 1.0);

    // Draw world coordinate axes at table center
    gizmos.arrow(world_center, Vec3::X * axis_length, red); // X axis (right)
    gizmos.arrow(world_center, Vec3::Y * axis_length, green); // Y axis (up)
    gizmos.arrow(world_center, Vec3::Z * axis_length, blue); // Z axis (forward)

    // Add labels
    // gizmos.text(world_center + Vec3::X * axis_length, "X", red);
    // gizmos.text(world_center + Vec3::Y * axis_length, "Y", green);
    // gizmos.text(world_center + Vec3::Z * axis_length, "Z", blue);
}
