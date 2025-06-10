// Remove the following 3 lines to enable compiler checkings
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use axum::debug_handler;
use axum::{
    extract::Form,
    response::Html,
    routing::{get, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use nanoid::nanoid;
use std::path::PathBuf;
use tokio::signal;

use host::{fire, join_game, report, wave, win, FormData};
use std::net::SocketAddr;

#[debug_handler]
async fn index() -> Html<String> {
    render_html(None, None, None, None, None, None)
}

fn process_input_data(input_data: FormData) -> FormData {
    match &input_data.random {
        Some(random) if !random.is_empty() => input_data,
        _ => FormData {
            random: Some(nanoid!(12)),
            ..input_data
        },
    }
}

#[debug_handler]
async fn submit(Form(input_data): Form<FormData>) -> Html<String> {
    let gameid = input_data.gameid.clone();
    let fleetid = input_data.fleetid.clone();
    let data = process_input_data(input_data);
    let random = data.random.clone();
    let board = data.board.clone();
    let shots = data.shots.clone();
    let response_text = match data.button.as_str() {
        "Join" => join_game(data).await,
        "Fire" => fire(data).await,
        "Report" => report(data).await,
        "Wave" => wave(data).await,
        "Win" => win(data).await,
        _ => "Unknown button pressed".to_string(),
    };
    render_html(gameid, fleetid, random, board, shots, Some(response_text))
}

fn render_html(
    gameid: Option<String>,
    fleetid: Option<String>,
    random: Option<String>,
    board: Option<String>,
    shots: Option<String>,
    response: Option<String>,
) -> Html<String> {
    let fleetid = fleetid.unwrap_or("".to_string());
    let gameid = gameid.unwrap_or("".to_string());
    let response_html = if let Some(response) = response {
        if response == "OK" {
            if gameid != "" {
                format!(
                    "Playing Game: <b>{}</b> with fleet's ID: <b>{}</b> ",
                    gameid, fleetid
                )
            } else {
                "Not in game".to_string()
            }
        } else {
            format!("<p style='color:red'>{}</p>", response)
        }
    } else {
        "".to_string()
    };
    let random = random.unwrap_or("".to_string());

    let board = board.unwrap_or("".to_string());
    let shots = shots.unwrap_or("".to_string());

    let path = "host/src/page.html";
    let html = std::fs::read_to_string(path).unwrap();
    let html = html.replace("{response_html}", &response_html);
    let html = html.replace("{gameid}", &gameid);
    let html = html.replace("{fleetid}", &fleetid);
    let html = html.replace("{random}", &random);
    let html = html.replace("{board}", &board);
    let html = html.replace("{shots}", &shots);

    Html(html)
}

#[tokio::main]
async fn main() {
    // Initialize the default crypto provider for rustls
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install crypto provider");

    // Create state that implements Send + Sync for use with Axum
    let app = Router::new()
        .route("/", get(index))
        .route("/submit", post(submit));

    // Configure TLS
    let config = RustlsConfig::from_pem_file(
        PathBuf::from("certs/host-cert.pem"),
        PathBuf::from("certs/host-key.pem"),
    )
    .await
    .expect("Failed to load TLS certificates");

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Listening on https://{}", addr);

    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
