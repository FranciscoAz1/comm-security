use fleetcore::{FireInputs, ReportJournal};
use risc0_zkvm::guest::env;
use risc0_zkvm::sha::Digest;
use sha2::{Digest as _, Sha256};

/// Converts a coordinate string (e.g., "A0") back to a position number (0-99)
/// This is the inverse of the xy_pos function
fn pos_xy(coord: &str) -> Result<u8, &'static str> {
    let chars: Vec<char> = coord.chars().collect();

    if chars.len() < 2 {
        return Err("Coordinate string is too short");
    }

    // First character should be a letter A-J
    let x_char = chars[0].to_ascii_uppercase();
    if x_char < 'A' || x_char > 'J' {
        return Err("X {:#?} coordinate must be a letter from A to J");
    }
    let x = (x_char as u8) - 65; // Convert 'A' to 0, 'B' to 1, etc.

    // Second+ characters should form a number 0-9
    let y_str: String = chars[1..].iter().collect();
    let y = match y_str.parse::<u8>() {
        Ok(num) if num < 10 => num,
        Ok(_) => return Err("Y coordinate must be between 0 and 9"),
        Err(_) => return Err("Y coordinate must be a valid number"),
    };

    // Calculate position
    Ok(y * 10 + x)
}
fn main() {
    // Read the input
    let input: FireInputs = env::read();
    println!("REPORT PROOFING");

    // Hash both the random number and current board to create a commitment (random first for better security)
    let mut hasher = Sha256::new();
    hasher.update(input.random.as_bytes());
    hasher.update(&input.board);
    let board_hash: [u8; 32] = hasher.finalize().into();
    let board_digest = Digest::from(board_hash);

    // Simulate updating the board after the report (in a real implementation,
    // this would modify the board based on the hit/miss report)

    // TODO: correct this logic based on actual game board design. We want to hit a flit, and get a new board that is updated from that fleet
    // if input.pos < board_length.len() as u8 {
    //     let idx = input.pos as usize;
    //     next_board[idx] = 1;
    // }

    println!("board: {:#?}", &input.board);
    // Convert position number to string representation for pos_xy function
    let pos_str = format!(
        "{}{}",
        ((input.pos % 10) as u8 + 65) as char,
        input.pos / 10
    );
    let xy = pos_xy(&pos_str);
    println!("xy: {:#?} pos_str: {:#?}", &xy, &pos_str);

    // print!("board: {:#?}", &input.board);
    // check hit or miss
    let xy = xy.unwrap();
    let mut hit = false;

    // Create a new board by filtering out the hit position
    let mut next_board = input.board.clone();
    next_board.retain(|&cell| {
        if cell == xy {
            hit = true;
            false // Remove this cell
        } else {
            true // Keep this cell
        }
    });
    println!("next_board: {:#?}", &next_board);
    // Hash both the random number and next board state (random first for better security)
    let mut next_hasher = Sha256::new();
    next_hasher.update(input.random.as_bytes());
    next_hasher.update(&next_board);
    let next_board_hash: [u8; 32] = next_hasher.finalize().into();
    let next_board_digest = Digest::from(next_board_hash);

    // Create the journal output
    let output = ReportJournal {
        gameid: input.gameid.clone(),
        fleet: input.fleet.clone(),
        report: input.target.clone(), // In our implementation, target field contains the report value (Hit/Miss)
        pos: input.pos.clone(),
        board: board_digest.clone(),
        next_board: next_board_digest.clone(),
    };

    // Commit the output to the journal
    env::commit(&output);
}
