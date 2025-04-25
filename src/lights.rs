use crate::TableResource;
use bevy::color::Color;
use bevy::log::info;
use bevy::math::Vec3;
use bevy::pbr::PointLight;
use bevy::prelude::{Commands, Name, Res, Transform, Visibility, default};
use vpin::vpx::gameitem::light::Light;
use vpin::vpx::vpu_to_m;

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
