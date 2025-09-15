use bevy::log::info;
use bevy::prelude::{Name, Query, Trigger};
use bevy_picking::events::{Click, Pointer};

pub(crate) fn on_click_print_name(click: Trigger<Pointer<Click>>, q_names: Query<&Name>) {
    let entity = click.target();
    if let Ok(name) = q_names.get(entity) {
        info!("{} ({}) was clicked!", name, entity);
    } else {
        info!("Entity {} was clicked!", entity);
    }
}
