// Remove the following 3 lines to enable compiler checkings
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use axum::{
    extract::Extension,
    response::{sse::Event, Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use futures::stream::StreamExt;
use rand::{seq::IteratorRandom, SeedableRng};
use risc0_zkvm::Digest;
use std::path::PathBuf;
use std::{
    collections::HashMap,
    error::Error,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use fleetcore::{BaseJournal, Command, CommunicationData, FireJournal, ReportJournal};
use methods::{FIRE_ID, JOIN_ID, REPORT_ID, WAVE_ID, WIN_ID};

/// Updates the turn counters for all players in a game.
/// Sets the current player's want_turn_count to 0 and increments all other players' want_turn_count.
fn update_turn_counters(game: &mut Game, current_player: &str) {
    // Set current player's want_turn_count to 0
    if let Some(player) = game.pmap.get_mut(current_player) {
        player.want_turn_count = 0;
    }

    // Increment all players' want_turn_count
    for (name, other_player) in game.pmap.iter_mut() {
        if name != current_player {
            other_player.want_turn_count += 1;
        }
    }
}

struct Player {
    name: String,
    current_state: Digest,
    want_turn_count: u32, // Number of turns this player has not played
    shots_hit: Vec<u8>,   // Positions where this player has hit a ship
}

struct Game {
    pmap: HashMap<String, Player>,
    next_player: Option<String>,
    next_report: Option<String>,
    next_shot: Option<u8>,
}

#[derive(Clone)]
struct SharedData {
    tx: broadcast::Sender<String>,
    gmap: Arc<Mutex<HashMap<String, Game>>>,
    rng: Arc<Mutex<rand::rngs::StdRng>>,
}

#[tokio::main]
async fn main() {
    // Initialize the default crypto provider for rustls
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install crypto provider");

    // Create a broadcast channel for log messages
    let (tx, _rx) = broadcast::channel::<String>(100);
    let shared = SharedData {
        tx: tx,
        gmap: Arc::new(Mutex::new(HashMap::new())),
        rng: Arc::new(Mutex::new(rand::rngs::StdRng::from_entropy())),
    };

    // Build our application with a route
    let app = Router::new()
        .route("/", get(index))
        .route("/logs", get(logs))
        .route("/chain", post(smart_contract))
        .layer(Extension(shared));

    // Configure TLS
    let config = RustlsConfig::from_pem_file(
        PathBuf::from("certs/blockchain-cert.pem"),
        PathBuf::from("certs/blockchain-key.pem"),
    )
    .await
    .expect("Failed to load TLS certificates");

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    println!("Listening on https://{}", addr);

    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// Handler to serve the HTML page
async fn index() -> Html<&'static str> {
    Html(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Blockchain Emulator</title>
        </head>
        <body>
            <h1>Registered Transactions</h1>          
            <ul id="logs"></ul>
            <script>
                const eventSource = new EventSource('/logs');
                eventSource.onmessage = function(event) {
                    const logs = document.getElementById('logs');
                    const log = document.createElement('li');
                    log.textContent = event.data;
                    logs.appendChild(log);
                };
            </script>
        </body>
        </html>
        "#,
    )
}

// Handler to manage SSE connections
#[axum::debug_handler]
async fn logs(Extension(shared): Extension<SharedData>) -> impl IntoResponse {
    let rx = BroadcastStream::new(shared.tx.subscribe());
    let stream = rx.filter_map(|result| async move {
        match result {
            Ok(msg) => Some(Ok(Event::default().data(msg))),
            Err(_) => Some(Err(Box::<dyn Error + Send + Sync>::from("Error"))),
        }
    });

    axum::response::sse::Sse::new(stream)
}
/// Converts a position number (0-99) to a coordinate string (e.g., "A0")
fn xy_pos(pos: u8) -> String {
    let x = pos % 10;
    let y = pos / 10;
    format!("{}{}", (x + 65) as char, y)
}

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
        return Err("X coordinate must be a letter from A to J");
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

async fn smart_contract(
    Extension(shared): Extension<SharedData>,
    Json(input_data): Json<CommunicationData>,
) -> String {
    match input_data.cmd {
        Command::Join => handle_join(&shared, &input_data),
        Command::Fire => handle_fire(&shared, &input_data),
        Command::Report => handle_report(&shared, &input_data),
        Command::Wave => handle_wave(&shared, &input_data),
        Command::Win => handle_win(&shared, &input_data),
    }
}
 
fn handle_join(shared: &SharedData, input_data: &CommunicationData) -> String {
    if input_data.receipt.verify(JOIN_ID).is_err() {
        shared
            .tx
            .send("Attempting to join game with invalid receipt".to_string())
            .unwrap();
        return "Could not verify receipt".to_string();
    }
    let data: BaseJournal = input_data.receipt.journal.decode().unwrap();
    let mut gmap = shared.gmap.lock().unwrap();
    // check if player is already in the game
    if gmap.contains_key(&data.gameid) && gmap[&data.gameid].pmap.contains_key(&data.fleet) {
        shared
            .tx
            .send(format!(
                "Player {} already in game {}",
                data.fleet, data.gameid
            ))
            .unwrap();
        return "Player already in game".to_string();
    }
    let game = gmap.entry(data.gameid.clone()).or_insert(Game {
        pmap: HashMap::new(),
        next_player: Some(data.fleet.clone()),
        next_report: None,
        next_shot: None,
    });
    let player_inserted = game
        .pmap
        .entry(data.fleet.clone())
        .or_insert_with(|| Player {
            name: data.fleet.clone(),
            current_state: data.board.clone(),
            want_turn_count: 1,
            shots_hit: Vec::new(),
        })
        .name
        == data.fleet;

    let mesg = if player_inserted {
        format!("Joined game {}", data.gameid)
    } else {
        format!("Player already in game {}", data.gameid)
    };
    shared.tx.send(mesg).unwrap();

    "OK".to_string()
}

fn handle_fire(shared: &SharedData, input_data: &CommunicationData) -> String {
    // Verify the receipt is valid for the FIRE action
    if input_data.receipt.verify(FIRE_ID).is_err() {
        shared
            .tx
            .send("Attempting to fire with invalid receipt".to_string())
            .unwrap();
        return "Could not verify receipt".to_string();
    }

    // Decode the journal data from the receipt
    let data: FireJournal = input_data.receipt.journal.decode().unwrap();
    let mut gmap = shared.gmap.lock().unwrap();

    // Check if the game exists
    let game = match gmap.get_mut(&data.gameid) {
        Some(game) => game,
        None => {
            shared
                .tx
                .send(format!("Game {} not found", data.gameid))
                .unwrap();
            return "Game not found".to_string();
        }
    };
    // Check if the target player exists in the game
    if !game.pmap.contains_key(&data.target) {
        shared
            .tx
            .send(format!(
                "Target fleet {} not found in game {}",
                data.target, data.gameid
            ))
            .unwrap();
        return "Target not found".to_string();
    }
    // Check if it's this player's turn to fire
    if game.next_player != Some(data.fleet.clone()) {
        shared
            .tx
            .send(format!(
                "Not {}'s turn to fire in game {}",
                data.fleet, data.gameid
            ))
            .unwrap();
        return "Not your turn".to_string();
    }
    // Check if the player needs to report a shot result before they can fire
    if game.next_report == Some(data.fleet.clone()) {
        shared
            .tx
            .send(format!(
                "Player {} must report the result of the received shot before firing in game {}",
                data.fleet, data.gameid
            ))
            .unwrap();
        return "Must report before firing".to_string();
    }
    // Check that player is not targeting themselves
    if data.fleet == data.target {
        shared
            .tx
            .send(format!(
                "Player {} tried to target themselves in game {}",
                data.fleet, data.gameid
            ))
            .unwrap();
        return "Cannot target yourself".to_string();
    }
    // Check if board is correctly synced
    if data.board != game.pmap.get(&data.fleet).unwrap().current_state {
        shared
            .tx
            .send(format!(
                "Player {} tried to fire with a different board state in game {}",
                data.fleet, data.gameid
            ))
            .unwrap();
        return "Cannot fire with a different board state".to_string();
    }
    // TODO: check player's current state, if board is empty/no boats

    // Update game state - next player should be the target to report hit/miss
    // TODO: next_report should have more information about the shot, such as position
    game.next_player = Some(data.target.clone());
    game.next_report = Some(data.target.clone());
    game.next_shot = Some(data.pos);

    // Update turn counters
    update_turn_counters(game, &data.fleet);

    // Send notification about the fire action
    let pos_str = xy_pos(data.pos);
    shared
        .tx
        .send(format!(
            "Player {} fired at position {} targeting {} in game {}",
            data.fleet, pos_str, data.target, data.gameid
        ))
        .unwrap();

    // Notify whose turn it is next
    shared
        .tx
        .send(format!(
            "It's {}'s turn to report hit/miss in game {}",
            data.target, data.gameid
        ))
        .unwrap();

    "OK".to_string()
}

fn handle_report(shared: &SharedData, input_data: &CommunicationData) -> String {
    // Verify the receipt is valid for the REPORT action
    if input_data.receipt.verify(REPORT_ID).is_err() {
        shared
            .tx
            .send("Attempting to report with invalid receipt".to_string())
            .unwrap();
        return "Could not verify receipt".to_string();
    }

    // Decode the journal data from the receipt
    let data: ReportJournal = input_data.receipt.journal.decode().unwrap();
    let mut gmap = shared.gmap.lock().unwrap();

    // Check if the game exists
    let game = match gmap.get_mut(&data.gameid) {
        Some(game) => game,
        None => {
            shared
                .tx
                .send(format!("Game {} not found", data.gameid))
                .unwrap();
            return "Game not found".to_string();
        }
    };
    // Check if it's this player's turn to report
    if game.next_player != Some(data.fleet.clone()) {
        shared
            .tx
            .send(format!(
                "Not {}'s turn to report in game {}",
                data.fleet, data.gameid
            ))
            .unwrap();
        return "Not your turn to report".to_string();
    }
    // Check if we're expecting a report
    if game.next_report.is_none() {
        shared
            .tx
            .send(format!(
                "No pending shot to report on in game {}",
                data.gameid
            ))
            .unwrap();
        return "No pending shot".to_string();
    }
    // Check from data if position is the same as the previously fired position, receive from game.next_report
    if game.next_shot.is_none() || game.next_shot.unwrap() != data.pos {
        shared
            .tx
            .send(format!(
                "Reported position {} does not match expected shot position in game {}",
                xy_pos(data.pos),
                data.gameid
            ))
            .unwrap();
        return "Reported position mismatch".to_string();
    }
    // TODO: If miss, check if data.board_next is the same as game.pmap[data.target].current_state TO TEST
    if data.report.to_ascii_lowercase() == "miss" {
        if data.next_board != game.pmap.get(&data.fleet).unwrap().current_state {
            shared
                .tx
                .send(format!(
                    "Next board state does not match the target's current state in game {}",
                    data.gameid
                ))
                .unwrap();
            return "Next board state mismatch".to_string();
        }
    }

    // TODO: Check if the position is a boat that has already been hit before (maybe this part should be done in methods)
    if game
        .pmap
        .get(&data.fleet)
        .unwrap()
        .shots_hit
        .contains(&data.pos)
        && data.report.to_ascii_lowercase() != "water"
    {
        shared
            .tx
            .send(format!(
                "Bad report: Position {} was already hit, but reported as {} in game {}",
                xy_pos(data.pos),
                data.report,
                data.gameid
            ))
            .unwrap();
        return "Bad report".to_string();
    }

    // TODO: If hit, check if data.board is the same as game.pmap[data.target].current_state TO TEST!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!1

    if data.report.to_ascii_lowercase() == "hit" {
        if data.next_board == game.pmap.get(&data.fleet).unwrap().current_state {
            shared
                .tx
                .send(format!(
                    "Next board state matches the target's current state in game {}, but it should not",
                    data.gameid
                ))
                .unwrap();
            return "Next board state should not match".to_string();
        }
        // Add the shot position to the target player's shots_hit
        let target_player = game.pmap.get_mut(&data.fleet).unwrap();
        target_player.shots_hit.push(data.pos);
    }

    // Clone shooter before mutably borrowing game
    let shooter = game.next_report.as_ref().unwrap().clone();

    // Update the reporting player's board state
    let player = game.pmap.get_mut(&data.fleet).unwrap();
    player.current_state = data.next_board;

    // Format the position for display
    let pos_str = xy_pos(data.pos);

    // TODO: fix this, we receive HIT/MISS/WATER in data, no need for the => format! macro
    // Send notification about the report result (It is shit, maybe could be shorter)
    let result_message = match data.report.as_str().to_ascii_lowercase().as_str() {
        "hit" => format!(
            "Player {} reports HIT at position {} from player {} in game {}",
            data.fleet, pos_str, shooter, data.gameid
        ),
        "miss" => format!(
            "Player {} reports MISS at position {} from player {} in game {}",
            data.fleet, pos_str, shooter, data.gameid
        ),
        "water" => format!(
            "Player {} reports WATER (already hit) at position {} from player {} in game {}",
            data.fleet, pos_str, shooter, data.gameid
        ),
        _ => format!(
            "Player {} reports UNKNOWN RESULT at position {} from player {} in game {}",
            data.fleet, pos_str, shooter, data.gameid
        ),
    };

    shared.tx.send(result_message).unwrap();

    // Update the game state - next player should be chosen after report
    // Typically would go back to the shooter for their next move
    game.next_player = Some(shooter.clone());
    game.next_report = None;

    // Update turn counters
    update_turn_counters(game, &data.fleet);

    // TODO: Delete this if it anoys you, is debug message
    // Notify whose turn it is next
    shared
        .tx
        .send(format!(
            "It's {}'s turn to fire in game {}",
            shooter, data.gameid
        ))
        .unwrap();

    "OK".to_string()
}

fn handle_wave(shared: &SharedData, input_data: &CommunicationData) -> String {
    // Verify the receipt is valid for the WAVE action
    if input_data.receipt.verify(WAVE_ID).is_err() {
        shared
            .tx
            .send("Attempting to wave with invalid receipt".to_string())
            .unwrap();
        return "Could not verify receipt".to_string();
    }
    // Decode the journal data from the receipt
    let data: BaseJournal = input_data.receipt.journal.decode().unwrap();
    let mut gmap = shared.gmap.lock().unwrap();
    // Check if the game exists
    let game = match gmap.get_mut(&data.gameid) {
        Some(game) => game,
        None => {
            shared
                .tx
                .send(format!("Game {} not found", data.gameid))
                .unwrap();
            return "Game not found".to_string();
        }
    };
    // Check if the player exists in the game
    if !game.pmap.contains_key(&data.fleet) {
        shared
            .tx
            .send(format!(
                "Player {} not found in game {}",
                data.fleet, data.gameid
            ))
            .unwrap();
        return "Player not found".to_string();
    }
    // Check if it's this player's turn
    if game.next_player != Some(data.fleet.clone()) {
        shared
            .tx
            .send(format!(
                "Not {}'s turn to wave in game {}",
                data.fleet, data.gameid
            ))
            .unwrap();
        return "Not your turn to wave".to_string();
    }

    // Update turn counters
    update_turn_counters(game, &data.fleet);
    // get player's fleet with highest want_turn_count
    let mut max_want_turn_count = 0;
    let mut most_deserving_player = None;
    for (name, player) in game.pmap.iter() {
        if player.want_turn_count > max_want_turn_count {
            max_want_turn_count = player.want_turn_count;
            most_deserving_player = Some(name).clone();
        }
    }
    // Set the next player to the one with the highest want_turn_count
    game.next_player = most_deserving_player.cloned();
    // check if no players are available to take the next turn
    if most_deserving_player.is_none() {
        shared
            .tx
            .send("No players available to take the next turn".to_string())
            .unwrap();
        return "No players available".to_string();
    }
    // Notify all players about the wave action
    shared
        .tx
        .send(format!(
            "Player {} waved, next player is {}",
            data.fleet,
            most_deserving_player.as_ref().unwrap()
        ))
        .unwrap();
    "OK".to_string()
}

fn handle_win(shared: &SharedData, input_data: &CommunicationData) -> String {
    // Verify the receipt is valid for the WIN action
    if input_data.receipt.verify(WIN_ID).is_err() {
        shared
            .tx
            .send("Attempting to claim win with invalid receipt".to_string())
            .unwrap();
        return "Could not verify receipt".to_string();
    }

    // TODO: Check if the boards of all players are empty (all ships sunk)
    // get player from game
    let data: BaseJournal = input_data.receipt.journal.decode().unwrap();
    let mut gmap = shared.gmap.lock().unwrap();
    let game = match gmap.get_mut(&data.gameid) {
        Some(game) => game,
        None => {
            shared
                .tx
                .send(format!("Game {} not found", data.gameid))
                .unwrap();
            return "Game not found".to_string();
        }
    };

    // check if all players have shots_hit of length x = 10
    if game
        .pmap
        .values()
        .all(|player| player.shots_hit.len() == 1 || player.name == data.fleet)
    {
        shared
            .tx
            .send(format!(
                "Player {} claims victory in game {}",
                data.fleet, data.gameid
            ))
            .unwrap();
        return "OK".to_string();
    } else {
        shared
            .tx
            .send(format!(
                "Player {} tried to claim victory, but not all ships are sunk in game {}",
                data.fleet, data.gameid
            ))
            .unwrap();
        return "Not all ships are sunk".to_string();
    }
}
