use fleetcore::{BaseInputs, BaseJournal};
use risc0_zkvm::guest::env;
//use risc0_zkvm::Digest;
//use sha2::{Digest as _, Sha256};

fn main() {
    // read the input
    let mut _input: BaseInputs = env::read();
    println!("BaseInputs: {:?}", _input);
    // TODO: do something with the input
    let output = BaseJournal::default();

    env::commit(&output);
}
