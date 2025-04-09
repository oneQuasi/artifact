use chessing::{bitboard::BitInt, game::{Board, Team}};

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

    let white = board.state.white;
    let black = board.state.black;

    let white_pawns = pawns.and(white).count() as i32;
    let black_pawns = pawns.and(black).count() as i32;

    let white_knights = knights.and(white).count() as i32;
    let black_knights = knights.and(black).count() as i32;

    let white_bishops = bishops.and(white).count() as i32;
    let black_bishops = bishops.and(black).count() as i32;

    let white_rooks = rooks.and(white).count() as i32;
    let black_rooks = rooks.and(black).count() as i32;

    let white_queens = queens.and(white).count() as i32;
    let black_queens = queens.and(black).count() as i32;

    score += (white_pawns - black_pawns) * PAWN;
    score += (white_knights - black_knights) * KNIGHT;
    score += (white_bishops - black_bishops) * BISHOP;
    score += (white_rooks - black_rooks) * ROOK;
    score += (white_queens - black_queens) * QUEEN;

    score *= team_to_move(board);

    score
}