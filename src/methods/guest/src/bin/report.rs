use fleetcore::{FireInputs, ReportJournal};
use risc0_zkvm::guest::env;
use risc0_zkvm::sha::Digest;
use sha2::{Digest as _, Sha256};

fn main() {
    // Read the input
    let input: FireInputs = env::read();

    // Hash both the random number and current board to create a commitment (random first for better security)
    let mut hasher = Sha256::new();
    hasher.update(input.random.as_bytes());
    hasher.update(&input.board);
    let board_hash: [u8; 32] = hasher.finalize().into();
    let board_digest = Digest::from(board_hash);

    // Simulate updating the board after the report (in a real implementation,
    // this would modify the board based on the hit/miss report)
    let mut next_board = input.board.clone();

    // For demonstration purposes, set the position to 1 to mark it as "shot"
    // TODO: correct this logic based on actual game rules
    if input.pos < next_board.len() as u8 {
        let idx = input.pos as usize;
        next_board[idx] = 1;
    }

    // Hash both the random number and next board state (random first for better security)
    let mut next_hasher = Sha256::new();
    next_hasher.update(input.random.as_bytes());
    next_hasher.update(&next_board);
    let next_board_hash: [u8; 32] = next_hasher.finalize().into();
    let next_board_digest = Digest::from(next_board_hash);

    // Create the journal output
    let output = ReportJournal {
        gameid: input.gameid,
        fleet: input.fleet,
        report: input.target, // In our implementation, target field contains the report value (Hit/Miss)
        pos: input.pos,
        board: board_digest,
        next_board: next_board_digest,
    };

    // Commit the output to the journal
    env::commit(&output);
}
