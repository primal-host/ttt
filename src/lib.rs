use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub const WIN_LINES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cell {
    Empty,
    Blue,
    Red,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameStatus {
    BlueToMove,
    RedToMove,
    BlueWins,
    RedWins,
    Draw,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub cells: [[Cell; 9]; 9],
    pub board_winners: [Cell; 9],
    pub board_full: [bool; 9],
    pub required_board: Option<usize>,
    pub status: GameStatus,
    pub last_blue: Option<(usize, usize)>,
    pub last_red: Option<(usize, usize)>,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            cells: [[Cell::Empty; 9]; 9],
            board_winners: [Cell::Empty; 9],
            board_full: [false; 9],
            required_board: None,
            status: GameStatus::BlueToMove,
            last_blue: None,
            last_red: None,
        }
    }
}

#[derive(Serialize)]
pub struct MoveResponse {
    pub ok: bool,
    pub state: GameState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct HintResponse {
    pub board_idx: usize,
    pub cell_idx: usize,
    pub explanation: String,
}

pub fn check_winner(cells: &[Cell; 9]) -> Cell {
    for line in &WIN_LINES {
        let a = cells[line[0]];
        if a != Cell::Empty && a == cells[line[1]] && a == cells[line[2]] {
            return a;
        }
    }
    Cell::Empty
}

pub fn is_board_full(cells: &[Cell; 9]) -> bool {
    cells.iter().all(|c| *c != Cell::Empty)
}

fn is_meta_dead(board_winners: &[Cell; 9]) -> bool {
    WIN_LINES.iter().all(|line| {
        let has_blue = line.iter().any(|&i| board_winners[i] == Cell::Blue);
        let has_red = line.iter().any(|&i| board_winners[i] == Cell::Red);
        has_blue && has_red
    })
}

pub fn apply_move(state: &mut GameState, board_idx: usize, cell_idx: usize, player: Cell) {
    state.cells[board_idx][cell_idx] = player;

    if state.board_winners[board_idx] == Cell::Empty {
        let winner = check_winner(&state.cells[board_idx]);
        if winner != Cell::Empty {
            state.board_winners[board_idx] = winner;
        }
    }
    state.board_full[board_idx] = is_board_full(&state.cells[board_idx]);

    match player {
        Cell::Blue => state.last_blue = Some((board_idx, cell_idx)),
        Cell::Red => state.last_red = Some((board_idx, cell_idx)),
        _ => {}
    }

    if state.board_full[cell_idx] {
        state.required_board = None;
    } else {
        state.required_board = Some(cell_idx);
    }

    let meta_winner = check_winner(&state.board_winners);
    if meta_winner == Cell::Blue {
        state.status = GameStatus::BlueWins;
    } else if meta_winner == Cell::Red {
        state.status = GameStatus::RedWins;
    } else if is_meta_dead(&state.board_winners)
        || state.board_winners.iter().zip(state.board_full.iter()).all(|(w, f)| *w != Cell::Empty || *f) {
        state.status = GameStatus::Draw;
    } else {
        state.status = match player {
            Cell::Blue => GameStatus::RedToMove,
            Cell::Red => GameStatus::BlueToMove,
            _ => state.status,
        };
    }
}

pub fn legal_moves(state: &GameState) -> Vec<(usize, usize)> {
    let mut moves = Vec::new();
    let boards: Vec<usize> = match state.required_board {
        Some(b) => vec![b],
        None => (0..9)
            .filter(|&b| !state.board_full[b])
            .collect(),
    };
    for b in boards {
        for c in 0..9 {
            if state.cells[b][c] == Cell::Empty {
                moves.push((b, c));
            }
        }
    }
    moves
}

fn would_win_board(cells: &[Cell; 9], cell_idx: usize, player: Cell) -> bool {
    let mut test = *cells;
    test[cell_idx] = player;
    check_winner(&test) == player
}

fn would_win_meta(board_winners: &[Cell; 9], board_idx: usize, player: Cell) -> bool {
    let mut test = *board_winners;
    test[board_idx] = player;
    check_winner(&test) == player
}

fn creates_meta_threat(board_winners: &[Cell; 9], board_idx: usize, player: Cell) -> bool {
    let mut test = *board_winners;
    test[board_idx] = player;
    for line in &WIN_LINES {
        if !line.contains(&board_idx) { continue; }
        let count = line.iter().filter(|&&i| test[i] == player).count();
        let empty = line.iter().filter(|&&i| test[i] == Cell::Empty).count();
        if count == 2 && empty == 1 {
            return true;
        }
    }
    false
}

fn creates_fork(cells: &[Cell; 9], cell_idx: usize, player: Cell) -> bool {
    let mut test = *cells;
    test[cell_idx] = player;
    let threats = (0..9).filter(|&i| {
        test[i] == Cell::Empty && would_win_board(&test, i, player)
    }).count();
    threats >= 2
}

fn evaluate(state: &GameState) -> i32 {
    let meta_w = check_winner(&state.board_winners);
    if meta_w == Cell::Red { return 10000; }
    if meta_w == Cell::Blue { return -10000; }
    let mut score = 0i32;
    for b in 0..9 {
        match state.board_winners[b] {
            Cell::Red => score += 100,
            Cell::Blue => score -= 100,
            _ => {}
        }
    }
    for line in &WIN_LINES {
        let red = line.iter().filter(|&&i| state.board_winners[i] == Cell::Red).count();
        let blue = line.iter().filter(|&&i| state.board_winners[i] == Cell::Blue).count();
        if blue == 0 {
            if red == 2 { score += 50; }
            else if red == 1 { score += 10; }
        }
        if red == 0 {
            if blue == 2 { score -= 50; }
            else if blue == 1 { score -= 10; }
        }
    }
    if state.board_winners[4] == Cell::Red { score += 5; }
    else if state.board_winners[4] == Cell::Blue { score -= 5; }
    for b in 0..9 {
        if state.board_winners[b] == Cell::Empty {
            if state.cells[b][4] == Cell::Red { score += 1; }
            else if state.cells[b][4] == Cell::Blue { score -= 1; }
        }
    }
    score
}

fn best_move_two_ply(state: &GameState, moves: &[(usize, usize)]) -> (usize, usize) {
    let mut best_score = i32::MIN;
    let mut best_moves = Vec::new();
    for &(b, c) in moves {
        let mut s1 = state.clone();
        apply_move(&mut s1, b, c, Cell::Red);
        let score = if s1.status != GameStatus::BlueToMove {
            evaluate(&s1)
        } else {
            let blue_moves = legal_moves(&s1);
            blue_moves.iter().map(|&(bb, bc)| {
                let mut s2 = s1.clone();
                apply_move(&mut s2, bb, bc, Cell::Blue);
                evaluate(&s2)
            }).min().unwrap_or(evaluate(&s1))
        };
        if score > best_score {
            best_score = score;
            best_moves = vec![(b, c)];
        } else if score == best_score {
            best_moves.push((b, c));
        }
    }
    *best_moves.choose(&mut rand::thread_rng()).unwrap()
}

fn best_move_one_ply(state: &GameState, moves: &[(usize, usize)]) -> (usize, usize) {
    let mut best_score = i32::MIN;
    let mut best_moves = Vec::new();
    for &(b, c) in moves {
        let mut s = state.clone();
        apply_move(&mut s, b, c, Cell::Red);
        let score = evaluate(&s);
        if score > best_score {
            best_score = score;
            best_moves = vec![(b, c)];
        } else if score == best_score {
            best_moves.push((b, c));
        }
    }
    *best_moves.choose(&mut rand::thread_rng()).unwrap()
}

fn prefer_center(moves: &[(usize, usize)]) -> Option<(usize, usize)> {
    let center_cell: Vec<_> = moves.iter().filter(|&&(_, c)| c == 4).copied().collect();
    if !center_cell.is_empty() {
        let center_both: Vec<_> = center_cell.iter().filter(|&&(b, _)| b == 4).copied().collect();
        if !center_both.is_empty() {
            return Some(*center_both.choose(&mut rand::thread_rng()).unwrap());
        }
        return Some(*center_cell.choose(&mut rand::thread_rng()).unwrap());
    }
    let center_board: Vec<_> = moves.iter().filter(|&&(b, _)| b == 4).copied().collect();
    if !center_board.is_empty() {
        return Some(*center_board.choose(&mut rand::thread_rng()).unwrap());
    }
    None
}

fn prefer_corners(moves: &[(usize, usize)]) -> Option<(usize, usize)> {
    let corners = [0, 2, 6, 8];
    let corner_cell: Vec<_> = moves.iter().filter(|&&(_, c)| corners.contains(&c)).copied().collect();
    if !corner_cell.is_empty() {
        let corner_both: Vec<_> = corner_cell.iter().filter(|&&(b, _)| corners.contains(&b)).copied().collect();
        if !corner_both.is_empty() {
            return Some(*corner_both.choose(&mut rand::thread_rng()).unwrap());
        }
        return Some(*corner_cell.choose(&mut rand::thread_rng()).unwrap());
    }
    let corner_board: Vec<_> = moves.iter().filter(|&&(b, _)| corners.contains(&b)).copied().collect();
    if !corner_board.is_empty() {
        return Some(*corner_board.choose(&mut rand::thread_rng()).unwrap());
    }
    None
}

fn pick_random(moves: &[(usize, usize)]) -> (usize, usize) {
    *moves.choose(&mut rand::thread_rng()).unwrap()
}

fn pick_move(state: &GameState, level: u32, moves: &[(usize, usize)]) -> (usize, usize) {
    if level == 0 {
        let non_winning: Vec<_> = moves.iter()
            .filter(|&&(b, c)| !would_win_board(&state.cells[b], c, Cell::Red))
            .copied().collect();
        if !non_winning.is_empty() { return pick_random(&non_winning); }
        return pick_random(moves);
    }

    if level == 1 {
        return pick_random(moves);
    }

    if level >= 21 {
        return best_move_two_ply(state, moves);
    }

    if level >= 20 {
        return best_move_one_ply(state, moves);
    }

    let winning: Vec<_> = moves.iter()
        .filter(|&&(b, c)| {
            state.board_winners[b] == Cell::Empty
                && would_win_board(&state.cells[b], c, Cell::Red)
        })
        .copied().collect();
    if !winning.is_empty() {
        if level >= 12 {
            let meta_win: Vec<_> = winning.iter()
                .filter(|&&(b, _)| would_win_meta(&state.board_winners, b, Cell::Red))
                .copied().collect();
            if !meta_win.is_empty() { return pick_random(&meta_win); }
        }
        if level >= 11 {
            let defensive: Vec<_> = winning.iter()
                .filter(|&&(b, _)| {
                    state.cells[b].iter().enumerate().any(|(i, &cell)| {
                        cell == Cell::Empty && would_win_board(&state.cells[b], i, Cell::Blue)
                    })
                })
                .copied().collect();
            if !defensive.is_empty() { return pick_random(&defensive); }
        }
        if level >= 16 {
            let meta_threat: Vec<_> = winning.iter()
                .filter(|&&(b, _)| creates_meta_threat(&state.board_winners, b, Cell::Red))
                .copied().collect();
            if !meta_threat.is_empty() { return pick_random(&meta_threat); }
        }
        return pick_random(&winning);
    }

    if level >= 3 {
        let blocking: Vec<_> = moves.iter()
            .filter(|&&(b, c)| {
                state.board_winners[b] == Cell::Empty
                    && would_win_board(&state.cells[b], c, Cell::Blue)
            })
            .copied().collect();
        if !blocking.is_empty() {
            if level >= 13 {
                let meta_block: Vec<_> = blocking.iter()
                    .filter(|&&(b, _)| would_win_meta(&state.board_winners, b, Cell::Blue))
                    .copied().collect();
                if !meta_block.is_empty() { return pick_random(&meta_block); }
            }
            return pick_random(&blocking);
        }
    }

    if level >= 4 {
        let block_forks: Vec<_> = moves.iter()
            .filter(|&&(b, c)| {
                state.board_winners[b] == Cell::Empty
                    && creates_fork(&state.cells[b], c, Cell::Blue)
            })
            .copied().collect();
        if !block_forks.is_empty() { return pick_random(&block_forks); }
    }

    if level >= 19 {
        let forks: Vec<_> = moves.iter()
            .filter(|&&(b, c)| {
                state.board_winners[b] == Cell::Empty
                    && creates_fork(&state.cells[b], c, Cell::Red)
            })
            .copied().collect();
        if !forks.is_empty() { return pick_random(&forks); }
    }

    let mut candidates = moves.to_vec();

    if level >= 7 {
        let safer: Vec<_> = candidates.iter()
            .filter(|&&(_, c)| {
                state.board_full[c]
                    || state.board_winners[c] != Cell::Empty
                    || {
                        let blue = state.cells[c].iter().filter(|&&cell| cell == Cell::Blue).count();
                        let red = state.cells[c].iter().filter(|&&cell| cell == Cell::Red).count();
                        blue <= red
                    }
            })
            .copied().collect();
        if !safer.is_empty() { candidates = safer; }
    }

    if level >= 8 {
        let safe: Vec<_> = candidates.iter()
            .filter(|&&(_, c)| {
                state.board_full[c]
                    || state.board_winners[c] != Cell::Empty
                    || !state.cells[c].iter().enumerate().any(|(i, &cell)| {
                        cell == Cell::Empty && would_win_board(&state.cells[c], i, Cell::Blue)
                    })
            })
            .copied().collect();
        if !safe.is_empty() { candidates = safe; }
    }

    if level >= 9 {
        let safe_fork: Vec<_> = candidates.iter()
            .filter(|&&(_, c)| {
                state.board_full[c]
                    || state.board_winners[c] != Cell::Empty
                    || !state.cells[c].iter().enumerate().any(|(i, &cell)| {
                        cell == Cell::Empty && creates_fork(&state.cells[c], i, Cell::Blue)
                    })
            })
            .copied().collect();
        if !safe_fork.is_empty() { candidates = safe_fork; }
    }

    if level >= 17 {
        let protect: Vec<_> = candidates.iter()
            .filter(|&&(_, c)| {
                state.board_full[c]
                    || state.board_winners[c] != Cell::Empty
                    || !WIN_LINES.iter().any(|line| {
                        line.contains(&c)
                            && state.board_winners[c] == Cell::Empty
                            && line.iter().filter(|&&i| state.board_winners[i] == Cell::Red).count() == 2
                    })
            })
            .copied().collect();
        if !protect.is_empty() { candidates = protect; }
    }

    if level >= 18 {
        let safe_meta: Vec<_> = candidates.iter()
            .filter(|&&(_, c)| {
                state.board_full[c]
                    || state.board_winners[c] != Cell::Empty
                    || (!would_win_meta(&state.board_winners, c, Cell::Blue)
                        && !creates_meta_threat(&state.board_winners, c, Cell::Blue))
            })
            .copied().collect();
        if !safe_meta.is_empty() { candidates = safe_meta; }
    }

    if level >= 10 {
        let trap: Vec<_> = candidates.iter()
            .filter(|&&(_, c)| {
                if state.board_full[c] { return false; }
                let empties: Vec<usize> = state.cells[c].iter().enumerate()
                    .filter(|(_, &cell)| cell == Cell::Empty)
                    .map(|(i, _)| i)
                    .collect();
                empties.len() == 1 && {
                    let dest = empties[0];
                    state.cells[dest].iter().enumerate().any(|(i, &cell)| {
                        cell == Cell::Empty && would_win_board(&state.cells[dest], i, Cell::Red)
                    })
                }
            })
            .copied().collect();
        if !trap.is_empty() { return pick_random(&trap); }
    }

    if level >= 5 {
        let to_empty: Vec<_> = candidates.iter()
            .filter(|&&(_, c)| {
                !state.board_full[c] && state.cells[c].iter().all(|&cell| cell == Cell::Empty)
            })
            .copied().collect();
        if !to_empty.is_empty() {
            if level >= 14 {
                if let Some(m) = prefer_center(&to_empty) { return m; }
            }
            if level >= 15 {
                if let Some(m) = prefer_corners(&to_empty) { return m; }
            }
            return pick_random(&to_empty);
        }
    }

    if level >= 6 {
        let empty_count = |c: usize| {
            if state.board_full[c] { 0 }
            else { state.cells[c].iter().filter(|&&cell| cell == Cell::Empty).count() }
        };
        let max_empty = candidates.iter().map(|&(_, c)| empty_count(c)).max().unwrap();
        let most_empty: Vec<_> = candidates.iter()
            .filter(|&&(_, c)| empty_count(c) == max_empty)
            .copied().collect();
        if level >= 14 {
            if let Some(m) = prefer_center(&most_empty) { return m; }
        }
        if level >= 15 {
            if let Some(m) = prefer_corners(&most_empty) { return m; }
        }
        return pick_random(&most_empty);
    }

    if level >= 14 {
        if let Some(m) = prefer_center(&candidates) { return m; }
    }
    if level >= 15 {
        if let Some(m) = prefer_corners(&candidates) { return m; }
    }

    pick_random(&candidates)
}

pub fn best_move_for_blue(state: &GameState, moves: &[(usize, usize)]) -> (usize, usize) {
    let mut best_score = i32::MAX;
    let mut best_moves = Vec::new();
    for &(b, c) in moves {
        let mut s1 = state.clone();
        apply_move(&mut s1, b, c, Cell::Blue);
        let score = if s1.status != GameStatus::RedToMove {
            evaluate(&s1)
        } else {
            let red_moves = legal_moves(&s1);
            red_moves.iter().map(|&(rb, rc)| {
                let mut s2 = s1.clone();
                apply_move(&mut s2, rb, rc, Cell::Red);
                evaluate(&s2)
            }).max().unwrap_or(evaluate(&s1))
        };
        if score < best_score {
            best_score = score;
            best_moves = vec![(b, c)];
        } else if score == best_score {
            best_moves.push((b, c));
        }
    }
    best_moves[0]
}

fn generate_explanation(state: &GameState, board_idx: usize, cell_idx: usize) -> String {
    let wins_board = state.board_winners[board_idx] == Cell::Empty
        && would_win_board(&state.cells[board_idx], cell_idx, Cell::Blue);
    let wins_meta = wins_board
        && would_win_meta(&state.board_winners, board_idx, Cell::Blue);

    if wins_board && wins_meta {
        return "Wins the game!".into();
    }
    if wins_board && creates_meta_threat(&state.board_winners, board_idx, Cell::Blue) {
        return "Wins board and threatens the game".into();
    }
    if wins_board {
        return "Wins a board".into();
    }

    if state.board_winners[board_idx] == Cell::Empty
        && would_win_board(&state.cells[board_idx], cell_idx, Cell::Red)
    {
        return "Blocks red from winning a board".into();
    }

    if creates_meta_threat(&state.board_winners, board_idx, Cell::Blue) {
        return "Threatens to win the game".into();
    }

    if state.board_winners[board_idx] == Cell::Empty
        && creates_fork(&state.cells[board_idx], cell_idx, Cell::Blue)
    {
        return "Creates two ways to win a board".into();
    }

    if state.board_full[cell_idx] {
        return "Gives you a free choice next".into();
    }

    "Best positional move".into()
}

pub fn computer_move(state: &mut GameState, level: u32) {
    let moves = legal_moves(state);
    if moves.is_empty() { return; }
    let chosen = pick_move(state, level, &moves);
    apply_move(state, chosen.0, chosen.1, Cell::Red);
}

// --- WASM exports ---

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn wasm_new_game() -> JsValue {
    serde_wasm_bindgen::to_value(&GameState::new()).unwrap()
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn wasm_make_move(state_js: JsValue, board_idx: usize, cell_idx: usize, level: u32) -> JsValue {
    let mut state: GameState = serde_wasm_bindgen::from_value(state_js).unwrap();

    if board_idx >= 9 || cell_idx >= 9 {
        return serde_wasm_bindgen::to_value(&MoveResponse {
            ok: false,
            state,
            error: Some("Invalid indices".into()),
        }).unwrap();
    }
    if state.status != GameStatus::BlueToMove {
        return serde_wasm_bindgen::to_value(&MoveResponse {
            ok: false,
            state,
            error: Some("Not blue's turn".into()),
        }).unwrap();
    }
    if !legal_moves(&state).contains(&(board_idx, cell_idx)) {
        return serde_wasm_bindgen::to_value(&MoveResponse {
            ok: false,
            state,
            error: Some("Illegal move".into()),
        }).unwrap();
    }

    apply_move(&mut state, board_idx, cell_idx, Cell::Blue);

    if state.status == GameStatus::RedToMove {
        computer_move(&mut state, level);
    }

    serde_wasm_bindgen::to_value(&MoveResponse {
        ok: true,
        state,
        error: None,
    }).unwrap()
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn wasm_get_hint(state_js: JsValue) -> JsValue {
    let state: GameState = serde_wasm_bindgen::from_value(state_js).unwrap();

    if state.status != GameStatus::BlueToMove {
        return serde_wasm_bindgen::to_value(&HintResponse {
            board_idx: 0,
            cell_idx: 0,
            explanation: "Not blue's turn".into(),
        }).unwrap();
    }

    let moves = legal_moves(&state);
    if moves.is_empty() {
        return serde_wasm_bindgen::to_value(&HintResponse {
            board_idx: 0,
            cell_idx: 0,
            explanation: "No legal moves".into(),
        }).unwrap();
    }

    let (b, c) = best_move_for_blue(&state, &moves);
    let explanation = generate_explanation(&state, b, c);

    serde_wasm_bindgen::to_value(&HintResponse {
        board_idx: b,
        cell_idx: c,
        explanation,
    }).unwrap()
}
