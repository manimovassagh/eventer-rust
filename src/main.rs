use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use actix_web::http::header;
use actix_cors::Cors;
use rand::Rng;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio::time::{self, Duration};
use futures_util::StreamExt; // Import StreamExt for handling streams

// ScoreData represents the game score
#[derive(Serialize, Clone)]
struct ScoreData {
    score: Score,
}

#[derive(Serialize, Clone)]
struct Score {
    team1: u32,
    team2: u32,
}

// Shared app state
struct AppState {
    broadcaster: broadcast::Sender<ScoreData>,
}

// Endpoint for clients to receive live updates via SSE
#[get("/events")]
async fn events(data: web::Data<AppState>) -> impl Responder {
    let  rx = data.broadcaster.subscribe();

    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "text/event-stream"))
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .insert_header((header::CONNECTION, "keep-alive"))
        .streaming(tokio_stream::wrappers::BroadcastStream::new(rx).map(|msg| {
            match msg {
                Ok(score_data) => Ok::<web::Bytes, actix_web::Error>(web::Bytes::from(
                    format!("data: {}\n\n", serde_json::to_string(&score_data).unwrap())
                )),
                Err(_) => Ok::<web::Bytes, actix_web::Error>(web::Bytes::from("data: {}\n\n")),
            }
        }))
}

// Function to simulate game and broadcast updates
async fn simulate_game(broadcaster: broadcast::Sender<ScoreData>) {
    let mut interval = time::interval(Duration::from_millis(500));
    let mut score = ScoreData {
        score: Score { team1: 0, team2: 0 },
    };

    loop {
        interval.tick().await;
        let mut rng = rand::thread_rng();

        // Randomly update the score
        if rng.gen::<f32>() < 0.3 {
            score.score.team1 += 1;
        }
        if rng.gen::<f32>() < 0.3 {
            score.score.team2 += 1;
        }

        // Broadcast the updated score to all clients
        let _ = broadcaster.send(score.clone());
    }
}

// Main function to start the server and set up the game simulation
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (tx, _rx) = broadcast::channel(16);
    let state = web::Data::new(AppState { broadcaster: tx.clone() });

    // Start game simulation in a separate task
    tokio::spawn(simulate_game(tx));

    // Start HTTP server with CORS configuration
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin() // Allow requests from any origin
            .allowed_methods(vec!["GET", "POST"]) // Restrict to specific HTTP methods
            .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
            .supports_credentials();

        App::new()
            .wrap(cors)
            .app_data(state.clone())
            .service(events)
    })
    .bind(("127.0.0.1", 3001))?
    .run()
    .await
}