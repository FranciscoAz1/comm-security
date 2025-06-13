use fleetcore::{BaseInputs, BaseJournal};
use risc0_zkvm::guest::env;
use risc0_zkvm::sha::Digest;
use sha2::{Digest as _, Sha256};
use std::collections::{HashSet, VecDeque};

const BOARD_SIZE: usize = 10;

/// Discovers a ship using BFS and returns its size
fn discover_ship(
    grid: &[[bool; BOARD_SIZE]; BOARD_SIZE],
    start_row: usize,
    start_col: usize,
    visited: &mut HashSet<(usize, usize)>,
) -> usize {
    let mut queue = VecDeque::new();
    queue.push_back((start_row, start_col));
    visited.insert((start_row, start_col));
    let mut size = 0;

    while let Some((row, col)) = queue.pop_front() {
        size += 1;

        // Check 4-directional neighbors (no diagonals)
        for (dr, dc) in &[(0, 1), (1, 0), (0, -1), (-1, 0)] {
            let new_row = (row as isize + dr) as usize;
            let new_col = (col as isize + dc) as usize;

            // Proper bounds checking to prevent out-of-bounds access
            if new_row < BOARD_SIZE
                && new_col < BOARD_SIZE
                && grid[new_row][new_col]
                && !visited.contains(&(new_row, new_col))
            {
                visited.insert((new_row, new_col));
                queue.push_back((new_row, new_col));
            }
        }
    }

    size
}

/// Validates that a ship is placed in a straight line (horizontal or vertical)
fn is_valid_ship_shape(
    grid: &[[bool; BOARD_SIZE]; BOARD_SIZE],
    start_row: usize,
    start_col: usize,
) -> bool {
    let mut ship_positions = Vec::new();
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();

    queue.push_back((start_row, start_col));
    visited.insert((start_row, start_col));

    // Collect all positions of this ship
    while let Some((row, col)) = queue.pop_front() {
        ship_positions.push((row, col));

        for (dr, dc) in &[(0, 1), (1, 0), (0, -1), (-1, 0)] {
            let new_row = (row as isize + dr) as usize;
            let new_col = (col as isize + dc) as usize;

            if new_row < BOARD_SIZE
                && new_col < BOARD_SIZE
                && grid[new_row][new_col]
                && !visited.contains(&(new_row, new_col))
            {
                visited.insert((new_row, new_col));
                queue.push_back((new_row, new_col));
            }
        }
    }

    // Single cell ship is always valid
    if ship_positions.len() == 1 {
        return true;
    }

    // Sort positions to check if they form a line
    ship_positions.sort();

    // Check if all positions are in the same row (horizontal ship)
    let same_row = ship_positions
        .iter()
        .all(|(r, _)| *r == ship_positions[0].0);
    if same_row {
        // Check if columns are consecutive
        for i in 1..ship_positions.len() {
            if ship_positions[i].1 != ship_positions[i - 1].1 + 1 {
                return false;
            }
        }
        return true;
    }

    // Check if all positions are in the same column (vertical ship)
    let same_col = ship_positions
        .iter()
        .all(|(_, c)| *c == ship_positions[0].1);
    if same_col {
        // Check if rows are consecutive
        for i in 1..ship_positions.len() {
            if ship_positions[i].0 != ship_positions[i - 1].0 + 1 {
                return false;
            }
        }
        return true;
    }

    false // Ship is neither horizontal nor vertical
}

fn validate_board(board: &[u8]) -> Result<(), String> {
    // Convert positions into a grid (10x10)
    let mut grid = [[false; BOARD_SIZE]; BOARD_SIZE];
    let mut positions = HashSet::new();

    for &pos in board {
        // Validate position range (0-99 for 10x10 board)
        if pos >= 100 {
            return Err(format!("Position {} is out of bounds (must be 0-99)", pos));
        }

        let row = (pos / 10) as usize;
        let col = (pos % 10) as usize;

        if grid[row][col] {
            return Err(format!("Duplicate position: {}", pos));
        }

        grid[row][col] = true;
        positions.insert((row, col));
    }

    // Verify total positions count: 2 size1 + 2 size2 + 1 size4 = 10
    let expected_total = 2 * 1 + 2 * 2 + 1 * 4;
    if positions.len() != expected_total {
        return Err(format!(
            "Invalid number of positions: expected {}, got {}",
            expected_total,
            positions.len()
        ));
    }

    // Expected ship counts: 2x1, 2x2, 1x4
    let mut size1 = 0;
    let mut size2 = 0;
    let mut size4 = 0;

    // Visited tracker to avoid reprocessing
    let mut visited = HashSet::new();

    for &(row, col) in &positions {
        if visited.contains(&(row, col)) {
            continue;
        }

        let ship_size = discover_ship(&grid, row, col, &mut visited);

        // Validate ship shape (must be straight line)
        if !is_valid_ship_shape(&grid, row, col) {
            return Err(format!(
                "Ship starting at ({}, {}) is not in a straight line",
                row, col
            ));
        }

        match ship_size {
            1 => size1 += 1,
            2 => size2 += 1,
            4 => size4 += 1,
            s => return Err(format!("Invalid ship size: {} (allowed: 1, 2, 4)", s)),
        }
    }

    // Verify counts
    if size1 != 2 || size2 != 2 || size4 != 1 {
        return Err(format!(
            "Invalid ship counts: Expected (2x1, 2x2, 1x4), Found ({}x1, {}x2, {}x4)",
            size1, size2, size4
        ));
    }

    Ok(())
}

fn main() {
    // Read the input
    let input: BaseInputs = env::read();

    // Debug print the input
    risc0_zkvm::guest::env::log(&format!("[DEBUG] Full Input: {:#?}", input));

    // Validate board configuration
    // if let Err(e) = unmarshal_board(&input.board) {
    if let Err(e) = validate_board(&input.board) {
        risc0_zkvm::guest::env::log(&format!("[ERROR] Board validation failed: {}", e));

        // Create a "failure" journal entry with zero hash to indicate invalid board
        let zero_digest = Digest::from([0u8; 32]);
        let failure_output = BaseJournal {
            gameid: input.gameid,
            fleet: input.fleet,
            board: zero_digest, // Zero hash indicates validation failure
        };
        env::commit(&failure_output);
        return;
    }

    // Hash both the random number and board to create a commitment
    let mut hasher = Sha256::new();
    hasher.update(input.random.as_bytes());
    hasher.update(&input.board);
    let board_hash: [u8; 32] = hasher.finalize().into();
    let board_digest = Digest::from(board_hash);

    // Create the journal output with the data
    let output = BaseJournal {
        gameid: input.gameid, // onde vamos dar join
        fleet: input.fleet,   // fleet positions
        board: board_digest,  // hash
    };

    // Commit the output to the journal
    env::commit(&output);
}
