use crate::piece::*;
use crate::board::*;
use regex::Regex;

pub const INITIAL_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn char_to_role(c: char) -> Option<PieceRole> {
    match c.to_ascii_lowercase() {
        'k' => Some(PieceRole::King),
        'q' => Some(PieceRole::Queen),
        'r' => Some(PieceRole::Rook),
        'b' => Some(PieceRole::Bishop),
        'n' => Some(PieceRole::Knight),
        'p' => Some(PieceRole::Pawn),
        _ => None,
    }
}

fn piece_to_char(piece: Option<Piece>) -> char {
    if let Some(p) = piece {
        let c = match p.piece_role {
            PieceRole::Pawn => 'p',
            PieceRole::Rook => 'r',
            PieceRole::Knight => 'n',
            PieceRole::Bishop => 'b',
            PieceRole::Queen => 'q',
            PieceRole::King => 'k',
        };
        match p.piece_color {
            PieceColor::White => c.to_ascii_uppercase(),
            PieceColor::Black => c,
        }
    } else {
        '1'
    }
}

pub fn read_fen(fen: String) -> Board {
    let mut parts = fen.split_whitespace();
    let piece_placement = parts.next().unwrap_or("");
    let active_color = parts.next().unwrap_or("w");
    let castling = parts.next().unwrap_or("KQkq");
    let en_passant = parts.next().unwrap_or("-");
    let halfmove = parts.next().unwrap_or("0");
    let fullmove = parts.next().unwrap_or("1");

    // 1. 解析棋子位置
    let mut board = Pieces::new();
    let mut row = 7;
    let mut col = 0;

    for c in piece_placement.chars() {
        match c {
            ' ' => break,
            '/' => {
                if row == 0 { break; }
                row -= 1;
                col = 0;
            }
            c if c.is_ascii_digit() => {
                let spaces = c.to_digit(10).unwrap() as usize;
                col += spaces;
            }
            c => {
                if let Some(role) = char_to_role(c) {
                    board[col][row] = Some(Piece {
                        piece_role: role,
                        piece_color: if c.is_ascii_lowercase() { PieceColor::Black } else { PieceColor::White },
                    });
                    col += 1;
                }
            }
        }
    }

    // 2. 解析当前行棋方
    let active_color = match active_color {
        "w" => PieceColor::White,
        "b" => PieceColor::Black,
        _ => PieceColor::White, // 默认白方
    };

    // 3. 解析易位权限
    let (white_kingside, white_queenside, black_kingside, black_queenside) = (
        castling.contains('K'),
        castling.contains('Q'),
        castling.contains('k'),
        castling.contains('q'),
    );

    // 4. 解析吃过路兵格
    let en_passant_target = if en_passant == "-" {
        None
    } else {
        let file = en_passant.chars().nth(0).unwrap() as usize - 'a' as usize;
        let rank = en_passant.chars().nth(1).unwrap().to_digit(10).unwrap() as usize - 1;
        Some((file, rank))
    };

    // 5. 解析半回合计数和总回合数
    let halfmove = halfmove.parse().unwrap_or(0);
    let fullmove = fullmove.parse().unwrap_or(1);

    Board {
        pieces: board,
        active_color,
        castling_availability: (white_kingside, white_queenside, black_kingside, black_queenside),
        en_passant_target,
        halfmove,
        fullmove,
    }
}

pub fn write_fen(board: Board) -> String {
    // 1. 生成棋子位置部分
    let piece_placement = (0..BOARD_SIZE_J).rev().map(|j| {
        (0..BOARD_SIZE_I).map(|i| {
            piece_to_char(board.pieces[i][j].clone())
        })
        .collect::<String>()
    })
    .collect::<Vec<_>>()
    .join("/");

    // 压缩连续空格（如 "1111" -> "4"）
    let re = Regex::new("1{2,}").unwrap();
    let piece_placement = re.replace_all(&piece_placement, |caps: &regex::Captures| {
        caps[0].len().to_string()
    }).to_string();

    // 2. 当前行棋方
    let active_color = match board.active_color {
        PieceColor::White => "w",
        PieceColor::Black => "b",
    };

    // 3. 易位权限
    let (wk, wq, bk, bq) = board.castling_availability;
    let castling = {
        let mut s = String::new();
        if wk { s.push('K'); }
        if wq { s.push('Q'); }
        if bk { s.push('k'); }
        if bq { s.push('q'); }
        if s.is_empty() { "-".to_string() } else { s }
    };

    // 4. 吃过路兵格
    let en_passant = match board.en_passant_target {
        Some((file, rank)) => {
            let file_char = (b'a' + file as u8) as char;
            let rank_char = (b'1' + rank as u8) as char;
            format!("{}{}", file_char, rank_char)
        }
        None => "-".to_string(),
    };

    // 5. 半回合计数和总回合数
    let halfmove = board.halfmove.to_string();
    let fullmove = board.fullmove.to_string();

    // 组合所有部分
    format!(
        "{} {} {} {} {} {}",
        piece_placement, active_color, castling, en_passant, halfmove, fullmove
    )
}