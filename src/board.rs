use std::{cmp::max, default};
use std::iter::*;
use bevy::prelude::*;
use crate::{
    piece::*,
    fen::*,
};

pub enum BoardResult {
    Winner(PieceColor),
    Draw,
}

#[derive(Clone)]
pub struct Board {
    pub pieces: Pieces,
    pub active_color: PieceColor,
    pub castling_availability: (bool, bool, bool, bool), // 王车易位权，四位依次表示白王侧、白后侧、黑王侧、黑后侧
    pub en_passant_target: Option<(usize, usize)>,
    pub halfmove: usize,
    pub fullmove: usize,
}

impl Default for Board {
    fn default() -> Self {
        read_fen(INITIAL_FEN.to_string())
    }
}

#[derive(PartialEq, Clone, Copy)]
pub struct Step {
    pub from: (usize, usize),
    pub to: (usize, usize),
}

fn abs_and_sign(num: isize) -> (isize, isize) {
    (num.abs(), num.signum())
}

fn add_offset(from: usize, delta: isize) -> usize {
    from.wrapping_add(delta as usize)
}

// 忽略所有特殊情况执行一步棋，进行棋子移动、活跃方转换和步数统计，吃过路兵改为不可行，易位权不变
fn act_move_pre(board: &Board, step: Step) -> Board {
    let mut b = board.clone();

    let (from_x, from_y) = step.from;
    let (to_x, to_y) = step.to;
    b.pieces[to_x][to_y] = b.pieces[from_x][from_y].take();
    b.active_color = b.active_color.flip();
    b.halfmove += 1;
    if b.active_color == PieceColor::White {
        b.fullmove += 1;
    }
    b.en_passant_target = None;

    b
}

// 忽略颜色，王车易位，将军，初步判断一步棋是否可行
fn can_move_pre(board: &Board, step: Step) -> bool {
    let pieces = &board.pieces;
    let (from_x, from_y) = step.from;
    let (to_x, to_y) = step.to;
    let Some(piece) = &pieces[from_x][from_y] else {
        return false
    };

    if let Some(to_piece) = &pieces[to_x][to_y] {
        if to_piece.piece_color == piece.piece_color {
            return false
        }
    }

    let dx: isize = to_x as isize - from_x as isize;
    let dy: isize = to_y as isize - from_y as isize;
    let (dx_val, dx_sig) = abs_and_sign(dx);
    let (dy_val, dy_sig) = abs_and_sign(dy);

    match piece.piece_role {
        PieceRole::Bishop => {
            if dx_val == dy_val {
                let mut res = true;
                for i in 1..dx_val {
                    let x = add_offset(from_x, dx_sig * i);
                    let y = add_offset(from_y, dy_sig * i);
                    // dbg!(x, y);
                    if pieces[x][y].is_some() {
                        res = false;
                    }
                }
                res
            } else {
                false
            }
        },
        PieceRole::Pawn => {
            let forward_dir = match piece.piece_color {
                PieceColor::White => 1, 
                PieceColor::Black => -1,  
            };
            
            // 普通前进
            if dx == 0 {
                // 前进1步
                if dy == forward_dir && pieces[to_x][to_y].is_none() {
                    return true
                }
                // 初始位置可以前进2步
                let start_row = match piece.piece_color {
                    PieceColor::White => 1,
                    PieceColor::Black => 6,
                };
                if from_y == start_row && dy == 2 * forward_dir && dx == 0 {
                    // 检查路径上是否有阻挡
                    let intermediate_y = add_offset(from_y, forward_dir);
                    if pieces[to_x][intermediate_y].is_none() && pieces[to_x][to_y].is_none() {
                        return true
                    }
                }
                false
            }
            // 吃子（斜着走1步，或吃过路兵）
            else if dy == forward_dir && dx_val == 1 {
                if pieces[to_x][to_y].is_some() 
                || board.en_passant_target.is_some_and(|pos| pos == (to_x, to_y)) {
                    true
                } else {
                    false
                }
            } else {
                false
            }
        },
        PieceRole::Rook => {
            if dx_val == 0 || dy_val == 0 {
                let mut res = true;
                for i in 1..(dx_val+dy_val) {
                    let x = add_offset(from_x, dx_sig * i);
                    let y = add_offset(from_y, dy_sig * i);
                    // dbg!(x, y);
                    if pieces[x][y].is_some() {
                        res = false;
                    }
                }
                res
            } else {
                false
            }
        },
        PieceRole::Knight => {
            dx_val > 0 && dx_val < 3 && dx_val + dy_val == 3
        },
        PieceRole::Queen => {
            if dx_val == dy_val || dx_val == 0 || dy_val == 0 {
                let mut res = true;
                for i in 1..max(dx_val, dy_val) {
                    let x = add_offset(from_x, dx_sig * i);
                    let y = add_offset(from_y, dy_sig * i);
                    // dbg!(x, y);
                    if pieces[x][y].is_some() {
                        res = false;
                    }
                }
                res
            } else {
                false
            }
        },
        PieceRole::King => {
            dx_val <= 1 && dy_val <= 1
        },
    }
}

