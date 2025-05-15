use fleetcore::{BaseInputs, BaseJournal};
use risc0_zkvm::guest::env;
//use risc0_zkvm::Digest;
//use sha2::{Digest as _, Sha256};

fn main() {
    // read the input
    let _input: BaseInputs = env::read();

    // TODO: do something with the input
    prinntln!("BaseInputs: {:?}", _input);
    let output = BaseJournal::default();

    // write public output to the journal
    env::commit(&output);
}
