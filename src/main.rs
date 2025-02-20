mod error;

use bson::oid::ObjectId;
use mongodb::bson::doc;
use mongodb::Client;
use mongodb::Collection;
use serde::Deserialize;
use serde::Serialize;

use error::Result;

const DB_NAME: &str = "lotus_ledger_db";
const GAME_COLLECTION_NAME: &str = "game";

#[derive(Debug, Serialize, Deserialize)]
struct Game {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    player1: u8,
    player2: u8,
    player3: u8,
    player4: u8,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            id: None,
            player1: 40,
            player2: 40,
            player3: 40,
            player4: 40,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::with_uri_str("mongodb://localhost:27017").await?;

    let database = client.database(DB_NAME);
    let game_collection: Collection<Game> = database.collection(GAME_COLLECTION_NAME);

    game_collection.insert_one(Game::default()).await?;

    Ok(())
}
