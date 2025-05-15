use fleetcore::{FireInputs, ReportJournal};
use risc0_zkvm::guest::env;
//use risc0_zkvm::Digest;
//use sha2::{Digest as _, Sha256};

fn main() {
    // read the input

    let mut _input: FireInputs = env::read();
    prinntln!("FireInputs: {:?}", _input);
    // TODO: do something with the input
    let output = ReportJournal::default();

    // write public output to the journal
    env::commit(&output);
}
