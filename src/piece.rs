use bevy::prelude::*;
use std::fmt;

#[derive(Clone, Copy, PartialEq)]
pub enum PieceRole {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

impl fmt::Display for PieceRole {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let role = match self {
            PieceRole::Pawn => "Pawn",
            PieceRole::Rook => "Rook",
            PieceRole::Knight => "Knight",
            PieceRole::Bishop => "Bishop",
            PieceRole::Queen => "Queen",
            PieceRole::King => "King",
        };
        write!(f, "{}", role)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum PieceColor {
    White,
    Black,
}

impl PieceColor {
    pub fn flip(&self) -> PieceColor {
        match *self {
            PieceColor::White => PieceColor::Black,
            PieceColor::Black => PieceColor::White,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Piece {
    pub piece_role: PieceRole,
    pub piece_color: PieceColor,
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let color = match self.piece_color {
            PieceColor::White => "W",
            PieceColor::Black => "B",
        };
        write!(f, "{}{}", color, self.piece_role.to_string())
    }
}

pub const BOARD_SIZE_I: usize = 8;
pub const BOARD_SIZE_J: usize = 8;

#[derive(Deref, DerefMut, Clone)]
pub struct Pieces(Vec<Vec<Option<Piece>>>);

impl Pieces {
    pub fn new() -> Self {
        Pieces::with_size(BOARD_SIZE_I, BOARD_SIZE_J)
    }

    pub fn with_size(rows: usize, cols: usize) -> Self {
        let grid = (0..rows)
            .map(|_| (0..cols).map(|_| None).collect())
            .collect();
            
        Pieces(grid)
    }
}

impl Default for Pieces {
    fn default() -> Self {
        Pieces::new()
    }
}