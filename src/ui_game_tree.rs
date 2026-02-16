use bevy::prelude::*;
use bevy_egui::{egui::{self, Ui, Grid}, EguiContexts};
use crate::{
    event::DeleteVariationEvent, game_tree::*, menu::UiMenuState, Game, UpdateBoard
};

pub fn ui_game_tree(
    mut contexts: EguiContexts,
    mut event_writer: EventWriter<UpdateBoard>,
    mut game: ResMut<Game>,
    mut ui_menu: ResMut<UiMenuState>,
    mut ew_dv: EventWriter<DeleteVariationEvent>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    egui::Window::new("Game Tree")
        .open(&mut ui_menu.tree_window_open)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    // ui.label("empty game tree");
                    game.tree.display_egui(ui, &mut event_writer, &mut ew_dv);
                });
            
            ui.separator();
            egui::TopBottomPanel::bottom("button_panel")
                .show_inside(ui, |ui| {
                    let total_width = ui.available_width();
                    let btn_width = total_width * 0.20;
                    let is_first_board = game.tree.is_first_board();
                    let is_last_board = game.tree.is_last_board();

                    Grid::new("navigation_buttons")
                        .num_columns(4)
                        .min_col_width(btn_width)
                        .show(ui, |ui| {
                            let response = ui.add_enabled(!is_first_board, |ui: &mut Ui| {
                                ui.add_sized([btn_width, 0.0], egui::Button::new("<<"))
                            });
                            if response.clicked() {
                                game.tree.move_to_start();
                                event_writer.write(UpdateBoard {
                                    new_board: game.tree.board()
                                });
                            }

                            let response = ui.add_enabled(!is_first_board, |ui: &mut Ui| {
                                ui.add_sized([btn_width, 0.0], egui::Button::new("<"))
                            });
                            if response.clicked() {
                                game.tree.move_backward();
                                event_writer.write(UpdateBoard {
                                    new_board: game.tree.board()
                                });
                            }
                            
                            let response = ui.add_enabled(!is_last_board, |ui: &mut Ui| {
                                ui.add_sized([btn_width, 0.0], egui::Button::new(">"))
                            });
                            if response.clicked() {
                                game.tree.move_forward();
                                event_writer.write(UpdateBoard {
                                    new_board: game.tree.board()
                                });
                            }

                            let response = ui.add_enabled(!is_last_board, |ui: &mut Ui| {
                                ui.add_sized([btn_width, 0.0], egui::Button::new(">>"))
                            });
                            if response.clicked() {
                                game.tree.move_to_end();
                                event_writer.write(UpdateBoard {
                                    new_board: game.tree.board()
                                });
                            }
                        });
                });
        });

    Ok(())
}