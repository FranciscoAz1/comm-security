// src/game_actions.rs

use fleetcore::{BaseInputs, Command, FireInputs};
use methods::{FIRE_ELF, JOIN_ELF, REPORT_ELF, WAVE_ELF, WIN_ELF};

use crate::{unmarshal_data, unmarshal_fire, unmarshal_report, send_receipt, FormData};

pub async fn join_game(idata: FormData) -> String {
    let (gameid, fleetid, board, random) = match unmarshal_data(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };

    // TO DO: Rebuild the receipt

    // Uncomment the following line when you are ready to send the receipt
    // proofs
    //send_receipt(Command::Fire, receipt).await
    // Comment out the following line when you are ready to send the receipt
    "OK".to_string()
}

pub async fn fire(idata: FormData) -> String {
    let (gameid, fleetid, board, random, targetfleet, x, y) = match unmarshal_fire(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };
    // TO DO: Rebuild the receipt
    //same as win because it verifies there are no boats
    // Uncomment the following line when you are ready to send the receipt
    //send_receipt(Command::Fire, receipt).await
    // Comment out the following line when you are ready to send the receipt
    "OK".to_string()
}

pub async fn report(idata: FormData) -> String {
    let (gameid, fleetid, board, random, _report, x, y) = match unmarshal_report(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };
    // TO DO: Rebuild the receipt

    // Uncomment the following line when you are ready to send the receipt
    //send_receipt(Command::Fire, receipt).await
    // Comment out the following line when you are ready to send the receipt
    "OK".to_string()
}

pub async fn wave(idata: FormData) -> String {
    let (gameid, fleetid, board, random) = match unmarshal_data(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };
    // TO DO: Rebuild the receipt

    // Uncomment the following line when you are ready to send the receipt
    //send_receipt(Command::Fire, receipt).await
    // Comment out the following line when you are ready to send the receipt
    "OK".to_string()
}

pub async fn win(idata: FormData) -> String {
    let (gameid, fleetid, board, random) = match unmarshal_data(&idata) {
        Ok(values) => values,
        Err(err) => return err,
    };
    // TO DO: Rebuild the receipt

    // Uncomment the following line when you are ready to send the receipt
    //send_receipt(Command::Fire, receipt).await
    // Comment out the following line when you are ready to send the receipt
    "OK".to_string()
}
