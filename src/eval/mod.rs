use chessing::{bitboard::BitInt, game::{Board, Team}};
use psqt::{BISHOP_EG, BISHOP_EG_WHITE, BISHOP_MG, BISHOP_MG_WHITE, KING_EG, KING_EG_WHITE, KING_MG, KING_MG_WHITE, KNIGHT_EG, KNIGHT_EG_WHITE, KNIGHT_MG, KNIGHT_MG_WHITE, PAWN_EG, PAWN_EG_WHITE, PAWN_MG, PAWN_MG_WHITE, QUEEN_EG, QUEEN_EG_WHITE, QUEEN_MG, QUEEN_MG_WHITE, ROOK_EG, ROOK_EG_WHITE, ROOK_MG, ROOK_MG_WHITE};

mod psqt;

pub fn team_to_move<T: BitInt>(board: &mut Board<T>) -> i32 {
    match board.state.moving_team {
        Team::White => 1,
        Team::Black => -1
    }
}

pub const PAWN: i32 = 100;
pub const KNIGHT: i32 = 305;
pub const BISHOP: i32 = 333;
pub const ROOK: i32 = 563;
pub const QUEEN: i32 = 950;

pub const MATERIAL: [ i32; 6 ] = [ PAWN, KNIGHT, BISHOP, ROOK, QUEEN, 0 ];

pub fn eval<T: BitInt>(board: &mut Board<T>) -> i32 {
    let mut score = 0;

    let pawns = board.state.pieces[0];
    let knights = board.state.pieces[1];
    let bishops = board.state.pieces[2];
    let rooks = board.state.pieces[3];
    let queens = board.state.pieces[4];
    let kings = board.state.pieces[5];

    let white = board.state.white;
    let black = board.state.black;

    let white_pawns = pawns.and(white);
    let black_pawns = pawns.and(black);

    let white_knights = knights.and(white);
    let black_knights = knights.and(black);

    let white_bishops = bishops.and(white);
    let black_bishops = bishops.and(black);

    let white_rooks = rooks.and(white);
    let black_rooks = rooks.and(black);

    let white_queens = queens.and(white);
    let black_queens = queens.and(black);

    let white_king = kings.and(white);
    let black_king = kings.and(black);

    let white_material = 
        (white_pawns.count() as i32 * PAWN) +
        (white_knights.count() as i32 * KNIGHT) +
        (white_bishops.count() as i32 * BISHOP) +
        (white_rooks.count() as i32 * ROOK) +
        (white_queens.count() as i32 * QUEEN);

    let black_material = 
        (black_pawns.count() as i32 * PAWN) +
        (black_knights.count() as i32 * KNIGHT) +
        (black_bishops.count() as i32 * BISHOP) +
        (black_rooks.count() as i32 * ROOK) +
        (black_queens.count() as i32 * QUEEN);

    score += white_material;
    score -= black_material;

    let total_material = white_material + black_material;
    let relevant_material = total_material - (pawns.count() as i32 * PAWN);

    if relevant_material > 3000 {
        // Middlegame

        for pawn in white_pawns.iter() {
            score += PAWN_MG_WHITE[pawn as usize];
        }
    
        for pawn in black_pawns.iter() {
            score -= PAWN_MG[pawn as usize];
        }
    
        for knight in white_knights.iter() {
            score += KNIGHT_MG_WHITE[knight as usize];
        }
    
        for knight in black_knights.iter() {
            score -= KNIGHT_MG[knight as usize];
        }
    
        for bishop in white_bishops.iter() {
            score += BISHOP_MG_WHITE[bishop as usize];
        }
    
        for bishop in black_bishops.iter() {
            score -= BISHOP_MG[bishop as usize];
        }
    
        for rook in white_rooks.iter() {
            score += ROOK_MG_WHITE[rook as usize];
        }
    
        for rook in black_rooks.iter() {
            score -= ROOK_MG[rook as usize];
        }
    
        for queen in white_queens.iter() {
            score += QUEEN_MG_WHITE[queen as usize];
        }
    
        for queen in black_queens.iter() {
            score -= QUEEN_MG[queen as usize];
        }
    
        for king in white_king.iter() {
            score += KING_MG_WHITE[king as usize];
        }
    
        for king in black_king.iter() {
            score -= KING_MG[king as usize];
        }
    } else {
        for pawn in white_pawns.iter() {
            score += PAWN_EG_WHITE[pawn as usize];
        }
    
        for pawn in black_pawns.iter() {
            score -= PAWN_EG[pawn as usize];
        }
    
        for knight in white_knights.iter() {
            score += KNIGHT_EG_WHITE[knight as usize];
        }
    
        for knight in black_knights.iter() {
            score -= KNIGHT_EG[knight as usize];
        }
    
        for bishop in white_bishops.iter() {
            score += BISHOP_EG_WHITE[bishop as usize];
        }
    
        for bishop in black_bishops.iter() {
            score -= BISHOP_EG[bishop as usize];
        }
    
        for rook in white_rooks.iter() {
            score += ROOK_EG_WHITE[rook as usize];
        }
    
        for rook in black_rooks.iter() {
            score -= ROOK_EG[rook as usize];
        }
    
        for queen in white_queens.iter() {
            score += QUEEN_EG_WHITE[queen as usize];
        }
    
        for queen in black_queens.iter() {
            score -= QUEEN_EG[queen as usize];
        }
    
        for king in white_king.iter() {
            score += KING_EG_WHITE[king as usize];
        }
    
        for king in black_king.iter() {
            score -= KING_EG[king as usize];
        }
    }

    score *= team_to_move(board);

    score
}