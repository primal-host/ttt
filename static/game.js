const WIN_LINES = [
  [0,1,2],[3,4,5],[6,7,8],
  [0,3,6],[1,4,7],[2,5,8],
  [0,4,8],[2,4,6]
];

let state = null;
let busy = false;

const metaBoard = document.getElementById("meta-board");
const statusEl = document.getElementById("status");
const newGameBtn = document.getElementById("new-game");

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
      if (st.board_winners[b] === "empty" && !st.board_full[b]) {
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
    if (state.board_winners[b] === "blue") boardEl.classList.add("won-blue");
    else if (state.board_winners[b] === "red") boardEl.classList.add("won-red");

    // Active board highlight
    if (isBluesTurn && state.required_board !== null && state.required_board !== undefined && state.required_board === b) {
      boardEl.classList.add("active");
    } else if (isBluesTurn && (state.required_board === null || state.required_board === undefined) &&
               state.board_winners[b] === "empty" && !state.board_full[b]) {
      boardEl.classList.add("active");
    }

    for (let c = 0; c < 9; c++) {
      const el = cells[b][c];
      el.className = "cell";

      if (state.cells[b][c] === "blue") el.classList.add("blue");
      else if (state.cells[b][c] === "red") el.classList.add("red");

      if (legal.has(`${b},${c}`)) el.classList.add("legal");

      // Last-move markers
      if (state.last_blue && state.last_blue[0] === b && state.last_blue[1] === c) {
        el.classList.add("last-blue");
        if (isBluesTurn) el.classList.add("current-turn");
      }
      if (state.last_red && state.last_red[0] === b && state.last_red[1] === c) {
        el.classList.add("last-red");
        if (!isBluesTurn && state.status === "redtomove") el.classList.add("current-turn");
      }
    }
  }

  // Status text
  statusEl.className = "";
  switch (state.status) {
    case "bluetomove":
      statusEl.textContent = "Your turn";
      statusEl.classList.add("blue");
      break;
    case "redtomove":
      statusEl.textContent = "Computer thinking...";
      statusEl.classList.add("red");
      break;
    case "bluewins":
      statusEl.textContent = "You win!";
      statusEl.classList.add("blue");
      break;
    case "redwins":
      statusEl.textContent = "Computer wins!";
      statusEl.classList.add("red");
      break;
    case "draw":
      statusEl.textContent = "Draw!";
      statusEl.classList.add("draw");
      break;
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
      body: JSON.stringify({ state, board_idx: b, cell_idx: c }),
    });
    const data = await resp.json();
    if (data.ok) {
      state = data.state;
      render();
    }
  } finally {
    busy = false;
  }
}

async function newGame() {
  busy = true;
  try {
    const resp = await fetch("/api/new", { method: "POST" });
    state = await resp.json();
    render();
  } finally {
    busy = false;
  }
}

newGameBtn.addEventListener("click", newGame);
buildBoard();
newGame();