// 判断一个盘面的某个棋子是否安全，即未被另一个颜色攻击，用于判断将军和王车易位的可行性
fn piece_safe(board: &Board, (x, y): (usize, usize)) -> bool {
    for i in 0..BOARD_SIZE_I {
        for j in 0..BOARD_SIZE_J {
            if can_move_pre(board, Step {
                from: (i, j),
                to: (x, y),
            }) {
                return false
            }
        }
    }
    true
}

// 寻找某个颜色的王的坐标。如果有多个，返回任意一个。如果没有，返回 None
fn king_pos(board: &Board, c: PieceColor) -> Option<(usize, usize)> {
    for i in 0..BOARD_SIZE_I {
        for j in 0..BOARD_SIZE_J {
            if let Some(piece) = board.pieces[i][j] {
                if piece.piece_role == PieceRole::King && piece.piece_color == c {
                    return Some((i, j))
                }
            }
        }
    }
    None
}

// 判断某个颜色的王是否安全。如果该颜色没有王则总是安全，如果该颜色有多个王则会判断其中某个王的安全性。不用在有多个王的时候调用。
pub fn king_safe(board: &Board, c: PieceColor) -> bool {
    let Some(kp) = king_pos(board, c) else {
        return true
    };
    piece_safe(board, kp)
}

// 判断局面是否合法。如果当前行动方能一步吃掉对手的王，则不合法
fn board_valid(board: &Board) -> bool {
    king_safe(board, board.active_color.flip())
}

// 尝试王车易位的中间方法。
fn try_castle_single(board: &Board, step: Step, passby: Vec<(usize, usize)>, king_passby: Vec<(usize, usize)>, rook_step: Step) -> Option<Board> {
    let pieces = &board.pieces;
    let (from_x, from_y) = step.from;

    // 判断是否被将军
    if !piece_safe(board, step.from) {
        debug!("try castle fail: king is checked");
        return None
    }

    // 判断中间是否有其它棋子阻隔，判断王经过或到达的位置是否被攻击
    for (i, j) in passby {
        if pieces[i][j].is_some() {
            debug!("try castle fail: there is another piece at ({}, {})", i, j);
            return None
        }
        
    }

    for (i, j ) in king_passby {
        let mut bp = board.clone();
        bp.pieces[i][j] = bp.pieces[from_x][from_y].take();
        if !piece_safe(&bp, (i, j)) {
            debug!("try castle fail: king is checked while moving to ({}, {})", i, j);
            return None
        }
    }

    let mut b = act_move_pre(board, step);
    let (rook_from_x, rook_from_y) = rook_step.from;
    let (rook_to_x, rook_to_y) = rook_step.to;
    b.pieces[rook_to_x][rook_to_y] = b.pieces[rook_from_x][rook_from_y].take();
    let (wk, wq, bk, bq) = board.castling_availability;
    b.castling_availability = match board.active_color {
        PieceColor::White => (false, false, bk, bq),
        PieceColor::Black => (wk, wq, false, false),
    };
    Some(b)
}

// 尝试王车易位。如果可行，返回成功后的盘面
pub fn try_castle(board: &Board, step: Step) -> Option<Board> {
    let (from_x, from_y) = step.from;
    let (to_x, to_y) = step.to;
    // 这里的标识仅用于表示王和车都尚未移动过
    let (wk, wq, bk, bq) = board.castling_availability;
    debug!("try castle availabilities: {}, {}, {}, {}", wk, wq, bk, bq);

    if from_x == 4 && to_x == 6 && from_y == 0 && to_y == 0 && wk {
        debug!("try castling wk");
        try_castle_single(board, step, vec![(5, 0), (6, 0)], vec![(5, 0), (6, 0)], Step { from: (7, 0), to: (5, 0) })
    } else if from_x == 4 && to_x == 2 && from_y == 0 && to_y == 0 && wq {
        debug!("try castling wq");
        try_castle_single(board, step, vec![(3, 0), (2, 0), (1, 0)], vec![(3, 0), (2, 0)], Step { from: (0, 0), to: (3, 0) })
    } else if from_x == 4 && to_x == 6 && from_y == 7 && to_y == 7 && bk {
        debug!("try castling bk");
        try_castle_single(board, step, vec![(5, 7), (6, 7)], vec![(5, 7), (6, 7)], Step { from: (7, 7), to: (5, 7) })
    } else if from_x == 4 && to_x == 2 && from_y == 7 && to_y == 7 && bq {
        debug!("try castling bq");
        try_castle_single(board, step, vec![(3, 7), (2, 7), (1, 7)], vec![(3, 7), (2, 7)], Step { from: (0, 7), to: (3, 7) })
    } else {
        None
    }
}

