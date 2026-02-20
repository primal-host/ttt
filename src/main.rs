use axum::{extract::Json, http::StatusCode, response::IntoResponse, routing::post, Router};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
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

    // Check if this board is now won or full
    let winner = check_winner(&state.cells[board_idx]);
    if winner != Cell::Empty {
        state.board_winners[board_idx] = winner;
    }
    state.board_full[board_idx] = is_board_full(&state.cells[board_idx]);

    // Track last move
    match player {
        Cell::Blue => state.last_blue = Some((board_idx, cell_idx)),
        Cell::Red => state.last_red = Some((board_idx, cell_idx)),
        _ => {}
    }

    // Set required board for next move
    if state.board_winners[cell_idx] != Cell::Empty || state.board_full[cell_idx] {
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
            .filter(|&b| state.board_winners[b] == Cell::Empty && !state.board_full[b])
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

fn computer_move(state: &mut GameState) {
    let moves = legal_moves(state);
    if moves.is_empty() {
        return;
    }
    let &(b, c) = moves.choose(&mut rand::thread_rng()).unwrap();
    apply_move(state, b, c, Cell::Red);
}

#[derive(Deserialize)]
struct MoveRequest {
    state: GameState,
    board_idx: usize,
    cell_idx: usize,
}

#[derive(Serialize)]
struct MoveResponse {
    ok: bool,
    state: GameState,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
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
        computer_move(&mut state);
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
        .route("/api/new", post(handle_new))
        .route("/api/move", post(handle_move))
        .fallback_service(ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}
