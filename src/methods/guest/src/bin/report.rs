use fleetcore::{FireInputs, ReportJournal, DEAD};
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

    // TODO: correct this logic based on actual game board design. We want to hit a flit, and get a new board that is updated from that fleet
    // if input.pos < board_length.len() as u8 {
    //     let idx = input.pos as usize;
    //     next_board[idx] = 1;
    // }
    let idx = input.pos as usize;
    next_board[idx] = 1;

    print!("board: {:#?}", &input.board);
    print!("next_board: {:#?}", &next_board);

    // until here is testing
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
