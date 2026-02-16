use bevy::{
    color::palettes::css::WHITE, input::common_conditions::{input_just_pressed, input_just_released}, prelude::*, render::view::RenderLayers, sprite::Anchor, transform, window::{PrimaryWindow, WindowLevel}
};
use bevy_egui::{
    egui, EguiContext, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass,
    PrimaryEguiContext,
};

use crate::{
    piece::*,
    board::*,
    fen::*,
    ui_fen::*,
    game_tree::*,
    ui_game_tree::*,
    menu::*,
    event::*,
};

mod fen;
mod piece;
mod board;
mod step;
mod pgn;
mod menu;
mod ui_fen;
mod game_tree;
mod ui_game_tree;
mod event;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum GameState {
    #[default]
    Playing,
    GameOver,
}

const BACKGROUND_COLOR: Color = Color::srgb(0.0, 0.0, 0.0);
const BLACKCELL_COLOR: Color = Color::srgb(181.0/256.0, 136.0/256.0, 99.0/256.0);
const WHITECELL_COLOR: Color = Color::srgb(240.0/256.0, 217.0/256.0, 181.0/256.0);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Chess".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .init_resource::<Game>()
        .init_resource::<UiMenuState>()
        .init_resource::<UiFenState>()
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(CursorWorldPos(None))
        .init_state::<GameState>()
        .add_event::<UpdateBoard>()
        .add_event::<DeleteVariationEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update, 
            (
                get_cursor_world_pos,
                (
                    start_drag.run_if(input_just_pressed(MouseButton::Left)),
                    end_drag.run_if(input_just_released(MouseButton::Left)),
                    drag.run_if(resource_exists::<DragOperation>),
                ),
                update_board.run_if(on_event::<UpdateBoard>),
            ).chain(),
        )
        .add_systems(
            EguiPrimaryContextPass, 
            (
                (ui_fen_system, ui_game_tree, ui_menu),
                handle_delete_variation_events
            ).chain(),
        )
        .run();
}

/// The projected 2D world coordinates of the cursor (if it's within primary window bounds).
#[derive(Resource)]
struct CursorWorldPos(Option<Vec2>);

#[derive(Component)]
struct MainCamera;

/// The current drag operation including the offset with which we grabbed the Bevy logo.
#[derive(Resource)]
struct DragOperation {
    dragged_entity: Entity,
    start_cell: Entity,
}

#[derive(Component)]
struct PieceCom {
    x: usize,
    y: usize,
    entity: Option<Entity>,
}

#[derive(Component, Clone)]
struct CellCom {
    x: usize,
    y: usize,
    entity: Option<Entity>,
}

#[derive(Resource, Default)]
struct Game {
    board: Board,
    tree: GameTree,
    result: Option<BoardResult>,
    cells: Vec<Vec<CellCom>>,
    pieces: Vec<Vec<Option<PieceCom>>>,
    leftdown: (f32, f32),
    info: Option<Entity>,
}

#[derive(Event)]
struct UpdateBoard {
    new_board: Board,
}

const CELL_SIZE_I: f32 = 50.0;
const CELL_SIZE_J: f32 = 50.0;
const CELL_SIZE: Vec2 = Vec2::new(CELL_SIZE_I, CELL_SIZE_J);

