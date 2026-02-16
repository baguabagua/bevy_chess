use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::{
    fen::*, game_tree::GameTree, Game, UpdateBoard
};

#[derive(Default, Resource)]
pub struct UiMenuState {
    load_pgn: String,
    load_tree: String, 
    load_tree_error: String,
    pub fen_window_open: bool,
    pub tree_window_open: bool,
}

pub fn ui_menu(
    mut ui_state: ResMut<UiMenuState>,
    mut contexts: EguiContexts,
    mut event_writer: EventWriter<UpdateBoard>,
    mut game: ResMut<Game>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    egui::SidePanel::left("side_panel")
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.heading("Menu");

            ui.separator();

            ui.checkbox(&mut ui_state.fen_window_open, "show FEN window");
            ui.checkbox(&mut ui_state.tree_window_open, "show game tree");

            ui.separator();

            if ui.button("New Game").clicked() {
                let new_board = read_fen(INITIAL_FEN.to_string());
                event_writer.write(UpdateBoard {
                    new_board: new_board.clone(),
                });
                game.tree = GameTree::new(new_board);
            }

            ui.separator();

            if ui.button("Copy current game tree").clicked() {
                ctx.copy_text(game.tree.to_string());
            }

            ui.horizontal(|ui| {
                ui.label("Load game tree: ");
                egui::TextEdit::multiline(&mut ui_state.load_tree)
                    .desired_rows(4)
                    .show(ui);
            });
            ui.horizontal(|ui| {
                if ui.button("Load").clicked() {
                    if let Some(tree) = GameTree::from_string(ui_state.load_tree.clone()) {
                        game.tree = tree;
                        event_writer.write(UpdateBoard { new_board: game.tree.board() });
                    } else {
                        ui_state.load_tree_error = "invalid tree text".to_string();
                    }
                }
                ui.label(ui_state.load_tree_error.clone());
            });

            ui.horizontal(|ui| {
                ui.label("Load PGN: ");
                ui.text_edit_singleline(&mut ui_state.load_pgn);
            });
            if ui.button("Load").clicked() {
                game.tree = GameTree::from_pgn(ui_state.load_pgn.clone());
                event_writer.write(UpdateBoard { new_board: game.tree.board() });
            }
        });

    Ok(())
}