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
use futures::stream::StreamExt;
use rand::{seq::IteratorRandom, SeedableRng};
use risc0_zkvm::Digest;
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

struct Player {
    name: String,
    current_state: Digest,
}
struct Game {
    pmap: HashMap<String, Player>,
    next_player: Option<String>,
    next_report: Option<String>,
}

#[derive(Clone)]
struct SharedData {
    tx: broadcast::Sender<String>,
    gmap: Arc<Mutex<HashMap<String, Game>>>,
    rng: Arc<Mutex<rand::rngs::StdRng>>,
}

#[tokio::main]
async fn main() {
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

    // Run our app with hyper
    //let addr = SocketAddr::from(([127, 0, 0, 1], 3001));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    println!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
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

fn xy_pos(pos: u8) -> String {
    let x = pos % 10;
    let y = pos / 10;
    format!("{}{}", (x + 65) as char, y)
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
    let game = gmap.entry(data.gameid.clone()).or_insert(Game {
        pmap: HashMap::new(),
        next_player: Some(data.fleet.clone()),
        next_report: None,
    });
    let player_inserted = game
        .pmap
        .entry(data.fleet.clone())
        .or_insert_with(|| Player {
            name: data.fleet.clone(),
            current_state: data.board.clone(),
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

    // TODO: check player's current state, if board is empty
    // Update the player's board statße
    let player = game.pmap.get_mut(&data.fleet).unwrap();
    player.current_state = data.board;

    // Update game state - next player should be the target to report hit/miss
    // TODO: next_report should have more information about the shot, such as position
    game.next_player = Some(data.target.clone());
    game.next_report = Some(data.target.clone());

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

    // TODO: Check from data if position is the same as the previously fired position, receive from game.next_report

    // TODO: Check from data if report is miss or hit, compare with game.next_report

    // TODO: If miss, check if data.board_next is the same as game.pmap[data.target].current_state

    // TODO: If hit, check if data.board is the same as game.pmap[data.target].current_state

    // TODO: Any other checks?

    // Clone shooter before mutably borrowing game
    let shooter = game.next_report.as_ref().unwrap().clone();

    // Update the reporting player's board state
    let player = game.pmap.get_mut(&data.fleet).unwrap();
    player.current_state = data.board;

    // Format the position for display
    let pos_str = xy_pos(data.pos);

    // TODO: fix this, we receive HIT/MISS in data, no need for the => format! macro
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

    // For now, just log that a wave was received
    shared.tx.send("Wave signal received".to_string()).unwrap();

    // TODO: Debug message, could be removed
    // If we know who should play next, notify them
    if let Some(game) = shared.gmap.lock().unwrap().get(
        &input_data
            .receipt
            .journal
            .decode::<BaseJournal>()
            .unwrap()
            .gameid,
    ) {
        if let Some(next_player) = &game.next_player {
            shared
                .tx
                .send(format!("It's {}'s turn to play", next_player))
                .unwrap();
        }
    }

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

    // For now, just log that a win was claimed
    // In a complete implementation, you would verify the win condition
    shared
        .tx
        .send(format!("Player claims victory in game"))
        .unwrap();

    "OK".to_string()
}
