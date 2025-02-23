mod error;

use std::usize;

use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Json;
use axum::Router;
use bson::oid::ObjectId;
use futures::StreamExt;
use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::Client;
use mongodb::Collection;
use mongodb::Database;
use serde::Deserialize;
use serde::Serialize;

const DB_NAME: &str = "lotus_ledger_db";
const GAME_COLLECTION_NAME: &str = "game";

#[derive(Clone)]
struct AppState {
    db: Database,
}

#[derive(Debug, Deserialize, Default)]
pub struct Pagination {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Game {
    #[serde(
        rename = "_id",
        serialize_with = "bson::serde_helpers::serialize_object_id_as_hex_string"
    )]
    id: ObjectId,
    player1: u8,
    player2: u8,
    player3: u8,
    player4: u8,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            id: ObjectId::new(),
            player1: 40,
            player2: 40,
            player3: 40,
            player4: 40,
        }
    }
}

impl From<CreateGame> for Game {
    fn from(req: CreateGame) -> Self {
        let mut game = Self::default();
        game.player1 = req.player1;
        game.player2 = req.player2;

        if let Some(player3) = req.player3 {
            game.player3 = player3;
        }

        if let Some(player4) = req.player4 {
            game.player4 = player4;
        }

        game
    }
}

async fn get_games(
    pagination: Query<Pagination>,
    State(state): State<AppState>,
) -> Result<Json<Vec<Game>>, impl IntoResponse> {
    let game_collection: Collection<Game> = state.db.collection(GAME_COLLECTION_NAME);

    let Ok(games_cursor) = game_collection.find(doc! {}).await else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unable to find collection."),
        ));
    };

    let Ok(games): Result<Vec<Game>, _> = games_cursor
        .skip(pagination.offset.unwrap_or(0))
        .take(pagination.limit.unwrap_or(usize::MAX))
        .try_collect()
        .await
    else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unable to parse collection."),
        ));
    };

    Ok(games.into())
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateGame {
    player1: u8,
    player2: u8,
    player3: Option<u8>,
    player4: Option<u8>,
}

async fn create_game(
    State(state): State<AppState>,
    Json(game): Json<CreateGame>,
) -> Result<(StatusCode, Json<Game>), (StatusCode, String)> {
    let game_collection: Collection<Game> = state.db.collection(GAME_COLLECTION_NAME);

    let Ok(inser_one_result) = game_collection.insert_one(Game::from(game)).await else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unable to create game."),
        ));
    };

    let Some(new_game) = game_collection
        .find_one(doc! {"_id": inser_one_result.inserted_id })
        .await
        .ok()
        .flatten()
    else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unable to create game."),
        ));
    };

    Ok((StatusCode::CREATED, Json(new_game)))
}

#[tokio::main]
async fn main() -> error::Result<()> {
    let client = Client::with_uri_str("mongodb://localhost:27017").await?;
    let database = client.database(DB_NAME);

    let app_state = AppState { db: database };

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route(
            "/games",
            get(get_games).post(create_game).with_state(app_state),
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    // let mut input_string = String::new();
    //
    // println!("Enter 'x' to quit.");
    // while input_string.trim() != "x" {
    //     input_string.clear();
    //     print!(">> ");
    //     io::stdout().flush().unwrap();
    //     io::stdin().read_line(&mut input_string).unwrap();
    //
    //     let cmd_parts: Vec<&str> = input_string.split(' ').map(str::trim).collect();
    //     let main_cmd = if cmd_parts.is_empty() {
    //         ""
    //     } else {
    //         cmd_parts.first().unwrap()
    //     };
    //
    //     if main_cmd == "create" {
    //         println!("Creating a new game");
    //         let inser_one_result = game_collection.insert_one(Game::default()).await?;
    //         let new_game_id = inser_one_result.inserted_id;
    //         println!(
    //             "new game id: {:?}",
    //             new_game_id.as_object_id().unwrap().to_string()
    //         );
    //     } else if main_cmd == "list" {
    //         println!("Listing games");
    //         let mut games_cursor = game_collection.find(doc! {}).await?;
    //         while let Some(game) = games_cursor.try_next().await? {
    //             println!("Game id: {}", game.id.unwrap());
    //         }
    //     } else if main_cmd == "delete" {
    //         if let Some(id) = cmd_parts.get(1) {
    //             println!("Deleting game with id \"{id}\"");
    //             let delete_result = game_collection
    //                 .delete_many(doc! {
    //                    "_id": ObjectId::from_str(id).unwrap()
    //                 })
    //                 .await?;
    //             println!("Deleted {} games", delete_result.deleted_count);
    //         } else {
    //             println!("No id given.");
    //         }
    //     } else if main_cmd == "delete_all" {
    //         println!("Deleting all game");
    //         let delete_result = game_collection.delete_many(doc! {}).await?;
    //         println!("Deleted {} games", delete_result.deleted_count);
    //     }
    //     println!("---");
    // }

    Ok(())
}