fn setup(
    mut commands: Commands, 
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    asset_server: Res<AssetServer>, 
    mut game: ResMut<Game>,
    mut ui_state: ResMut<UiFenState>,
) {
    egui_global_settings.auto_create_primary_context = false;

    commands.spawn((Camera2d::default(), MainCamera));

    ui_state.current_fen = INITIAL_FEN.to_string();

    // setup game tree
    game.tree = GameTree::new(game.board.clone());

    // setup cells
    game.cells = (0..BOARD_SIZE_I)
            .map(|i| (0..BOARD_SIZE_J).map(|j| CellCom {
                x: i,
                y: j,
                entity: None,
            }).collect())
            .collect();

    let (center_x, center_y) = (0.0, 0.0);
    let (leftdown_x, leftdown_y) = (center_x - BOARD_SIZE_I as f32 / 2.0 * CELL_SIZE_I, center_y - BOARD_SIZE_J as f32 / 2.0 * CELL_SIZE_J);
    game.leftdown = (leftdown_x, leftdown_y);

    for i in 0..BOARD_SIZE_I {
        for j in 0..BOARD_SIZE_J {
            let color = if (i+j) % 2 == 0 {
                BLACKCELL_COLOR
            } else {
                WHITECELL_COLOR
            };
            let (x, y) = (leftdown_x + (i as f32 + 0.5) * CELL_SIZE_I, leftdown_y + (j as f32 + 0.5) * CELL_SIZE_J);
            game.cells[i][j].entity = Some(commands.spawn((
                Sprite::from_color(color, CELL_SIZE),
                Transform::from_xyz(x, y, 0.0),
                game.cells[i][j].clone(),
            )).id());
        }
    }

    // setup pieces
    game.pieces = (0..BOARD_SIZE_I)
            .map(|_| (0..BOARD_SIZE_J).map(|_| None)
            .collect())
            .collect();

    for i in 0..BOARD_SIZE_I {
        for j in 0..BOARD_SIZE_J {
            if let Some(ref mut piece) = game.board.pieces[i][j] {
                let (x, y) = (leftdown_x + (i as f32 + 0.5) * CELL_SIZE_I, leftdown_y + (j as f32 + 0.5) * CELL_SIZE_J);
                let e = commands.spawn((
                    {
                        let mut sprite = Sprite::from_image(asset_server.load(format!("chess_pieces/{}.png", piece.to_string())));
                        sprite.custom_size = Some(CELL_SIZE);
                        sprite
                    },
                    Transform::from_xyz(x, y, 1.0),
                )).id();
                game.pieces[i][j] = Some(PieceCom {
                    x: i,
                    y: j,
                    entity: Some(e),
                });
            }
        }
    }

    // game info
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_font = TextFont {
        font: font.clone(),
        font_size: 50.0,
        ..default()
    };
    game.info = Some(commands.spawn((
        Text2d::new("White play"),
        text_font.clone(),
        TextColor(WHITE.into()),
        Transform::from_xyz(0.0, 250.0, 0.0),
        Anchor::Center, 
    )).id());

    // Egui camera.
    commands.spawn((
        PrimaryEguiContext,
        Camera2d,
        RenderLayers::none(),
        Camera {
            order: 1,
            ..default()
        },
    ));
}

/// Project the cursor into the world coordinates and store it in a resource for easy use
fn get_cursor_world_pos(
    mut cursor_world_pos: ResMut<CursorWorldPos>,
    q_primary_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let primary_window = q_primary_window.single();
    let (main_camera, main_camera_transform) = q_camera.single().unwrap();
    // Get the cursor position in the world
    cursor_world_pos.0 = primary_window.unwrap()
        .cursor_position()
        .and_then(|cursor_pos| 
            main_camera.viewport_to_world_2d(main_camera_transform, cursor_pos).ok()
        );
}

fn cursor_cell(
    cursor_world_pos: Vec2,
    q_cell: Query<(&Sprite, &Transform, Entity), With<CellCom>>,
) -> Option<Entity> {
    for (sprite, transform, entity) in q_cell.iter() {
        let sprite_size = sprite.custom_size.unwrap_or(Vec2::new(1.0, 1.0));
        let min = transform.translation.truncate() - sprite_size / 2.0;
        let max = transform.translation.truncate() + sprite_size / 2.0;

        if cursor_world_pos.x >= min.x && cursor_world_pos.x <= max.x && 
        cursor_world_pos.y >= min.y && cursor_world_pos.y <= max.y {
            return Some(entity)
        }
    }
    return None
}

fn start_drag(
    mut commands: Commands,
    game: Res<Game>,
    cursor_world_pos: Res<CursorWorldPos>,
    q_cell: Query<(&Sprite, &Transform, Entity), With<CellCom>>,
    cells: Query<&CellCom>,
) {
    // If the cursor is not within the primary window skip this system
    let Some(cursor_world_pos) = cursor_world_pos.0 else {
        return;
    };

    let Some(cell) = cursor_cell(cursor_world_pos, q_cell) else {
        return;
    };

    let Ok(cell_com) = cells.get(cell) else {
        error!("Click a cell with no CellCom");
        return;
    };

    let x = cell_com.x;
    let y = cell_com.y;

    if let Some(piece) = &game.pieces[x][y] {
        commands.insert_resource(DragOperation {
            dragged_entity: piece.entity.unwrap(),
            start_cell: cell,
        });
    }
}

