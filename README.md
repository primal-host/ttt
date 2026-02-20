# Ultimate Tic-Tac-Toe

Sources:
- https://mathwithbaddrawings.com/ultimate-tic-tac-toe-original-post/
- https://mathwithbaddrawings.com/2013/06/16/ultimate-tic-tac-toe/
- https://en.wikipedia.org/wiki/Ultimate_tic-tac-toe

## Rules

### Board Structure

A 3x3 grid of 3x3 tic-tac-toe boards — 9 small boards forming one large meta-board (81 squares total).

### Turns

Players alternate marking one small square per turn.

### The Core Rule

**Your opponent's move determines which board you must play on next.** The position of their mark within the small board maps to the corresponding board on the meta-grid. For example, if your opponent plays in the top-right square of any small board, you must play on the top-right board next.

### Winning a Small Board

Get three in a row (horizontally, vertically, or diagonally) on a small board to claim it. Once claimed, ownership does not change — even if the opponent later gets three in a row on that same board.

### Winning the Game

Win three small boards in a row (horizontally, vertically, or diagonally) on the meta-board.

### Special Cases

- **Sent to a won or full board:** If your opponent's move sends you to a board that is already won or completely full, you may play on any open board of your choice.
- **Tied small boards:** A small board with no remaining moves and no winner counts for neither player.
