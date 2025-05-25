use fleetcore::{BaseInputs, BaseJournal};
use risc0_zkvm::guest::env;
use risc0_zkvm::sha::Digest;
use sha2::{Digest as _, Sha256};

fn main() {
    //NOSSO!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
    // Read the input
    let input: BaseInputs = env::read();
    
    // Hash the board to create a commitment
    let mut hasher = Sha256::new();
    hasher.update(&input.board);
    let board_hash: [u8; 32] = hasher.finalize().into();
    let board_digest = Digest::from(board_hash);
    
    // Create the journal output with the data
    let output = BaseJournal {
        gameid: input.gameid,
        fleet: input.fleet,
        board: board_digest,
    };

    // Commit the output to the journal
    env::commit(&output);
}
