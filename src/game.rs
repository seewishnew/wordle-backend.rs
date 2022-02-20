use rocket::{serde::{Serialize, Deserialize,}, request::FromParam, http::Status};
use mongodb::{bson::{oid::ObjectId, Bson, self}, Collection};

pub const DB: &'static str = "wordle";
pub const GAMES_COLLECTION: &'static str = "games";
pub const PLAYERS_FIELDNAME: &'static str = "players";
pub const PLAYERS_GUESSES_FIELDNAME: &'static str = "players.$.guesses";
pub const PLAYERS_ID_FIELDNAME: &'static str = "players._id";
pub const CREATOR_FIELDNAME: &'static str = "creator";

#[derive(Deserialize)]
pub struct CreateGameRequest<'r> {
    pub answer: &'r str,
}

pub struct GameConn(pub Collection<Game>);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub start_time: u64,
    pub creator: ObjectId,
    pub players: Vec<Player>,
    pub answer: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub name: String,
    pub start_time: u64,
    pub guesses: Vec<Guess>,
}

impl From<&Player> for Bson {
    fn from(player: &Player) -> Self {
        bson::to_bson(player).unwrap()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerResponse {
    pub name: String,
    pub start_time: u64,
    pub guesses: Vec<Guess>,
}

impl From<&Player> for PlayerResponse {
    fn from(player: &Player) -> PlayerResponse {
        Self {
            name: player.name.clone(),
            start_time: player.start_time,
            guesses: player.guesses.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Guess {
    pub guess: Vec<(char, Correctness)>,
    pub submit_time: u64,
}

impl From<&Guess> for Bson {
    fn from(guess: &Guess) -> Self {
        bson::to_bson(guess).unwrap()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Correctness {
    Correct,
    IncorrectPosition,
    Incorrect,
}

#[derive(Serialize, Deserialize)]
pub struct CreateGameResponse {
    pub game_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct ManageGameResponse {
    pub start_time: u64,
    pub players: Vec<PlayerResponse>,
    pub answer: String,
}

#[derive(Debug)]
pub struct GameIdParam(pub ObjectId);

impl FromParam<'_> for GameIdParam {
    type Error = Status;

    fn from_param(param: &'_ str) -> Result<Self, Self::Error> {
        match ObjectId::parse_str(param) {
            Ok(id) => Ok(Self(id)),
            Err(error) => {
                error!("Error while parsing game id: {error:?}");
                Err(Status::BadRequest)
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlayRequest {
    pub guess: Vec<char>
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlayResponse {
    pub game_over: bool,
    pub guess: Vec<(char, Correctness)>
}