use crate::{
    piece::*,
    board::*,
};
use regex::Regex;

fn piece_name(piecerole: PieceRole) -> String {
    match piecerole {
        PieceRole::Pawn => "".to_string(),
        PieceRole::Rook => "R".to_string(),
        PieceRole::Knight => "N".to_string(),
        PieceRole::Bishop => "B".to_string(),
        PieceRole::Queen => "Q".to_string(),
        PieceRole::King => "K".to_string(),
    }
}

fn name_to_role(name: String) -> PieceRole {
    match name.as_str() {
        "" => PieceRole::Pawn,
        "K" => PieceRole::King,
        "Q" => PieceRole::Queen,
        "R" => PieceRole::Rook,
        "B" => PieceRole::Bishop,
        "N" => PieceRole::Knight,
        _ => {
            unreachable!()
        },
    }
}

fn coordinate(s: String) -> (usize, usize) {
    let (x, y) = (s.as_bytes()[0], s.as_bytes()[1]);
    (
        (x - (b'a' as u8)) as usize,
        (y - (b'1' as u8)) as usize, 
    )
}

// 将一个表示步的字符串转换为Step. 
pub fn read_step(board: &Board, s: String) -> Option<Step> {
    let color = board.active_color;
    if s.starts_with("O-O") {
        let (wk, wq, bk, bq) = board.castling_availability;
        let step = if s.starts_with("O-O-O") {
            match color {
                PieceColor::White => {
                    if !wq { return None }
                    Step { from: (4, 0), to: (2, 0) }
                },
                PieceColor::Black => {
                    if !bq { return None }
                    Step { from: (4, 7), to: (2, 7) }
                },
            }
        } else {
            match color {
                PieceColor::White => {
                    if !wk { return None }
                    Step { from: (4, 0), to: (6, 0) }
                },
                PieceColor::Black => {
                    if !bk { return None }
                    Step { from: (4, 7), to: (2, 7) }
                },
            }
        };
        if write_step(board, step).is_some_and(|res_s| s == res_s) {
            Some(step)
        } else {
            None
        }
    } else {
        let chess_move_re = Regex::new(
            r"^([KQRBN]?)([a-h]?[1-8]?)(x?)([a-h][1-8])(?:=([QRBN]))?([+#]?)$"
        ).unwrap();
        
        let Some((role, target)) = chess_move_re.captures(&s).map(|caps| (
            caps.get(1).map_or("", |m| m.as_str()),  // role
            caps.get(4).map_or("", |m| m.as_str()),  // target
        )) else {
            return None
        };

        let (role, target) = (
            name_to_role(role.to_string()),
            coordinate(target.to_string()),
        );

        
        for fx in 0..BOARD_SIZE_I {
            for fy in 0..BOARD_SIZE_J {
                let Some(p) = board.pieces[fx][fy] else {
                    continue;
                };
                if p.piece_color != color || p.piece_role != role {
                    continue;
                }
                let step = Step {
                    from: (fx, fy),
                    to: target,
                };
                if write_step(board, step).is_some_and(|res_s| s == res_s) {
                    return Some(step)
                }
            }
        }

        return None
    }
}

pub fn write_step(board: &Board, step: Step) -> Option<String> {
    let Some(new_board) = try_move(board, step) else {
        return None
    };

    let (from_x, from_y) = step.from;
    let (to_x, to_y) = step.to;
    let Some(piece) = &board.pieces[from_x][from_y] else {
        unreachable!()
    };
    let role = piece.piece_role;

    let name = piece_name(role);
    let target = format!("{}{}", (b'a' + to_x as u8) as char, to_y + 1);
    let mut capture = board.pieces[to_x][to_y].is_some();
    let check = !king_safe(&new_board, new_board.active_color);
    let checkmate = match end_game(&new_board) {
        Some(res) => match res {
            BoardResult::Winner(_) => true,
            BoardResult::Draw => false,
        },
        None => false,
    };
    let check_string = if checkmate { 
        "#" 
    } else if check {
        "+"
    } else {
        ""
    };

    // 处理王车易位
    if role == PieceRole::King && try_castle(board, step).is_some() {
        let castle =  if to_x == 6 {
            "O-O"
        } else if to_x == 2 {
            "O-O-O"
        } else {
            unreachable!()
        };
        return Some(format!("{}{}", castle, check_string))
    }

    // 处理吃过路兵和升变
    let mut promotion = "".to_string();
    if role == PieceRole::Pawn {
        if board.en_passant_target.is_some_and(|tar| {tar == step.to}) {
            capture = true;
        }
        if to_y == 0 || to_y == 7 {
            promotion = "=Q".to_string();
        }
    }

    // 消除歧义。同种棋子可能走到同一格时，加出发格的行或列。
    let mut ambiguous = vec![(from_x, from_y)];
    for fx in 0..BOARD_SIZE_I {
        for fy in 0..BOARD_SIZE_J {
            if (fx, fy) == (from_x, from_y) {
                continue;
            }
            let Some(p) = board.pieces[fx][fy] else {
                continue;
            };
            if p != *piece {
                continue;
            }
            if try_move(board, Step {
                from: (fx, fy),
                to: step.to,
            }).is_some() {
                ambiguous.push((fx, fy));
            }
        }
    }
    let disambiguate = if ambiguous.len() == 1 {
        // 唯一的位置，兵吃子的情况需要消除歧义，否则不需要消除歧义
        if capture && role == PieceRole::Pawn {
            format!("{}", (b'a' + from_x as u8) as char)
        } else {
            String::new()
        }
    } else {
        // 检查是否是唯一横坐标
        let unique_x = ambiguous.iter().filter(|&&(x, _)| x == from_x).count() == 1;
        if unique_x {
            // 使用出发格的横坐标 (a-h)
            format!("{}", (b'a' + from_x as u8) as char)
        } else {
            // 检查是否是唯一纵坐标
            let unique_y = ambiguous.iter().filter(|&&(_, y)| y == from_y).count() == 1;
            if unique_y {
                // 使用出发格的纵坐标 (1-8)
                format!("{}", from_y + 1)
            } else {
                // 都不是，需要完整坐标 (如 a1)
                format!("{}{}", (b'a' + from_x as u8) as char, from_y + 1)
            }
        }
    };

    Some(format!(
        "{}{}{}{}{}{}", 
        name,
        disambiguate,
        if capture { "x" } else { "" },
        target,
        promotion,
        check_string,
    ))
}