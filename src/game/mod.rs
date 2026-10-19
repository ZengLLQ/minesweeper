use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

/// Minimum allowed board dimension for custom games in UI.
pub const MIN_CUSTOM_DIM: usize = 5;
/// Maximum allowed board width for custom games in UI.
pub const MAX_CUSTOM_WIDTH: usize = 50;
/// Maximum allowed board height for custom games in UI.
pub const MAX_CUSTOM_HEIGHT: usize = 35;

/// Difficulty presets and custom configuration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    #[default]
    Beginner,
    Intermediate,
    Expert,
    Custom {
        width: usize,
        height: usize,
        mines: usize,
    },
}

/// Errors returned when validating custom board parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CustomDifficultyError {
    WidthOutOfRange { min: usize, max: usize },
    HeightOutOfRange { min: usize, max: usize },
    TooFewMines,
    TooManyMines { max_allowed: usize },
}

impl fmt::Display for CustomDifficultyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WidthOutOfRange { min, max } => {
                write!(f, "Board width must be between {min} and {max}")
            }
            Self::HeightOutOfRange { min, max } => {
                write!(f, "Board height must be between {min} and {max}")
            }
            Self::TooFewMines => write!(f, "Mine count must be at least 1"),
            Self::TooManyMines { max_allowed } => {
                write!(
                    f,
                    "Too many mines (maximum {max_allowed} for this board size)"
                )
            }
        }
    }
}

impl std::error::Error for CustomDifficultyError {}

impl Difficulty {
    /// Returns `(width, height, mines)` for this difficulty.
    pub fn dimensions(&self) -> (usize, usize, usize) {
        match *self {
            Self::Beginner => (9, 9, 10),
            Self::Intermediate => (16, 16, 40),
            Self::Expert => (30, 16, 99),
            Self::Custom {
                width,
                height,
                mines,
            } => (width, height, mines),
        }
    }

    /// Display label for the difficulty.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Beginner => "Beginner",
            Self::Intermediate => "Intermediate",
            Self::Expert => "Expert",
            Self::Custom { .. } => "Custom",
        }
    }

    /// Validates custom board parameters so that a 3x3 safe zone around the first click is always possible.
    pub fn validate_custom(
        width: usize,
        height: usize,
        mines: usize,
    ) -> Result<Self, CustomDifficultyError> {
        if !(MIN_CUSTOM_DIM..=MAX_CUSTOM_WIDTH).contains(&width) {
            return Err(CustomDifficultyError::WidthOutOfRange {
                min: MIN_CUSTOM_DIM,
                max: MAX_CUSTOM_WIDTH,
            });
        }
        if !(MIN_CUSTOM_DIM..=MAX_CUSTOM_HEIGHT).contains(&height) {
            return Err(CustomDifficultyError::HeightOutOfRange {
                min: MIN_CUSTOM_DIM,
                max: MAX_CUSTOM_HEIGHT,
            });
        }
        if mines == 0 {
            return Err(CustomDifficultyError::TooFewMines);
        }
        let max_allowed = (width * height).saturating_sub(9).max(1);
        if mines > max_allowed {
            return Err(CustomDifficultyError::TooManyMines { max_allowed });
        }
        Ok(Self::Custom {
            width,
            height,
            mines,
        })
    }
}

/// Overall state of the Minesweeper game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameState {
    Ready,
    Playing,
    Won,
    Lost,
}

/// Visibility state of an individual cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellState {
    Hidden,
    Revealed,
    Flagged,
}

/// Data model for a single cell on the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub has_mine: bool,
    pub adjacent_mines: u8,
    pub state: CellState,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            has_mine: false,
            adjacent_mines: 0,
            state: CellState::Hidden,
        }
    }
}

