// src/game_actions.rs

use fleetcore::{BaseInputs, Command, FireInputs};
use methods::{FIRE_ELF, JOIN_ELF, REPORT_ELF, WAVE_ELF, WIN_ELF};
use risc0_zkvm::{default_prover, ExecutorEnv};

use crate::{unmarshal_data, unmarshal_fire, unmarshal_report, send_receipt, FormData};

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
    
    // Create the executor environment
    let env = ExecutorEnv::builder()
        .write(&input)
        .unwrap()
        .build()
        .unwrap();

    // Get the prover and generate the receipt
    let prover = default_prover();
    let receipt = prover.prove(env, methods::JOIN_ELF)
        .expect("Failed to generate zk-SNARK proof for join action")
        .receipt;

    // Send the receipt to the blockchain
    send_receipt(Command::Join, receipt).await
}

pub async fn fire(idata: FormData) -> String {
    let (gameid, fleetid, board, random, targetfleet, x, y) = match unmarshal_fire(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };
    
    // Calculate position from x and y coordinates (as a single byte)
    let pos = x * 10 + y;
    
    // Create the input data for the zkVM
    let input = fleetcore::FireInputs {
        gameid: gameid.clone(),
        fleet: fleetid.clone(),
        board: board.clone(),
        random: random.clone(),
        target: targetfleet.clone(),
        pos,
    };
    
    // Create the executor environment
    let env = ExecutorEnv::builder()
        .write(&input)
        .unwrap()
        .build()
        .unwrap();

    // Get the prover and generate the receipt
    let prover = default_prover();
    let receipt = prover.prove(env, methods::FIRE_ELF)
        .expect("Failed to generate zk-SNARK proof for fire action")
        .receipt;

    // Send the receipt to the blockchain
    send_receipt(Command::Fire, receipt).await
}

pub async fn report(idata: FormData) -> String {
    let (gameid, fleetid, board, random, report, x, y) = match unmarshal_report(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };
    
    // Calculate position from x and y coordinates (as a single byte)
    let pos = x * 10 + y;
    
    // Create the input data for the zkVM
    let input = fleetcore::FireInputs {
        gameid: gameid.clone(),
        fleet: fleetid.clone(),
        board: board.clone(),
        random: random.clone(),
        target: report.clone(), // We reuse the FireInputs struct with target field for report value
        pos,
    };
    
    // Create the executor environment
    let env = ExecutorEnv::builder()
        .write(&input)
        .unwrap()
        .build()
        .unwrap();

    // Get the prover and generate the receipt
    let prover = default_prover();
    let receipt = prover.prove(env, methods::REPORT_ELF)
        .expect("Failed to generate zk-SNARK proof for report action")
        .receipt;

    // Send the receipt to the blockchain
    send_receipt(Command::Report, receipt).await
}

pub async fn wave(idata: FormData) -> String {
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
    
    // Create the executor environment
    let env = ExecutorEnv::builder()
        .write(&input)
        .unwrap()
        .build()
        .unwrap();

    // Get the prover and generate the receipt
    let prover = default_prover();
    let receipt = prover.prove(env, methods::WAVE_ELF)
        .expect("Failed to generate zk-SNARK proof for wave action")
        .receipt;

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
    
    // Create the executor environment
    let env = ExecutorEnv::builder()
        .write(&input)
        .unwrap()
        .build()
        .unwrap();

    // Get the prover and generate the receipt
    let prover = default_prover();
    let receipt = prover.prove(env, methods::WIN_ELF)
        .expect("Failed to generate zk-SNARK proof for win action")
        .receipt;

    // Send the receipt to the blockchain
    send_receipt(Command::Win, receipt).await
}