fn drag(
    drag_operation: Option<Res<DragOperation>>,
    cursor_world_pos: Res<CursorWorldPos>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(drag_operation) = drag_operation else {
        return;
    };

    let Some(cursor_world_pos) = cursor_world_pos.0 else {
        return;
    };

    let entity = drag_operation.dragged_entity;

    if let Ok(mut transform) = transforms.get_mut(entity) {
        transform.translation.x = cursor_world_pos.x;
        transform.translation.y = cursor_world_pos.y;
    }
}

fn update_board(
    mut event_reader: EventReader<UpdateBoard>,
    mut commands: Commands, 
    mut game: ResMut<Game>,
    asset_server: Res<AssetServer>,
    mut q_text: Query<&mut Text2d>,
    mut ui_state: ResMut<UiFenState>,
) {
    let mut new_board = None;
    for event in event_reader.read() {
        new_board = Some(&event.new_board);
    }
    let Some(new_board) = new_board else{
        error!("update board read no event");
        return;
    };
    
    for i in 0..BOARD_SIZE_I {
        for j in 0..BOARD_SIZE_J {
            if game.board.pieces[i][j] != new_board.pieces[i][j] {
                if let Some(old_piece) = &game.pieces[i][j] {
                    commands.entity(old_piece.entity.unwrap()).despawn();
                }
                if let Some(new_piece) = &new_board.pieces[i][j] {
                    let (leftdown_x, leftdown_y) = game.leftdown;
                    let (x, y) = (leftdown_x + (i as f32 + 0.5) * CELL_SIZE_I, leftdown_y + (j as f32 + 0.5) * CELL_SIZE_J);
                    let e = commands.spawn((
                        {
                            let mut sprite = Sprite::from_image(asset_server.load(format!("chess_pieces/{}.png", new_piece.to_string())));
                            sprite.custom_size = Some(CELL_SIZE);
                            sprite
                        },
                        Transform::from_xyz(x, y, 1.0),
                    )).id();
                    game.pieces[i][j] = Some(PieceCom {
                        x: i,
                        y: j,
                        entity: Some(e),
                    });
                }
                
            }
        }
    }
    game.board = new_board.clone();

    q_text.get_mut(game.info.unwrap()).unwrap().0 = game_info(&game.board);
    ui_state.current_fen = write_fen(new_board.clone());
}

fn end_drag(
    drag_operation: Option<Res<DragOperation>>,
    mut commands: Commands, 
    cursor_world_pos: Res<CursorWorldPos>,
    mut game: ResMut<Game>,
    q_cell: Query<(&Sprite, &Transform, Entity), With<CellCom>>,
    mut event_writer: EventWriter<UpdateBoard>,
    cells: Query<&CellCom>,
    mut transforms: Query<&mut Transform, Without<CellCom>>,
) {
    let Some(drag_operation) = drag_operation else {
        return;
    };

    let entity = drag_operation.dragged_entity;

    let mut moved = false;
    if let Some(cursor_world_pos) = cursor_world_pos.0 {
        if let Some(to_cell) = cursor_cell(cursor_world_pos, q_cell) {
            let from_x = cells.get(drag_operation.start_cell).unwrap().x;
            let from_y = cells.get(drag_operation.start_cell).unwrap().y;
            let to_x = cells.get(to_cell).unwrap().x;
            let to_y = cells.get(to_cell).unwrap().y;
            let step = Step {
                from: (from_x, from_y),
                to: (to_x, to_y), 
            };

            if let Some(new_board) = try_move(&game.board, step) {
                moved = true;
                event_writer.write(UpdateBoard {
                    new_board: new_board,
                });
                game.tree.try_move(step);
            }
        }
    }
    
    if !moved {
        let (x, y) = if let Ok((_sprite, trans, _entity)) = q_cell.get(drag_operation.start_cell) {
            (trans.translation.x, trans.translation.y)
        } else {
            error!("piece move failed and cannot find the start cell");
            (cursor_world_pos.0.unwrap().x, cursor_world_pos.0.unwrap().y)
        };
        if let Ok(mut transform) = transforms.get_mut(entity) {
            transform.translation.x = x;
            transform.translation.y = y;
        }
    }

    commands.remove_resource::<DragOperation>();
}