/// Result of a reveal or chord operation, providing exact details for UI and animations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RevealOutcome {
    /// Cells revealed during this action as `(x, y, bfs_distance_from_origin)`.
    pub revealed: Vec<(usize, usize, usize)>,
    /// Mines that were triggered during this action.
    pub hit_mines: Vec<(usize, usize)>,
    /// Remaining hidden mines that were automatically flagged upon winning.
    pub auto_flagged: Vec<(usize, usize)>,
    /// New game state if a transition occurred (`Playing`, `Won`, or `Lost`).
    pub state_changed_to: Option<GameState>,
}

/// Result of toggling a flag on a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlagOutcome {
    Placed,
    Removed,
    Ignored,
}

/// Pure Minesweeper game board and state machine.
#[derive(Clone, Debug)]
pub struct Board {
    width: usize,
    height: usize,
    total_mines: usize,
    difficulty: Difficulty,
    cells: Vec<Cell>,
    state: GameState,
    flags_placed: usize,
    revealed_count: usize,
    mines_generated: bool,
    exploded_mines: Vec<(usize, usize)>,
}

impl Board {
    /// Creates a new board for the given difficulty preset or custom configuration.
    pub fn new(difficulty: Difficulty) -> Self {
        let (width, height, mines) = difficulty.dimensions();
        let width = width.max(1);
        let height = height.max(1);
        let max_mines = (width * height).saturating_sub(1);
        let total_mines = mines.min(max_mines);

        Self {
            width,
            height,
            total_mines,
            difficulty,
            cells: vec![Cell::default(); width * height],
            state: GameState::Ready,
            flags_placed: 0,
            revealed_count: 0,
            mines_generated: false,
            exploded_mines: Vec::new(),
        }
    }

    /// Creates a deterministic board with pre-placed mines (primarily for unit testing).
    pub fn from_mine_positions(
        width: usize,
        height: usize,
        mine_positions: &[(usize, usize)],
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let mut board = Self {
            width,
            height,
            total_mines: 0,
            difficulty: Difficulty::Custom {
                width,
                height,
                mines: mine_positions.len(),
            },
            cells: vec![Cell::default(); width * height],
            state: GameState::Playing,
            flags_placed: 0,
            revealed_count: 0,
            mines_generated: true,
            exploded_mines: Vec::new(),
        };

        let mut unique_mines = 0;
        for &(x, y) in mine_positions {
            if let Some(cell) = board.cell_mut(x, y) {
                if !cell.has_mine {
                    cell.has_mine = true;
                    unique_mines += 1;
                }
            }
        }
        board.total_mines = unique_mines;
        board.recalculate_adjacent_mines();
        board
    }

    /// Resets the board to the `Ready` state with the same dimensions and difficulty.
    pub fn reset(&mut self) {
        self.cells.fill(Cell::default());
        self.state = GameState::Ready;
        self.flags_placed = 0;
        self.revealed_count = 0;
        self.mines_generated = false;
        self.exploded_mines.clear();
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }

    #[inline]
    pub fn total_mines(&self) -> usize {
        self.total_mines
    }

    #[inline]
    pub fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    #[inline]
    pub fn state(&self) -> GameState {
        self.state
    }

    #[inline]
    pub fn flags_placed(&self) -> usize {
        self.flags_placed
    }

    #[inline]
    pub fn revealed_count(&self) -> usize {
        self.revealed_count
    }

    #[inline]
    pub fn mines_generated(&self) -> bool {
        self.mines_generated
    }

    #[inline]
    pub fn exploded_mines(&self) -> &[(usize, usize)] {
        &self.exploded_mines
    }

    /// Returns the number of remaining mines (`total_mines - flags_placed`). Can be negative if over-flagged.
    #[inline]
    pub fn remaining_mines(&self) -> isize {
        self.total_mines as isize - self.flags_placed as isize
    }

    #[inline]
    fn idx(&self, x: usize, y: usize) -> Option<usize> {
        if x < self.width && y < self.height {
            Some(y * self.width + x)
        } else {
            None
        }
    }