// 尝试移动。如果可行，返回成功后的盘面。兵自动升变为后。如果移动没有应将或者送王，那么移动不可行。
pub fn try_move(board: &Board, step: Step) -> Option<Board> {
    let res: Option<Board>;

    let pieces = &board.pieces;
    let (from_x, from_y) = step.from;
    let (to_x, to_y) = step.to;
    let Some(piece) = &pieces[from_x][from_y] else {
        return None
    };

    if piece.piece_color != board.active_color {
        return None
    }

    if let Some(to_piece) = &pieces[to_x][to_y] {
        if to_piece.piece_color == piece.piece_color {
            return None
        }
    }

    if can_move_pre(board, step) {
        let mut b = act_move_pre(board, step);

        let role = piece.piece_role;

        // 吃过路兵与升变
        if role == PieceRole::Pawn {
            if board.en_passant_target.is_some_and(|tar| {tar == step.to}) {
                match board.active_color {
                    PieceColor::White => { b.pieces[to_x][to_y-1] = None; },
                    PieceColor::Black => { b.pieces[to_x][to_y+1] = None; },
                }
            }
            if to_y == from_y + 2 {
                b.en_passant_target = Some((to_x, from_y + 1))
            } else if from_y == to_y + 2 {
                b.en_passant_target = Some((to_x, to_y + 1))
            }
            if to_y == 0 || to_y == 7 {
                if let Some(p) = &mut b.pieces[to_x][to_y] {
                    p.piece_role = PieceRole::Queen;
                }
            }
        }

        // 处理王车易位权
        if role == PieceRole::King {
            match board.active_color {
                PieceColor::White => { b.castling_availability.0 = false; b.castling_availability.1 = false; },
                PieceColor::Black => { b.castling_availability.2 = false; b.castling_availability.3 = false; },
            }
        }
        if role == PieceRole::Rook {
            match step.from {
                (0, 7) => { b.castling_availability.0 = false; },
                (0, 0) => { b.castling_availability.1 = false; },
                (7, 7) => { b.castling_availability.2 = false; },
                (7, 0) => { b.castling_availability.3 = false; },
                _default => {},
            }
        }
        match step.to {
            (0, 7) => { b.castling_availability.0 = false; },
            (0, 0) => { b.castling_availability.1 = false; },
            (7, 7) => { b.castling_availability.2 = false; },
            (7, 0) => { b.castling_availability.3 = false; },
            _default => {},
        }

        res = Some(b)
    } else {
        // 王车易位
        res = try_castle(board, step);
    }

    if let Some(ref b) = res {
        if let Some(kp) = king_pos(&b, b.active_color.flip()) {
            if piece_safe(&b, kp) {
                res
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

// 查询从一个位置出发可走到的所有位置
pub fn all_targets(board: &Board, (from_x, from_y): (usize, usize)) -> Vec<(usize, usize)> {
    (0..BOARD_SIZE_I).flat_map(|to_x| {
        (0..BOARD_SIZE_J).filter_map(move |to_y| {
            try_move(board, Step {
                from: (from_x, from_y),
                to: (to_x, to_y),
            }).map(|_| (to_x, to_y))
        })
    }).collect()
}

// 查询可能的所有移动
pub fn all_move(board: &Board) -> Vec<Step> {
    (0..BOARD_SIZE_I).flat_map(|from_x| {
        (0..BOARD_SIZE_J).flat_map(move |from_y| {
            board.pieces[from_x][from_y]
                .filter(|piece| piece.piece_color == board.active_color)
                .map(|_| {
                    all_targets(board, (from_x, from_y))
                    .into_iter()
                    .map(move |to| Step { from: (from_x, from_y), to })
                })
                .into_iter()
                .flatten()
        })
    }).collect()
}

// 判断当前局面是否是终局。如果是，返回棋局结果
pub fn end_game(board: &Board) -> Option<BoardResult> {
    if all_move(board).is_empty() {
        if let Some(kp) = king_pos(board, board.active_color) {
            if piece_safe(board, kp) {
                Some(BoardResult::Draw)
            } else {
                Some(BoardResult::Winner(board.active_color.flip()))
            }
        } else {
            Some(BoardResult::Draw)
        }
    } else {
        None
    }
}

// 返回局面信息（终局或行动方）
pub fn game_info(board: &Board) -> String {
    if let Some(res) = end_game(board) {
        match res {
            BoardResult::Winner(piece_color) => {
                match piece_color {
                    PieceColor::White => "White win".to_string(),
                    PieceColor::Black => "Black win".to_string(),
                }
            },
            BoardResult::Draw => "Draw".to_string(),
        }
    } else {
        match board.active_color {
            PieceColor::White => "White play".to_string(),
            PieceColor::Black => "Black play".to_string(),
        }
    }
}
