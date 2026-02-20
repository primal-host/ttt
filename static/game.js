const WIN_LINES = [
  [0,1,2],[3,4,5],[6,7,8],
  [0,3,6],[1,4,7],[2,5,8],
  [0,4,8],[2,4,6]
];

let state = null;
let busy = false;
let gameRecorded = false;
let currentPlayer = null;

// --- Player storage ---
// ttt_players = { "name": { level, history, game, recorded } }

function loadPlayers() {
  return JSON.parse(localStorage.getItem("ttt_players") || "{}");
}

function savePlayers(players) {
  localStorage.setItem("ttt_players", JSON.stringify(players));
}

function loadCurrentPlayerName() {
  return localStorage.getItem("ttt_current_player");
}

function saveCurrentPlayerName(name) {
  localStorage.setItem("ttt_current_player", name);
}

function loadPlayerData(name) {
  const players = loadPlayers();
  return players[name] || { level: 0, history: [], game: null, recorded: false };
}

function savePlayerData(name, data) {
  const players = loadPlayers();
  players[name] = data;
  savePlayers(players);
}

function getPlayerData() {
  if (!currentPlayer) return null;
  return loadPlayerData(currentPlayer);
}

function syncToPlayer() {
  const data = getPlayerData();
  if (!data) return;
  level = data.level;
  history = data.history || [];
  state = data.game;
  gameRecorded = data.recorded || false;
}

function syncFromPlayer() {
  if (!currentPlayer) return;
  savePlayerData(currentPlayer, {
    level,
    history,
    game: state,
    recorded: gameRecorded,
  });
}

// --- Game state ---
const MAX_LEVEL = 14;
let level = 0;
let history = [];

let prevBoardWinners = null;

// --- DOM refs ---
const gameView = document.getElementById("game-view");
const playersView = document.getElementById("players-view");
const metaBoard = document.getElementById("meta-board");
const newGameBtn = document.getElementById("new-game");
const levelEl = document.getElementById("level");
const playerNameEl = document.getElementById("player-name");
const infoImg = document.getElementById("info-img");

function refreshImage() {
  if (infoImg) infoImg.src = "https://picsum.photos/300/250?r=" + Math.random();
}
const playersList = document.getElementById("players-list");
const newPlayerBtn = document.getElementById("new-player-btn");

function updateLevelDisplay() {
  if (levelEl) levelEl.textContent = "Level " + level;
}

function updatePlayerNameDisplay() {
  if (playerNameEl) playerNameEl.textContent = currentPlayer || "";
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
  syncFromPlayer();
}

// --- Board ---
const cells = [];
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
      if (!st.board_full[b]) boards.push(b);
    }
  }
  for (const b of boards) {
    for (let c = 0; c < 9; c++) {
      if (st.cells[b][c] === "empty") moves.push([b, c]);
    }
  }
  return moves;
}

function findWinLine(boardCells) {
  for (const line of WIN_LINES) {
    const a = boardCells[line[0]];
    if (a !== "empty" && a === boardCells[line[1]] && a === boardCells[line[2]]) return line;
  }
  return null;
}

function isBoardDead(boardCells) {
  return WIN_LINES.every(line => {
    const hasBlue = line.some(i => boardCells[i] === "blue");
    const hasRed = line.some(i => boardCells[i] === "red");
    return hasBlue && hasRed;
  });
}