    /// Gets an immutable reference to the cell at `(x, y)`.
    #[inline]
    pub fn cell(&self, x: usize, y: usize) -> Option<&Cell> {
        self.idx(x, y).and_then(|i| self.cells.get(i))
    }

    #[inline]
    fn cell_mut(&mut self, x: usize, y: usize) -> Option<&mut Cell> {
        self.idx(x, y).and_then(|i| self.cells.get_mut(i))
    }

    /// Returns true if `(x, y)` is a mine that triggered the loss.
    pub fn is_exploded_mine(&self, x: usize, y: usize) -> bool {
        self.exploded_mines.contains(&(x, y))
    }

    /// Returns true if the game is lost and the cell at `(x, y)` has a flag on a non-mine cell.
    pub fn is_wrong_flag(&self, x: usize, y: usize) -> bool {
        if self.state != GameState::Lost {
            return false;
        }
        self.cell(x, y)
            .map(|c| c.state == CellState::Flagged && !c.has_mine)
            .unwrap_or(false)
    }

    /// Returns valid coordinates of all 8-directional neighbors of `(x, y)`.
    pub fn neighbors(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let mut result = Vec::with_capacity(8);
        let min_x = x.saturating_sub(1);
        let max_x = (x + 1).min(self.width.saturating_sub(1));
        let min_y = y.saturating_sub(1);
        let max_y = (y + 1).min(self.height.saturating_sub(1));

        for ny in min_y..=max_y {
            for nx in min_x..=max_x {
                if nx != x || ny != y {
                    result.push((nx, ny));
                }
            }
        }
        result
    }

    /// Generates mines after the first click at `(safe_x, safe_y)`.
    /// Guarantees that `(safe_x, safe_y)` and its 3x3 neighborhood are free of mines
    /// whenever the board has enough cells to accommodate the safe zone.
    pub fn generate_mines(&mut self, safe_x: usize, safe_y: usize) {
        let total_cells = self.width * self.height;
        let mut candidates: Vec<(usize, usize)> = Vec::with_capacity(total_cells);
        let mut ring_neighbors: Vec<(usize, usize)> = Vec::with_capacity(8);

        for y in 0..self.height {
            for x in 0..self.width {
                if x == safe_x && y == safe_y {
                    continue;
                }
                if x.abs_diff(safe_x) <= 1 && y.abs_diff(safe_y) <= 1 {
                    ring_neighbors.push((x, y));
                } else {
                    candidates.push((x, y));
                }
            }
        }

        let mut rng = rand::thread_rng();
        candidates.shuffle(&mut rng);

        // If the board is tiny and cannot fit all mines outside the 3x3 safe zone,
        // spill remaining mines into the immediate ring, while keeping (safe_x, safe_y) safe.
        if candidates.len() < self.total_mines {
            ring_neighbors.shuffle(&mut rng);
            candidates.extend(ring_neighbors);
        }

        for &(mx, my) in candidates.iter().take(self.total_mines) {
            if let Some(cell) = self.cell_mut(mx, my) {
                cell.has_mine = true;
            }
        }

        self.recalculate_adjacent_mines();
        self.mines_generated = true;
    }

