use crate::TableResource;
use bevy::app::{App, Plugin, Startup, Update};
use bevy::color::Color;
use bevy::input::ButtonInput;
use bevy::log::info;
use bevy::math::Vec3;
use bevy::pbr::PointLight;
use bevy::prelude::*;
use vpin::vpx::gameitem::light::Light;
use vpin::vpx::vpu_to_m;

#[derive(Resource, Debug)]
enum LightToggleState {
    Off,
    On,
    Default,
}

impl LightToggleState {
    fn toggle(&mut self) {
        *self = match *self {
            LightToggleState::Off => LightToggleState::On,
            LightToggleState::On => LightToggleState::Default,
            LightToggleState::Default => LightToggleState::Off,
        };
    }
}

#[derive(Component)]
struct VPLight {
    default_state: f32,
    intensity: f32,
}

pub(crate) struct ControlLightsPlugin;

impl Plugin for ControlLightsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_light_toggle_state)
            .add_systems(Update, toggle_lights_system);
    }
}

pub(crate) fn spawn_overhead_lights(commands: &mut Commands, table: Res<TableResource>) {
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

pub(crate) fn spawn_light(commands: &mut Commands, light: &Light) {
    info!(
        "Spawning light: {}, state: {:?}, intensity: {}, range {}",
        light.name, light.state, light.intensity, light.falloff_radius
    );
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

    let visibility = visible_to_visibility(light.visible);

    let default_state = light.state.unwrap_or(light.state_u32 as f32);

    commands.spawn((
        Name::new(light.name.to_string()),
        VPLight {
            default_state,
            intensity: light.intensity,
        },
        PointLight {
            // FIXME enabling shadows slows down the rendering
            // shadows_enabled: true,
            // shadow_depth_bias: 0.0,
            range: vpu_to_m(light.falloff_radius),
            intensity: light.intensity * default_state,
            color,
            ..default()
        },
        visibility,
        transform,
    ));
}

fn toggle_lights_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut PointLight, &VPLight)>,
    mut state: ResMut<LightToggleState>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyL) {
        state.toggle();

        for (mut point_light, vp_light) in query.iter_mut() {
            point_light.intensity = match state.as_ref() {
                LightToggleState::Off => vp_light.intensity * 0.0,
                LightToggleState::On => vp_light.intensity * 1.0,
                LightToggleState::Default => vp_light.intensity * vp_light.default_state,
            };
        }

        info!("All lights are now {:?}", state);
    }
}

fn setup_light_toggle_state(mut commands: Commands) {
    commands.insert_resource(LightToggleState::Default);
}

fn visible_to_visibility(visible: Option<bool>) -> Visibility {
    match visible {
        Some(true) => Visibility::Visible,
        Some(false) => Visibility::Hidden,
        None => Visibility::Visible,
    }
}
