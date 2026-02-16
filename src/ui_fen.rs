use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::{
    fen::*, game_tree::GameTree, Game, UpdateBoard, menu::*,
};

#[derive(Default, Resource)]
pub struct UiFenState {
    pub current_fen: String,
    pub load_fen: String,
    pub error_info: String,
}

pub fn ui_fen_system(
    mut ui_state: ResMut<UiFenState>,
    mut contexts: EguiContexts,
    mut event_writer: EventWriter<UpdateBoard>,
    mut game: ResMut<Game>,
    mut ui_menu: ResMut<UiMenuState>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    egui::Window::new("FEN")
        .vscroll(true)
        .open(&mut ui_menu.fen_window_open)
        .show(ctx, |ui| {
            ui.label(format!(
                "Current FEN: {}",
                ui_state.current_fen,
            ));
            if ui.button("Copy").clicked() {
                ui.ctx().copy_text(ui_state.current_fen.clone());
            }

            ui.horizontal(|ui| {
                ui.label("Load FEN: ");
                ui.text_edit_singleline(&mut ui_state.load_fen);
            });
            ui.horizontal(|ui| {
                if ui.button("Load").clicked() {
                    let new_board = read_fen(ui_state.load_fen.clone());
                    event_writer.write(UpdateBoard {
                        new_board: new_board.clone(),
                    });
                    game.tree = GameTree::new(new_board);
                }
                ui.label(ui_state.error_info.clone());
            });
            if ui.button("New Game").clicked() {
                let new_board = read_fen(INITIAL_FEN.to_string());
                event_writer.write(UpdateBoard {
                    new_board: new_board.clone(),
                });
                game.tree = GameTree::new(new_board);
            }
        });

    Ok(())
}