    /// Computes `adjacent_mines` (0..=8) for every cell on the board.
    fn recalculate_adjacent_mines(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let count = self
                    .neighbors(x, y)
                    .into_iter()
                    .filter(|&(nx, ny)| self.cell(nx, ny).map(|c| c.has_mine).unwrap_or(false))
                    .count() as u8;
                if let Some(cell) = self.cell_mut(x, y) {
                    cell.adjacent_mines = count;
                }
            }
        }
    }

    /// Toggles a flag on a `Hidden` or `Flagged` cell at `(x, y)`.
    pub fn toggle_flag(&mut self, x: usize, y: usize) -> FlagOutcome {
        if matches!(self.state, GameState::Won | GameState::Lost) {
            return FlagOutcome::Ignored;
        }
        let Some(cell) = self.cell_mut(x, y) else {
            return FlagOutcome::Ignored;
        };
        match cell.state {
            CellState::Hidden => {
                cell.state = CellState::Flagged;
                self.flags_placed += 1;
                FlagOutcome::Placed
            }
            CellState::Flagged => {
                cell.state = CellState::Hidden;
                self.flags_placed = self.flags_placed.saturating_sub(1);
                FlagOutcome::Removed
            }
            CellState::Revealed => FlagOutcome::Ignored,
        }
    }

    /// Reveals the cell at `(x, y)`.
    /// On the first click (`GameState::Ready`), generates mines first so `(x, y)` and its neighbors are safe.
    pub fn reveal(&mut self, x: usize, y: usize) -> RevealOutcome {
        let mut outcome = RevealOutcome::default();
        if matches!(self.state, GameState::Won | GameState::Lost) {
            return outcome;
        }
        let Some(initial_cell) = self.cell(x, y).copied() else {
            return outcome;
        };
        if initial_cell.state != CellState::Hidden {
            return outcome;
        }

        if !self.mines_generated {
            self.generate_mines(x, y);
            self.state = GameState::Playing;
            outcome.state_changed_to = Some(GameState::Playing);
        }

        let Some(cell) = self.cell(x, y).copied() else {
            return outcome;
        };

        if cell.has_mine {
            if let Some(c) = self.cell_mut(x, y) {
                c.state = CellState::Revealed;
            }
            self.exploded_mines.push((x, y));
            outcome.hit_mines.push((x, y));
            self.state = GameState::Lost;
            outcome.state_changed_to = Some(GameState::Lost);
            return outcome;
        }

        self.flood_fill_reveal_multi(&[(x, y, 0)], &mut outcome);
        self.check_win_condition(&mut outcome);
        outcome
    }

    /// Chording: when clicking on an already-revealed numbered cell whose adjacent flag count
    /// equals `adjacent_mines`, reveals all non-flagged adjacent cells simultaneously.
    pub fn chord(&mut self, x: usize, y: usize) -> RevealOutcome {
        let mut outcome = RevealOutcome::default();
        if self.state != GameState::Playing {
            return outcome;
        }
        let Some(center_cell) = self.cell(x, y).copied() else {
            return outcome;
        };
        if center_cell.state != CellState::Revealed || center_cell.adjacent_mines == 0 {
            return outcome;
        }

        let neighbors = self.neighbors(x, y);
        let flagged_neighbors = neighbors
            .iter()
            .filter(|&&(nx, ny)| {
                self.cell(nx, ny)
                    .map(|c| c.state == CellState::Flagged)
                    .unwrap_or(false)
            })
            .count();

        if flagged_neighbors != center_cell.adjacent_mines as usize {
            return outcome;
        }

        let mut safe_starts = Vec::with_capacity(8);
        for (nx, ny) in neighbors {
            let Some(neighbor) = self.cell(nx, ny).copied() else {
                continue;
            };
            if neighbor.state != CellState::Hidden {
                continue;
            }
            if neighbor.has_mine {
                if let Some(c) = self.cell_mut(nx, ny) {
                    c.state = CellState::Revealed;
                }
                self.exploded_mines.push((nx, ny));
                outcome.hit_mines.push((nx, ny));
                self.state = GameState::Lost;
                outcome.state_changed_to = Some(GameState::Lost);
            } else {
                safe_starts.push((nx, ny, 1));
            }
        }

        if !safe_starts.is_empty() {
            self.flood_fill_reveal_multi(&safe_starts, &mut outcome);
        }

        if self.state != GameState::Lost {
            self.check_win_condition(&mut outcome);
        }
        outcome
    }

    /// Iterative Breadth-First Search (BFS) flood-fill to reveal connected empty areas
    /// and their bordering numbered cells from one or more starting cells.
    /// Marks cells as `Revealed` immediately upon enqueueing to avoid duplicate queue entries
    /// and preserve shortest-path wave distance from the click origin.
    fn flood_fill_reveal_multi(
        &mut self,
        starts: &[(usize, usize, usize)],
        outcome: &mut RevealOutcome,
    ) {
        let mut queue = VecDeque::with_capacity(starts.len() * 4);

        for &(sx, sy, initial_dist) in starts {
            if let Some(start_cell) = self.cell_mut(sx, sy) {
                if start_cell.state == CellState::Hidden && !start_cell.has_mine {
                    start_cell.state = CellState::Revealed;
                    self.revealed_count += 1;
                    queue.push_back((sx, sy, initial_dist));
                }
            }
        }

        while let Some((cx, cy, dist)) = queue.pop_front() {
            outcome.revealed.push((cx, cy, dist));

            let adjacent_mines = self.cell(cx, cy).map(|c| c.adjacent_mines).unwrap_or(1);

            // Only expand further if this cell has 0 adjacent mines.
            if adjacent_mines == 0 {
                for (nx, ny) in self.neighbors(cx, cy) {
                    if let Some(neighbor) = self.cell_mut(nx, ny) {
                        if neighbor.state == CellState::Hidden && !neighbor.has_mine {
                            neighbor.state = CellState::Revealed;
                            self.revealed_count += 1;
                            queue.push_back((nx, ny, dist + 1));
                        }
                    }
                }
            }
        }
    }

    /// Checks whether all non-mine cells have been revealed; if so, transitions to `GameState::Won`
    /// and automatically flags any remaining unflagged mines.
    fn check_win_condition(&mut self, outcome: &mut RevealOutcome) {
        let safe_cells = (self.width * self.height).saturating_sub(self.total_mines);
        if self.revealed_count == safe_cells && self.state != GameState::Lost {
            self.state = GameState::Won;
            outcome.state_changed_to = Some(GameState::Won);

            // Auto-flag all remaining mines on win
            for y in 0..self.height {
                for x in 0..self.width {
                    if let Some(cell) = self.cell_mut(x, y) {
                        if cell.has_mine && cell.state == CellState::Hidden {
                            cell.state = CellState::Flagged;
                            outcome.auto_flagged.push((x, y));
                        }
                    }
                }
            }
            self.flags_placed = self.total_mines;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mine_generation_count_and_first_click_safe_zone() {
        for _ in 0..30 {
            let mut board = Board::new(Difficulty::Intermediate);
            let click_x = 7;
            let click_y = 8;
            board.reveal(click_x, click_y);

            assert!(board.mines_generated());
            let actual_mines = board.cells.iter().filter(|cell| cell.has_mine).count();
            assert_eq!(actual_mines, 40);

            // Verify the 3x3 area around (click_x, click_y) has zero mines
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = (click_x as i32 + dx) as usize;
                    let ny = (click_y as i32 + dy) as usize;
                    let cell = board.cell(nx, ny).expect("valid neighbor");
                    assert!(
                        !cell.has_mine,
                        "Expected safe zone at ({nx}, {ny}) around first click ({click_x}, {click_y})"
                    );
                }
            }
        }
    }

    #[test]
    fn test_neighbor_mine_counts() {
        // 3x3 board with mines at (0,0), (2,0), and (1,2)
        // Layout:
        // M 2 M
        // 2 3 2
        // 1 M 1
        let mines = [(0, 0), (2, 0), (1, 2)];
        let board = Board::from_mine_positions(3, 3, &mines);

        assert_eq!(board.cell(1, 0).map(|c| c.adjacent_mines), Some(2));
        assert_eq!(board.cell(0, 1).map(|c| c.adjacent_mines), Some(2));
        assert_eq!(board.cell(1, 1).map(|c| c.adjacent_mines), Some(3));
        assert_eq!(board.cell(2, 1).map(|c| c.adjacent_mines), Some(2));
        assert_eq!(board.cell(0, 2).map(|c| c.adjacent_mines), Some(1));
        assert_eq!(board.cell(2, 2).map(|c| c.adjacent_mines), Some(1));
    }

    #[test]
    fn test_iterative_bfs_flood_fill() {
        // 5x5 board with a single mine at (4, 4)
        // Clicking (0, 0) should reveal all cells except (4, 4)
        let board_mines = [(4, 4)];
        let mut board = Board::from_mine_positions(5, 5, &board_mines);

        // Place a flag on (0, 2) to verify flood-fill respects flags
        assert_eq!(board.toggle_flag(0, 2), FlagOutcome::Placed);

        let outcome = board.reveal(0, 0);
        // 25 total cells - 1 mine - 1 flagged cell = 23 revealed cells
        assert_eq!(outcome.revealed.len(), 23);
        assert_eq!(board.revealed_count(), 23);
        assert_eq!(board.cell(0, 2).map(|c| c.state), Some(CellState::Flagged));
        assert_eq!(board.cell(4, 4).map(|c| c.state), Some(CellState::Hidden));

        // Distance of (0,0) should be 0, and distances should be non-decreasing in BFS order
        assert_eq!(outcome.revealed.first(), Some(&(0, 0, 0)));
        for window in outcome.revealed.windows(2) {
            assert!(window[0].2 <= window[1].2);
        }
    }

    #[test]
    fn test_chording_success_and_wrong_flag_loss() {
        // 4x4 board with mine at (0, 0)
        // (1, 0), (0, 1), (1, 1) all have adjacent_mines == 1
        let mut board = Board::from_mine_positions(4, 4, &[(0, 0)]);

        // Reveal (1, 1) which has adjacent_mines == 1
        let first = board.reveal(1, 1);
        assert_eq!(first.revealed.len(), 1);
        assert_eq!(board.state(), GameState::Playing);

        // Chording without flags does nothing
        let no_op = board.chord(1, 1);
        assert!(no_op.revealed.is_empty());

        // Flag the correct mine at (0, 0) and chord on (1, 1)
        board.toggle_flag(0, 0);
        let chord_ok = board.chord(1, 1);
        assert!(!chord_ok.revealed.is_empty());
        // All direct neighbors of (1, 1) that were revealed by chording must have distance 1,
        // and distances across the entire chord flood-fill must be non-decreasing.
        for &(rx, ry, dist) in &chord_ok.revealed {
            if rx.abs_diff(1) <= 1 && ry.abs_diff(1) <= 1 {
                assert_eq!(dist, 1, "Direct neighbor ({rx}, {ry}) must have distance 1");
            }
        }
        for window in chord_ok.revealed.windows(2) {
            assert!(window[0].2 <= window[1].2);
        }
        // Since (0, 0) was the only mine, chording (1, 1) opens neighbors including 0-cells, winning the game!
        assert_eq!(board.state(), GameState::Won);
        assert_eq!(chord_ok.state_changed_to, Some(GameState::Won));

        // Now test chording with a wrong flag
        let mut board2 = Board::from_mine_positions(4, 4, &[(0, 0)]);
        board2.reveal(1, 1);
        // Misplace flag at (1, 0) instead of (0, 0)
        board2.toggle_flag(1, 0);
        let chord_fail = board2.chord(1, 1);
        assert_eq!(board2.state(), GameState::Lost);
        assert_eq!(chord_fail.hit_mines, vec![(0, 0)]);
        assert!(board2.is_wrong_flag(1, 0));
        assert!(!board2.is_wrong_flag(0, 0));
        assert!(board2.is_exploded_mine(0, 0));
    }

    #[test]
    fn test_win_and_lose_conditions() {
        // 3x3 board with 1 mine at (2, 2)
        let mut board = Board::from_mine_positions(3, 3, &[(2, 2)]);
        // Clicking (0, 0) has adjacent_mines == 0 and flood-fills all 8 safe cells!
        let outcome = board.reveal(0, 0);
        assert_eq!(board.state(), GameState::Won);
        assert_eq!(outcome.state_changed_to, Some(GameState::Won));
        assert_eq!(outcome.auto_flagged, vec![(2, 2)]);
        assert_eq!(board.remaining_mines(), 0);
        assert_eq!(board.cell(2, 2).map(|c| c.state), Some(CellState::Flagged));

        // Losing by clicking directly on a mine
        let mut lose_board = Board::from_mine_positions(3, 3, &[(1, 1)]);
        let lose_outcome = lose_board.reveal(1, 1);
        assert_eq!(lose_board.state(), GameState::Lost);
        assert_eq!(lose_outcome.state_changed_to, Some(GameState::Lost));
        assert_eq!(lose_outcome.hit_mines, vec![(1, 1)]);
        // Subsequent reveals or flags after loss are ignored
        assert!(lose_board.reveal(0, 0).revealed.is_empty());
        assert_eq!(lose_board.toggle_flag(0, 0), FlagOutcome::Ignored);
    }

    #[test]
    fn test_custom_difficulty_validation() {
        assert!(Difficulty::validate_custom(10, 10, 15).is_ok());
        assert_eq!(
            Difficulty::validate_custom(3, 10, 5),
            Err(CustomDifficultyError::WidthOutOfRange {
                min: MIN_CUSTOM_DIM,
                max: MAX_CUSTOM_WIDTH
            })
        );
        assert_eq!(
            Difficulty::validate_custom(10, 40, 5),
            Err(CustomDifficultyError::HeightOutOfRange {
                min: MIN_CUSTOM_DIM,
                max: MAX_CUSTOM_HEIGHT
            })
        );
        assert_eq!(
            Difficulty::validate_custom(10, 10, 0),
            Err(CustomDifficultyError::TooFewMines)
        );
        assert_eq!(
            Difficulty::validate_custom(5, 5, 20),
            Err(CustomDifficultyError::TooManyMines { max_allowed: 16 })
        );
    }

    #[test]
    fn test_out_of_bounds_overflagging_and_tiny_board_edge_cases() {
        let mut board = Board::new(Difficulty::Beginner);

        // Out-of-bounds coordinates should be safely ignored without panicking
        assert!(board.reveal(100, 100).revealed.is_empty());
        assert_eq!(board.toggle_flag(100, 100), FlagOutcome::Ignored);
        assert!(board.chord(100, 100).revealed.is_empty());

        // Flagged cell should be protected from left-click reveal
        assert_eq!(board.toggle_flag(2, 2), FlagOutcome::Placed);
        assert!(board.reveal(2, 2).revealed.is_empty());
        assert_eq!(board.cell(2, 2).map(|c| c.state), Some(CellState::Flagged));
        assert_eq!(board.toggle_flag(2, 2), FlagOutcome::Removed);

        // Over-flagging on a small board can produce negative remaining_mines safely
        let mut tiny = Board::from_mine_positions(2, 2, &[(0, 0)]);
        tiny.toggle_flag(0, 0);
        tiny.toggle_flag(1, 0);
        tiny.toggle_flag(0, 1);
        assert_eq!(tiny.remaining_mines(), -2);

        // Tiny board where 3x3 safe zone spills into immediate neighbors still keeps clicked cell safe
        let mut dense = Board::new(Difficulty::Custom {
            width: 3,
            height: 3,
            mines: 5,
        });
        let outcome = dense.reveal(1, 1);
        assert_ne!(dense.state(), GameState::Lost);
        assert!(!outcome.revealed.is_empty());
        assert_eq!(dense.cell(1, 1).map(|c| c.has_mine), Some(false));
    }
}
