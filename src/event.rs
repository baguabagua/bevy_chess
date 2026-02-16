use::bevy::prelude::*;

use crate::{Game, UpdateBoard};

#[derive(Event)]
pub struct DeleteVariationEvent {
    pub node_to_delete: usize,
}

pub fn handle_delete_variation_events(
    mut game: ResMut<Game>,
    mut delete_events: EventReader<DeleteVariationEvent>,
    mut update_board_events: EventWriter<UpdateBoard>,
) {
    for event in delete_events.read() {
        game.tree.handle_delete_variation(event, &mut update_board_events);
    }
}