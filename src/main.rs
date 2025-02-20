mod error;

use std::io;
use std::io::Write;
use std::str::FromStr;

use bson::oid::ObjectId;
use futures::TryStreamExt;
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
        Self {
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

    let mut input_string = String::new();

    println!("Enter 'x' to quit.");
    while input_string.trim() != "x" {
        input_string.clear();
        print!(">> ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input_string).unwrap();

        let cmd_parts: Vec<&str> = input_string.split(' ').map(str::trim).collect();
        let main_cmd = if cmd_parts.is_empty() {
            ""
        } else {
            cmd_parts.first().unwrap()
        };

        if main_cmd == "create" {
            println!("Creating a new game");
            let inser_one_result = game_collection.insert_one(Game::default()).await?;
            let new_game_id = inser_one_result.inserted_id;
            println!(
                "new game id: {:?}",
                new_game_id.as_object_id().unwrap().to_string()
            );
        } else if main_cmd == "list" {
            println!("Listing games");
            let mut games_cursor = game_collection.find(doc! {}).await?;
            while let Some(game) = games_cursor.try_next().await? {
                println!("Game id: {}", game.id.unwrap());
            }
        } else if main_cmd == "delete" {
            if let Some(id) = cmd_parts.get(1) {
                println!("Deleting game with id \"{id}\"");
                let delete_result = game_collection
                    .delete_many(doc! {
                       "_id": ObjectId::from_str(id).unwrap()
                    })
                    .await?;
                println!("Deleted {} games", delete_result.deleted_count);
            } else {
                println!("No id given.");
            }
        } else if main_cmd == "delete_all" {
            println!("Deleting all game");
            let delete_result = game_collection.delete_many(doc! {}).await?;
            println!("Deleted {} games", delete_result.deleted_count);
        }
        println!("---");
    }

    Ok(())
}
