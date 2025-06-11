// src/game_actions.rs

use fleetcore::{BaseInputs, Command, FireInputs};
use methods::{FIRE_ELF, JOIN_ELF, REPORT_ELF, WAVE_ELF, WIN_ELF};
use risc0_zkvm::{default_prover, ExecutorEnv, Receipt};
use std::sync::Arc;

use crate::{
    send_receipt, unmarshal_data, unmarshal_fire, unmarshal_report, unmarshal_wave, FormData,
};

// Helper function to generate a proof without threading issues
fn generate_proof(
    input: &(impl serde::Serialize + std::fmt::Debug),
    method_elf: &[u8],
    action_name: &str,
) -> Receipt {
    // Create the executor environment
    let env = ExecutorEnv::builder()
        .write(input)
        .unwrap()
        .build()
        .unwrap();

    // Get the prover and generate the receipt
    let prover = default_prover();
    prover
        .prove(env, method_elf)
        .expect(&format!(
            "Failed to generate zk-SNARK proof for {} action",
            action_name
        ))
        .receipt
}

pub async fn join_game(idata: FormData) -> String {
    let (gameid, fleetid, board, random) = match unmarshal_data(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };

    // Create the input data for the zkVM
    let input = fleetcore::BaseInputs {
        gameid: gameid.clone(),
        fleet: fleetid.clone(),
        board: board.clone(),
        random: random.clone(),
    };
    // print!("Join game input: {:#?}", input);
    //TODO:  send message to host page

    // Generate proof
    let receipt = generate_proof(&input, methods::JOIN_ELF, "join");

    // Send the receipt to the blockchain
    send_receipt(Command::Join, receipt).await
}

pub async fn fire(idata: FormData) -> String {
    let (gameid, fleetid, board, random, targetfleet, x, y) = match unmarshal_fire(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };

    // Calculate position from x and y coordinates (as a single byte)
    let pos = y * 10 + x;

    // Create the input data for the zkVM
    let input = fleetcore::FireInputs {
        gameid: gameid.clone(),
        fleet: fleetid.clone(),
        board: board.clone(),
        random: random.clone(),
        target: targetfleet.clone(),
        pos,
    };

    // Generate proof
    let receipt = generate_proof(&input, methods::FIRE_ELF, "fire");

    // Send the receipt to the blockchain
    send_receipt(Command::Fire, receipt).await
}

pub async fn report(idata: FormData) -> String {
    let (gameid, fleetid, board, random, report, x, y) = match unmarshal_report(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };

    // Calculate position from x and y coordinates (as a single byte)
    let pos = y * 10 + x;

    // Create the input data for the zkVM
    let input = fleetcore::FireInputs {
        gameid: gameid.clone(),
        fleet: fleetid.clone(),
        board: board.clone(),
        random: random.clone(),
        target: report.clone(), // We reuse the FireInputs struct with target field for report value
        pos,
    };

    // Generate proof
    let receipt = generate_proof(&input, methods::REPORT_ELF, "report");

    // Send the receipt to the blockchain
    send_receipt(Command::Report, receipt).await
}

pub async fn wave(idata: FormData) -> String {
    let (gameid, fleetid, random) = match unmarshal_wave(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };

    // Create the input data for the zkVM
    let input = fleetcore::BaseInputs {
        gameid: gameid.clone(),
        fleet: fleetid.clone(),
        board: vec![], // Board is not used in wave, so we can pass an empty vector
        random: random.clone(),
    };

    // Generate proof
    let receipt = generate_proof(&input, methods::WAVE_ELF, "wave");

    // Send the receipt to the blockchain
    send_receipt(Command::Wave, receipt).await
}

pub async fn win(idata: FormData) -> String {
    let (gameid, fleetid, board, random) = match unmarshal_data(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };

    // Create the input data for the zkVM
    let input = fleetcore::BaseInputs {
        gameid: gameid.clone(),
        fleet: fleetid.clone(),
        board: board.clone(),
        random: random.clone(),
    };

    // Generate proof
    let receipt = generate_proof(&input, methods::WIN_ELF, "win");

    // Send the receipt to the blockchain
    send_receipt(Command::Win, receipt).await
}
