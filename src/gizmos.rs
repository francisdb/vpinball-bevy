use bevy::input::ButtonInput;
use bevy::log::info;
use bevy::prelude::*;

pub(crate) struct ControlGizmoPlugin;

impl Plugin for ControlGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_gizmo_config)
            .add_systems(Update, toggle_light_gizmos);
    }
}

fn setup_gizmo_config(mut gizmo_config_store: ResMut<GizmoConfigStore>) {
    gizmo_config_store
        .config_mut::<LightGizmoConfigGroup>()
        .1
        .draw_all = true;
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
