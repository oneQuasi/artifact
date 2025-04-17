use chessing::{bitboard::{BitBoard, BitInt}, game::{Board, Team}};
use psqt::{BISHOP_EG, BISHOP_EG_WHITE, BISHOP_MG, BISHOP_MG_WHITE, KING_EG, KING_EG_WHITE, KING_MG, KING_MG_WHITE, KNIGHT_EG, KNIGHT_EG_WHITE, KNIGHT_MG, KNIGHT_MG_WHITE, PAWN_EG, PAWN_EG_WHITE, PAWN_MG, PAWN_MG_WHITE, QUEEN_EG, QUEEN_EG_WHITE, QUEEN_MG, QUEEN_MG_WHITE, ROOK_EG, ROOK_EG_WHITE, ROOK_MG, ROOK_MG_WHITE};

use crate::search::SearchInfo;

mod psqt;

pub fn team_to_move<T: BitInt, const N: usize>(board: &mut Board<T, N>) -> i32 {
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

pub const MOBILITY: i32 = 3;

pub const MATERIAL: [ i32; 6 ] = [ PAWN, KNIGHT, BISHOP, ROOK, QUEEN, 0 ];

// For use in training neural nets on new variants
pub fn eval_primitive<T: BitInt, const N: usize>(
    board: &mut Board<T, N>,
    info: &mut SearchInfo,
    ply: usize
) -> i32 {
    let mut score = 0;

    score += 100 * board.state.white.count() as i32;
    score -= 100 * board.state.black.count() as i32;

    let mut white_mobility = 0;
    let mut black_mobility = 0;

    for ply in (0..ply).rev() {
        if white_mobility > 0 && black_mobility > 0 { break; }
        match info.mobility[ply] {
            Some((mobility, team)) => {
                match team {
                    Team::White => {
                        if white_mobility == 0 { white_mobility = mobility };
                    }
                    Team::Black => {
                        if black_mobility == 0 { black_mobility = mobility; }
                    }
                }
            }
            None => {}
        }
    } 

    let mobility_bonus = MOBILITY * ((white_mobility as i32)  - (black_mobility as i32));
    score += mobility_bonus;

    score * team_to_move(board)
}

pub fn eval<T: BitInt, const N: usize>(
    board: &mut Board<T, N>,
    info: &mut SearchInfo,
    ply: usize
) -> i32 {
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

    score += white_material - black_material;

    let total_material = white_material + black_material;

    if total_material > 5000 {
        score += compute_mg(
            white_pawns, black_pawns,
            white_knights, black_knights,
            white_bishops, black_bishops,
            white_rooks, black_rooks,
            white_queens, black_queens,
            white_king, black_king
        );
    } else if total_material < 2500 {
        score += compute_eg(
            white_pawns, black_pawns,
            white_knights, black_knights,
            white_bishops, black_bishops,
            white_rooks, black_rooks,
            white_queens, black_queens,
            white_king, black_king
        );
    } else {
        let mg = compute_mg(
            white_pawns, black_pawns,
            white_knights, black_knights,
            white_bishops, black_bishops,
            white_rooks, black_rooks,
            white_queens, black_queens,
            white_king, black_king
        );
        let eg = compute_eg(
            white_pawns, black_pawns,
            white_knights, black_knights,
            white_bishops, black_bishops,
            white_rooks, black_rooks,
            white_queens, black_queens,
            white_king, black_king
        );
        let weight = total_material - 2500;
        score += (mg * weight + eg * (2500 - weight)) / 2500;
    }

    let mut white_mobility = 0;
    let mut black_mobility = 0;

    for ply in (0..ply).rev() {
        if white_mobility > 0 && black_mobility > 0 { break; }
        match info.mobility[ply] {
            Some((mobility, team)) => {
                match team {
                    Team::White => {
                        if white_mobility == 0 { white_mobility = mobility };
                    }
                    Team::Black => {
                        if black_mobility == 0 { black_mobility = mobility; }
                    }
                }
            }
            None => {}
        }
    } 

    let mobility_bonus = MOBILITY * ((white_mobility as i32)  - (black_mobility as i32));
    score += mobility_bonus;

    score * team_to_move(board)
}

fn compute_mg<T: BitInt>(
    wp: BitBoard<T>, bp: BitBoard<T>,
    wn: BitBoard<T>, bn: BitBoard<T>,
    wb: BitBoard<T>, bb: BitBoard<T>,
    wr: BitBoard<T>, br: BitBoard<T>,
    wq: BitBoard<T>, bq: BitBoard<T>,
    wk: BitBoard<T>, bk: BitBoard<T>
) -> i32 {
    let mut mg = 0;

    for sq in wp.iter() { mg += PAWN_MG_WHITE[sq as usize]; }
    for sq in bp.iter() { mg -= PAWN_MG[sq as usize]; }
    for sq in wn.iter() { mg += KNIGHT_MG_WHITE[sq as usize]; }
    for sq in bn.iter() { mg -= KNIGHT_MG[sq as usize]; }
    for sq in wb.iter() { mg += BISHOP_MG_WHITE[sq as usize]; }
    for sq in bb.iter() { mg -= BISHOP_MG[sq as usize]; }
    for sq in wr.iter() { mg += ROOK_MG_WHITE[sq as usize]; }
    for sq in br.iter() { mg -= ROOK_MG[sq as usize]; }
    for sq in wq.iter() { mg += QUEEN_MG_WHITE[sq as usize]; }
    for sq in bq.iter() { mg -= QUEEN_MG[sq as usize]; }
    for sq in wk.iter() { mg += KING_MG_WHITE[sq as usize]; }
    for sq in bk.iter() { mg -= KING_MG[sq as usize]; }

    mg
}

fn compute_eg<T: BitInt>(
    wp: BitBoard<T>, bp: BitBoard<T>,
    wn: BitBoard<T>, bn: BitBoard<T>,
    wb: BitBoard<T>, bb: BitBoard<T>,
    wr: BitBoard<T>, br: BitBoard<T>,
    wq: BitBoard<T>, bq: BitBoard<T>,
    wk: BitBoard<T>, bk: BitBoard<T>
) -> i32 {
    let mut eg = 0;

    for sq in wp.iter() { eg += PAWN_EG_WHITE[sq as usize]; }
    for sq in bp.iter() { eg -= PAWN_EG[sq as usize]; }
    for sq in wn.iter() { eg += KNIGHT_EG_WHITE[sq as usize]; }
    for sq in bn.iter() { eg -= KNIGHT_EG[sq as usize]; }
    for sq in wb.iter() { eg += BISHOP_EG_WHITE[sq as usize]; }
    for sq in bb.iter() { eg -= BISHOP_EG[sq as usize]; }
    for sq in wr.iter() { eg += ROOK_EG_WHITE[sq as usize]; }
    for sq in br.iter() { eg -= ROOK_EG[sq as usize]; }
    for sq in wq.iter() { eg += QUEEN_EG_WHITE[sq as usize]; }
    for sq in bq.iter() { eg -= QUEEN_EG[sq as usize]; }
    for sq in wk.iter() { eg += KING_EG_WHITE[sq as usize]; }
    for sq in bk.iter() { eg -= KING_EG[sq as usize]; }

    eg
}