function render() {
  if (!state) return;

  const legal = state.status === "bluetomove"
    ? new Set(getLegalMoves(state).map(([b,c]) => `${b},${c}`))
    : new Set();

  const isBluesTurn = state.status === "bluetomove";
  const gameWon = state.status === "bluewins" || state.status === "redwins";
  const metaWinLine = gameWon ? findWinLine(state.board_winners) : null;

  for (let b = 0; b < 9; b++) {
    const boardEl = cells[b][0].parentElement;
    boardEl.className = "small-board";

    const winLine = (state.board_winners[b] !== "empty") ? findWinLine(state.cells[b]) : null;
    const dead = (state.board_winners[b] === "empty") && isBoardDead(state.cells[b]);

    if (isBluesTurn && state.required_board !== null && state.required_board !== undefined && state.required_board === b) {
      boardEl.classList.add("active");
    } else if (isBluesTurn && (state.required_board === null || state.required_board === undefined) && !state.board_full[b]) {
      boardEl.classList.add("active");
    }

    for (let c = 0; c < 9; c++) {
      const el = cells[b][c];
      el.className = "cell";

      if (state.cells[b][c] === "blue") el.classList.add("blue");
      else if (state.cells[b][c] === "red") el.classList.add("red");

      if (legal.has(`${b},${c}`)) el.classList.add("legal");

      if (metaWinLine && !metaWinLine.includes(b)) el.classList.add("dimmed");
      else if (winLine && !winLine.includes(c)) el.classList.add("dimmed");
      if (dead && state.cells[b][c] !== "empty") el.classList.add("dimmed");

      if (state.last_blue && state.last_blue[0] === b && state.last_blue[1] === c) el.classList.add("last-move");
      if (state.last_red && state.last_red[0] === b && state.last_red[1] === c) el.classList.add("last-move");

      if (state.status === "draw" && state.cells[b][c] === "empty") el.classList.add("draw-marker");
    }
  }

  // Check if any board was just won
  if (prevBoardWinners) {
    for (let i = 0; i < 9; i++) {
      if (state.board_winners[i] !== "empty" && prevBoardWinners[i] === "empty") {
        refreshImage();
        break;
      }
    }
  }
  prevBoardWinners = [...state.board_winners];

  const gameOver = state.status === "bluewins" || state.status === "redwins" || state.status === "draw";
  newGameBtn.classList.toggle("btn-hidden", !gameOver);
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
      syncFromPlayer();
    }
  } finally {
    busy = false;
  }
}

async function newGame() {
  gameRecorded = false;
  prevBoardWinners = null;
  refreshImage();
  busy = true;
  try {
    const resp = await fetch("/api/new", { method: "POST" });
    state = await resp.json();
    syncFromPlayer();
    updateLevelDisplay();
    render();
  } finally {
    busy = false;
  }
}

// --- Views ---
function showGameView() {
  gameView.classList.remove("hidden");
  playersView.classList.add("hidden");
}

function showPlayersView() {
  gameView.classList.add("hidden");
  playersView.classList.remove("hidden");
  renderPlayersList();
}

function renderPlayersList() {
  const players = loadPlayers();
  playersList.innerHTML = "";
  const names = Object.keys(players).sort((a, b) => a.localeCompare(b, undefined, { sensitivity: "base" }));
  for (const name of names) {
    const data = players[name];
    const row = document.createElement("div");
    row.className = "player-row";

    const nameSpan = document.createElement("span");
    nameSpan.className = "player-row-name";
    nameSpan.textContent = name;

    const infoSpan = document.createElement("span");
    infoSpan.className = "player-row-info";
    infoSpan.textContent = "Level " + (data.level || 0);

    row.appendChild(nameSpan);
    row.appendChild(infoSpan);
    row.addEventListener("click", () => selectPlayer(name));
    playersList.appendChild(row);
  }
}

function selectPlayer(name) {
  currentPlayer = name;
  saveCurrentPlayerName(name);
  syncToPlayer();
  prevBoardWinners = null;
  refreshImage();
  updatePlayerNameDisplay();
  updateLevelDisplay();
  showGameView();
  if (state) {
    gameRecorded = gameRecorded || false;
    render();
  } else {
    newGame();
  }
}

function promptNewPlayer() {
  const name = prompt("Enter your name:");
  if (!name || !name.trim()) return;
  const trimmed = name.trim();
  const players = loadPlayers();
  if (!players[trimmed]) {
    players[trimmed] = { level: 0, history: [], game: null, recorded: false };
    savePlayers(players);
  }
  selectPlayer(trimmed);
}

// --- Init ---
playerNameEl.addEventListener("click", showPlayersView);
newGameBtn.addEventListener("click", newGame);
newPlayerBtn.addEventListener("click", promptNewPlayer);
document.getElementById("level-count").textContent = MAX_LEVEL + 1;
buildBoard();

const savedName = loadCurrentPlayerName();
if (savedName && loadPlayers()[savedName]) {
  selectPlayer(savedName);
} else {
  showPlayersView();
}
