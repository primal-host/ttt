const WIN_LINES = [
  [0,1,2],[3,4,5],[6,7,8],
  [0,3,6],[1,4,7],[2,5,8],
  [0,4,8],[2,4,6]
];

let state = null;
let busy = false;
let gameRecorded = false;

// Persistent difficulty state
let level = parseInt(localStorage.getItem("ttt_level") || "0");
let history = JSON.parse(localStorage.getItem("ttt_history") || "[]");
if (level < 0) level = 0;

function saveProgress() {
  localStorage.setItem("ttt_level", level);
  localStorage.setItem("ttt_history", JSON.stringify(history));
}

function saveGameState() {
  if (state) {
    localStorage.setItem("ttt_game", JSON.stringify(state));
    localStorage.setItem("ttt_game_recorded", gameRecorded ? "1" : "0");
  }
}

function clearGameState() {
  localStorage.removeItem("ttt_game");
  localStorage.removeItem("ttt_game_recorded");
}

function recordResult(winner) {
  history.push(winner);
  if (history.length > 2) history = history.slice(-2);

  if (history.length === 2 && history.every(r => r === "blue")) {
    level++;
    history = [];
  } else if (history.length === 2 && history.every(r => r === "red")) {
    if (level > 0) level--;
    history = [];
  }
  saveProgress();
}

const metaBoard = document.getElementById("meta-board");
const newGameBtn = document.getElementById("new-game");
const levelEl = document.getElementById("level");

function updateLevelDisplay() {
  if (levelEl) levelEl.textContent = "Level " + level;
}

// Build DOM once
const cells = []; // cells[board][cell] = element
function buildBoard() {
  metaBoard.innerHTML = "";
  for (let b = 0; b < 9; b++) {
    const boardEl = document.createElement("div");
    boardEl.className = "small-board";
    boardEl.dataset.board = b;
    cells[b] = [];
    for (let c = 0; c < 9; c++) {
      const cellEl = document.createElement("div");
      cellEl.className = "cell";
      cellEl.dataset.board = b;
      cellEl.dataset.cell = c;
      cellEl.addEventListener("click", onCellClick);
      boardEl.appendChild(cellEl);
      cells[b][c] = cellEl;
    }
    metaBoard.appendChild(boardEl);
  }
}

function getLegalMoves(st) {
  const moves = [];
  let boards;
  if (st.required_board !== null && st.required_board !== undefined) {
    boards = [st.required_board];
  } else {
    boards = [];
    for (let b = 0; b < 9; b++) {
      if (!st.board_full[b]) {
        boards.push(b);
      }
    }
  }
  for (const b of boards) {
    for (let c = 0; c < 9; c++) {
      if (st.cells[b][c] === "empty") {
        moves.push([b, c]);
      }
    }
  }
  return moves;
}

function findWinLine(boardCells) {
  for (const line of WIN_LINES) {
    const a = boardCells[line[0]];
    if (a !== "empty" && a === boardCells[line[1]] && a === boardCells[line[2]]) {
      return line;
    }
  }
  return null;
}

function render() {
  if (!state) return;

  const legal = state.status === "bluetomove"
    ? new Set(getLegalMoves(state).map(([b,c]) => `${b},${c}`))
    : new Set();

  const isBluesTurn = state.status === "bluetomove";

  for (let b = 0; b < 9; b++) {
    const boardEl = cells[b][0].parentElement;

    // Board-level classes
    boardEl.className = "small-board";

    // Find winning line for this board
    const winLine = (state.board_winners[b] !== "empty") ? findWinLine(state.cells[b]) : null;
    const winClass = state.board_winners[b] === "blue" ? "win-blue" : state.board_winners[b] === "red" ? "win-red" : null;

    // Active board highlight
    if (isBluesTurn && state.required_board !== null && state.required_board !== undefined && state.required_board === b) {
      boardEl.classList.add("active");
    } else if (isBluesTurn && (state.required_board === null || state.required_board === undefined) &&
               !state.board_full[b]) {
      boardEl.classList.add("active");
    }

    for (let c = 0; c < 9; c++) {
      const el = cells[b][c];
      el.className = "cell";

      if (state.cells[b][c] === "blue") el.classList.add("blue");
      else if (state.cells[b][c] === "red") el.classList.add("red");

      if (legal.has(`${b},${c}`)) el.classList.add("legal");

      // Win-line markers
      if (winLine && winClass && winLine.includes(c)) {
        el.classList.add(winClass);
      }

      // Last-move markers for both players
      if (state.last_blue && state.last_blue[0] === b && state.last_blue[1] === c) {
        el.classList.add("last-move");
      }
      if (state.last_red && state.last_red[0] === b && state.last_red[1] === c) {
        el.classList.add("last-move");
      }
    }
  }

  // Game over: show new game button
  const gameOver = state.status === "bluewins" || state.status === "redwins" || state.status === "draw";
  newGameBtn.classList.toggle("hidden", !gameOver);
  if (gameOver && !gameRecorded) {
    gameRecorded = true;
    const winner = state.status === "bluewins" ? "blue" : state.status === "redwins" ? "red" : "draw";
    recordResult(winner);
    updateLevelDisplay();
  }
}

async function onCellClick(e) {
  if (busy || !state || state.status !== "bluetomove") return;
  const b = parseInt(e.target.dataset.board);
  const c = parseInt(e.target.dataset.cell);

  // Client-side legality check
  const legal = getLegalMoves(state);
  if (!legal.some(([lb, lc]) => lb === b && lc === c)) return;

  busy = true;
  try {
    const resp = await fetch("/api/move", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ state, board_idx: b, cell_idx: c, level }),
    });
    const data = await resp.json();
    if (data.ok) {
      state = data.state;
      render();
      saveGameState();
    }
  } finally {
    busy = false;
  }
}

function newGame() {
  clearGameState();
  gameRecorded = false;
  window.location = "/?v=" + Math.floor(Date.now() / 1000);
}

async function initGame() {
  const saved = localStorage.getItem("ttt_game");
  if (saved) {
    state = JSON.parse(saved);
    gameRecorded = localStorage.getItem("ttt_game_recorded") === "1";
    updateLevelDisplay();
    render();
    return;
  }
  busy = true;
  try {
    const resp = await fetch("/api/new", { method: "POST" });
    state = await resp.json();
    saveGameState();
    updateLevelDisplay();
    render();
  } finally {
    busy = false;
  }
}

newGameBtn.addEventListener("click", newGame);
buildBoard();
initGame();
