use axum::{
    extract::{Json, Query},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tower_http::services::ServeDir;

const WIN_LINES: [[usize; 3]; 8] = [
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
enum Cell {
    Empty,
    Blue,
    Red,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum GameStatus {
    BlueToMove,
    RedToMove,
    BlueWins,
    RedWins,
    Draw,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct GameState {
    cells: [[Cell; 9]; 9],
    board_winners: [Cell; 9],
    board_full: [bool; 9],
    required_board: Option<usize>,
    status: GameStatus,
    last_blue: Option<(usize, usize)>,
    last_red: Option<(usize, usize)>,
}

impl GameState {
    fn new() -> Self {
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

fn check_winner(cells: &[Cell; 9]) -> Cell {
    for line in &WIN_LINES {
        let a = cells[line[0]];
        if a != Cell::Empty && a == cells[line[1]] && a == cells[line[2]] {
            return a;
        }
    }
    Cell::Empty
}

fn is_board_full(cells: &[Cell; 9]) -> bool {
    cells.iter().all(|c| *c != Cell::Empty)
}

fn apply_move(state: &mut GameState, board_idx: usize, cell_idx: usize, player: Cell) {
    state.cells[board_idx][cell_idx] = player;

    // Only set board winner on first win (variant: play continues in won boards)
    if state.board_winners[board_idx] == Cell::Empty {
        let winner = check_winner(&state.cells[board_idx]);
        if winner != Cell::Empty {
            state.board_winners[board_idx] = winner;
        }
    }
    state.board_full[board_idx] = is_board_full(&state.cells[board_idx]);

    // Track last move
    match player {
        Cell::Blue => state.last_blue = Some((board_idx, cell_idx)),
        Cell::Red => state.last_red = Some((board_idx, cell_idx)),
        _ => {}
    }

    // Set required board for next move (only full boards trigger free choice)
    if state.board_full[cell_idx] {
        state.required_board = None;
    } else {
        state.required_board = Some(cell_idx);
    }

    // Check meta-board winner
    let meta_winner = check_winner(&state.board_winners);
    if meta_winner == Cell::Blue {
        state.status = GameStatus::BlueWins;
    } else if meta_winner == Cell::Red {
        state.status = GameStatus::RedWins;
    } else if state.board_winners.iter().zip(state.board_full.iter()).all(|(w, f)| *w != Cell::Empty || *f) {
        state.status = GameStatus::Draw;
    } else {
        state.status = match player {
            Cell::Blue => GameStatus::RedToMove,
            Cell::Red => GameStatus::BlueToMove,
            _ => state.status,
        };
    }
}

fn legal_moves(state: &GameState) -> Vec<(usize, usize)> {
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

// --- AI helper functions ---

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

fn pick_random(moves: &[(usize, usize)]) -> (usize, usize) {
    *moves.choose(&mut rand::thread_rng()).unwrap()
}

fn pick_move(state: &GameState, level: u32, moves: &[(usize, usize)]) -> (usize, usize) {
    // Level 0: avoid winning if possible (newbie-friendly)
    if level == 0 {
        let non_winning: Vec<_> = moves.iter()
            .filter(|&&(b, c)| !would_win_board(&state.cells[b], c, Cell::Red))
            .copied().collect();
        if !non_winning.is_empty() { return pick_random(&non_winning); }
        return pick_random(moves);
    }

    // Level 1: pure random
    if level == 1 {
        return pick_random(moves);
    }

    // Level 14+: two-ply minimax lookahead
    if level >= 14 {
        return best_move_two_ply(state, moves);
    }

    // Level 2+: win a small board if possible (skip already-won boards)
    let winning: Vec<_> = moves.iter()
        .filter(|&&(b, c)| {
            state.board_winners[b] == Cell::Empty
                && would_win_board(&state.cells[b], c, Cell::Red)
        })
        .copied().collect();
    if !winning.is_empty() {
        // Level 8+: prefer moves that win the meta-game
        if level >= 8 {
            let meta_win: Vec<_> = winning.iter()
                .filter(|&&(b, _)| would_win_meta(&state.board_winners, b, Cell::Red))
                .copied().collect();
            if !meta_win.is_empty() { return pick_random(&meta_win); }
        }
        // Level 11+: prefer moves that create meta-board threats
        if level >= 11 {
            let meta_threat: Vec<_> = winning.iter()
                .filter(|&&(b, _)| creates_meta_threat(&state.board_winners, b, Cell::Red))
                .copied().collect();
            if !meta_threat.is_empty() { return pick_random(&meta_threat); }
        }
        return pick_random(&winning);
    }

    // Level 3+: block opponent from winning a small board (skip already-won boards)
    if level >= 3 {
        let blocking: Vec<_> = moves.iter()
            .filter(|&&(b, c)| {
                state.board_winners[b] == Cell::Empty
                    && would_win_board(&state.cells[b], c, Cell::Blue)
            })
            .copied().collect();
        if !blocking.is_empty() {
            // Level 9+: prioritize blocking meta-game-winning moves
            if level >= 9 {
                let meta_block: Vec<_> = blocking.iter()
                    .filter(|&&(b, _)| would_win_meta(&state.board_winners, b, Cell::Blue))
                    .copied().collect();
                if !meta_block.is_empty() { return pick_random(&meta_block); }
            }
            return pick_random(&blocking);
        }
    }

    // Level 13+: create forks (two simultaneous winning threats)
    if level >= 13 {
        let forks: Vec<_> = moves.iter()
            .filter(|&&(b, c)| {
                state.board_winners[b] == Cell::Empty
                    && creates_fork(&state.cells[b], c, Cell::Red)
            })
            .copied().collect();
        if !forks.is_empty() { return pick_random(&forks); }
    }

    // Build candidate pool with destination filters
    let mut candidates = moves.to_vec();

    // Level 6+: avoid sending opponent where they can win a board in one move
    if level >= 6 {
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

    // Level 12+: avoid sending opponent to boards that advance their meta-strategy
    if level >= 12 {
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

    // Level 7+: trap opponent (send to board with 1 empty cell leading to our advantage)
    if level >= 7 {
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

    // Level 4+: prefer sending to an empty board
    if level >= 4 {
        let to_empty: Vec<_> = candidates.iter()
            .filter(|&&(_, c)| {
                !state.board_full[c] && state.cells[c].iter().all(|&cell| cell == Cell::Empty)
            })
            .copied().collect();
        if !to_empty.is_empty() {
            if level >= 10 {
                if let Some(m) = prefer_center(&to_empty) { return m; }
            }
            return pick_random(&to_empty);
        }
    }

    // Level 5+: send to the board with the most empty cells
    if level >= 5 {
        let empty_count = |c: usize| {
            if state.board_full[c] { 0 }
            else { state.cells[c].iter().filter(|&&cell| cell == Cell::Empty).count() }
        };
        let max_empty = candidates.iter().map(|&(_, c)| empty_count(c)).max().unwrap();
        let most_empty: Vec<_> = candidates.iter()
            .filter(|&&(_, c)| empty_count(c) == max_empty)
            .copied().collect();
        if level >= 10 {
            if let Some(m) = prefer_center(&most_empty) { return m; }
        }
        return pick_random(&most_empty);
    }

    // Level 10+: prefer center positions as final tiebreaker
    if level >= 10 {
        if let Some(m) = prefer_center(&candidates) { return m; }
    }

    pick_random(&candidates)
}

fn computer_move(state: &mut GameState, level: u32) {
    let moves = legal_moves(state);
    if moves.is_empty() { return; }
    let chosen = pick_move(state, level, &moves);
    apply_move(state, chosen.0, chosen.1, Cell::Red);
}

#[derive(Deserialize)]
struct MoveRequest {
    state: GameState,
    board_idx: usize,
    cell_idx: usize,
    #[serde(default)]
    level: u32,
}

#[derive(Serialize)]
struct MoveResponse {
    ok: bool,
    state: GameState,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

async fn handle_index(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    match params.get("v") {
        Some(v) if v.parse::<u64>().is_ok() => {
            let html = include_str!("../static/index.html").replace("{{VERSION}}", v);
            (
                [(header::CACHE_CONTROL, "no-store")],
                Html(html),
            ).into_response()
        }
        _ => {
            let url = format!("/?v={}", now_secs());
            Redirect::to(&url).into_response()
        }
    }
}

async fn handle_new() -> Json<GameState> {
    Json(GameState::new())
}

async fn handle_move(Json(req): Json<MoveRequest>) -> impl IntoResponse {
    let mut state = req.state;
    let board_idx = req.board_idx;
    let cell_idx = req.cell_idx;

    // Validate
    if board_idx >= 9 || cell_idx >= 9 {
        return (
            StatusCode::BAD_REQUEST,
            Json(MoveResponse {
                ok: false,
                state,
                error: Some("Invalid indices".into()),
            }),
        );
    }
    if state.status != GameStatus::BlueToMove {
        return (
            StatusCode::BAD_REQUEST,
            Json(MoveResponse {
                ok: false,
                state,
                error: Some("Not blue's turn".into()),
            }),
        );
    }
    if !legal_moves(&state).contains(&(board_idx, cell_idx)) {
        return (
            StatusCode::BAD_REQUEST,
            Json(MoveResponse {
                ok: false,
                state,
                error: Some("Illegal move".into()),
            }),
        );
    }

    // Apply blue's move
    apply_move(&mut state, board_idx, cell_idx, Cell::Blue);

    // If game isn't over, computer plays
    if state.status == GameStatus::RedToMove {
        computer_move(&mut state, req.level);
    }

    (
        StatusCode::OK,
        Json(MoveResponse {
            ok: true,
            state,
            error: None,
        }),
    )
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(handle_index))
        .route("/api/new", post(handle_new))
        .route("/api/move", post(handle_move))
        .fallback_service(ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}
