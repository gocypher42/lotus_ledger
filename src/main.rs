mod error;

use std::str::FromStr;
use std::usize;

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::delete;
use axum::routing::get;
use axum::Json;
use axum::Router;
use bson::oid::ObjectId;
use bson::to_document;
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

async fn game_list(
    pagination: Query<Pagination>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let game_collection: Collection<Game> = state.db.collection(GAME_COLLECTION_NAME);

    let Ok(games_cursor) = game_collection.find(doc! {}).await else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let Ok(games): Result<Vec<Game>, _> = games_cursor
        .skip(pagination.offset.unwrap_or(0))
        .take(pagination.limit.unwrap_or(usize::MAX))
        .try_collect()
        .await
    else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    Ok(Json(games))
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateGame {
    player1: u8,
    player2: u8,
    player3: Option<u8>,
    player4: Option<u8>,
}

async fn game_create(
    State(state): State<AppState>,
    Json(game): Json<CreateGame>,
) -> Result<impl IntoResponse, StatusCode> {
    let game_collection: Collection<Game> = state.db.collection(GAME_COLLECTION_NAME);

    let Ok(inser_one_result) = game_collection.insert_one(Game::from(game)).await else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let Some(new_game) = game_collection
        .find_one(doc! {"_id": inser_one_result.inserted_id })
        .await
        .ok()
        .flatten()
    else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    Ok((StatusCode::CREATED, Json(new_game)))
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateGame {
    player1: u8,
    player2: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    player3: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    player4: Option<u8>,
}

async fn game_update(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(game): Json<UpdateGame>,
) -> Result<impl IntoResponse, StatusCode> {
    let Ok(id) = ObjectId::from_str(id.as_str()) else {
        return Err(StatusCode::NOT_FOUND);
    };
    let game_collection: Collection<Game> = state.db.collection(GAME_COLLECTION_NAME);

    let filter = doc! {"_id": id};

    let Ok(t) = game_collection
        .update_one(filter.clone(), doc! {"$set": to_document(&game).unwrap()})
        .await
    else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    if t.matched_count == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let Some(updated_game) = game_collection.find_one(filter).await.ok().flatten() else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    Ok(Json(updated_game))
}

async fn game_delete(Path(id): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    let Ok(id) = ObjectId::from_str(id.as_str()) else {
        return StatusCode::NOT_FOUND;
    };

    let Ok(res) = state
        .db
        .collection::<Game>(GAME_COLLECTION_NAME)
        .delete_many(doc! {
           "_id": id
        })
        .await
    else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };

    if res.deleted_count == 0 {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::NO_CONTENT
    }
}

#[tokio::main]
async fn main() -> error::Result<()> {
    let client = Client::with_uri_str("mongodb://localhost:27017").await?;
    let database = client.database(DB_NAME);

    let app_state = AppState { db: database };

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/games", get(game_list).post(game_create))
        .route("/games/{id}", delete(game_delete).put(game_update))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
