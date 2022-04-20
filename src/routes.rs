use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use crate::mongo_utils::{ID, NOT_EQUAL, PUSH};
use crate::{
    game::GameIdParam,
    user::{User, UserConn},
};

use crate::game::{
    Correctness, CreateGameRequest, CreateGameResponse, Game, GameConn, GetStateResponse, Guess,
    ManageGameResponse, PlayRequest, PlayResponse, Player, PlayerResponse, CREATOR_FIELDNAME,
    PLAYERS_FIELDNAME, PLAYERS_GUESSES_FIELDNAME, PLAYERS_ID_FIELDNAME,
};
use crate::user::{CreateUserIdRequest, COOKIE_USER_ID};
use mongodb::{
    bson::{doc, oid::ObjectId},
    results::InsertOneResult,
};
use rocket::{
    http::{Cookie, CookieJar, SameSite, Status},
    serde::json::Json,
    State,
};

macro_rules! parse_user_id {
    ($cookie:ident,$user_id:ident, $block:block) => {
        if let Some(user_id) = $cookie
            .get_private(COOKIE_USER_ID)
            .map(|crumb| crumb.value().to_owned())
        {
            match ObjectId::parse_str(user_id) {
                Ok($user_id) => $block,
                Err(error) => {
                    error!("Could not obtain user id from cookie! {error:?}");
                    Err(Status::Unauthorized)
                }
            }
        } else {
            error!("Could not obtain user id from cookie!");
            Err(Status::Unauthorized)
        }
    };
}

#[post("/create", data = "<req>")]
pub async fn create_game(
    cookies: &CookieJar<'_>,
    conn: &State<GameConn>,
    req: Json<CreateGameRequest<'_>>,
) -> Result<Json<CreateGameResponse>, Status> {
    parse_user_id!(cookies, user_id, {
        let Json(CreateGameRequest { answer }) = req;
        let game = Game {
            id: ObjectId::new(),
            creator: user_id,
            answer: answer.into(),
            players: Vec::new(),
            start_time: chrono::Utc::now().timestamp_millis() as u64,
        };

        match conn.0.insert_one(&game, None).await {
            Ok(InsertOneResult { inserted_id, .. }) => {
                info!("Created game {inserted_id:?}");
                Ok(Json::from(CreateGameResponse {
                    game_id: game.id.to_hex(),
                }))
            }
            Err(error) => {
                error!("Error occured during insert: {error:?}");
                Err(Status::InternalServerError)
            }
        }
    })
}

#[get("/manage/<game_id>")]
pub async fn manage_game(
    cookies: &CookieJar<'_>,
    user_conn: &State<UserConn>,
    game_conn: &State<GameConn>,
    game_id: GameIdParam,
) -> Result<Json<ManageGameResponse>, Status> {
    parse_user_id!(cookies, user_id, {
        match user_conn.0.find_one(doc! {ID: user_id.clone()}, None).await {
            Ok(Some(user)) => {
                match game_conn
                    .0
                    .find_one(doc! {ID: game_id.0, CREATOR_FIELDNAME: user.id}, None)
                    .await
                {
                    Ok(Some(game)) => Ok(Json::from(ManageGameResponse {
                        start_time: game.start_time,
                        players: game
                            .players
                            .iter()
                            .map(|player| PlayerResponse::from(player))
                            .collect(),
                        answer: game.answer,
                    })),
                    Ok(None) => {
                        error!("Could not find game id {game_id:?}");
                        Err(Status::BadRequest)
                    }
                    Err(error) => {
                        error!("Error while fetching game details for id {game_id:?}: {error:?}");
                        Err(Status::InternalServerError)
                    }
                }
            }
            Ok(None) => {
                error!("Could not find user id: {user_id:?}");
                Err(Status::Unauthorized)
            }
            Err(error) => {
                error!("Something went wrong accessing user id: {user_id:?}: {error:?}");
                Err(Status::InternalServerError)
            }
        }
    })
}

#[post("/user_id", data = "<req>")]
pub async fn user_id(
    cookies: &CookieJar<'_>,
    user_conn: &State<UserConn>,
    req: Json<CreateUserIdRequest>,
) -> Result<Status, Status> {
    let Json(CreateUserIdRequest { name }) = req;
    let user_id = cookies
        .get_private(COOKIE_USER_ID)
        .map_or(String::new(), |crumb| crumb.value().to_owned());
    let user_id = ObjectId::parse_str(user_id).unwrap_or(ObjectId::new());
    match user_conn
        .0
        .find_one(doc! {ID: user_id}, None)
        .await
        .map_err(|error| {
            error!("Could not find user: {error:?}");
            Status::InternalServerError
        })? {
        None => {
            let user = User {
                id: ObjectId::new(),
                name,
            };

            user_conn.0.insert_one(&user, None).await.map_err(|error| {
                error!("Could not create new user: {user:?}: {error:?}");
                Status::InternalServerError
            })?;
            cookies.add_private(
                Cookie::build(COOKIE_USER_ID, user.id.to_string())
                    .http_only(true)
                    .permanent()
                    .same_site(SameSite::Lax)
                    .finish(),
            );
            Ok(Status::Ok)
        }
        Some(user) => {
            info!("Received user_id request with an existing user: {user:?}");
            Ok(Status::Ok)
        }
    }
}

#[get("/user_id/verify")]
pub async fn verify_user_id(
    cookies: &CookieJar<'_>,
    user_conn: &State<UserConn>,
) -> Result<Status, Status> {
    let user_id = cookies
        .get_private(COOKIE_USER_ID)
        .map_or(String::new(), |crumb| crumb.value().to_owned());
    let user_id = ObjectId::parse_str(user_id).map_err(|_| Status::Unauthorized)?;
    info!("Got user_id: {user_id:?}");
    match user_conn
        .0
        .find_one(doc! {ID: user_id }, None)
        .await
        .map_err(|error| {
            error!("Could not find user: {error:?}");
            Status::InternalServerError
        })? {
        None => {
            error!("Unauthorized!");
            Err(Status::Unauthorized)
        }
        Some(user) => {
            info!("Received user_id request with an existing user: {user:?}");
            Ok(Status::Ok)
        }
    }
}

#[post("/game/<game_id>/register")]
pub async fn register(
    cookies: &CookieJar<'_>,
    user_conn: &State<UserConn>,
    game_conn: &State<GameConn>,
    game_id: GameIdParam,
) -> Result<Status, Status> {
    parse_user_id!(cookies, user_id, {
        let user = user_conn
            .0
            .find_one(doc! {ID: user_id}, None)
            .await
            .map_err(|error| {
                error!("Error while finding user for player id: {user_id:?}: {error:?}");
                Status::InternalServerError
            })?;
        if let Some(user) = user {
            let player = Player {
                id: user.id,
                name: user.name,
                guesses: Vec::new(),
                start_time: chrono::Utc::now().timestamp_millis() as u64,
            };

            let res = game_conn
                        .0
                        .update_one(
                            doc! {ID: game_id.0, CREATOR_FIELDNAME: doc!{NOT_EQUAL: user_id}, PLAYERS_ID_FIELDNAME: doc!{NOT_EQUAL: player.id}},
                            doc! {PUSH: doc!{PLAYERS_FIELDNAME: &player}},
                            None,
                        )
                        .await
                        .map_err(|error| {
                            error!("Error while updating game id: {game_id:?} with new player information: {player:?}: {error:?}");
                            Status::InternalServerError
                        })?;

            if res.matched_count != 1 {
                error!("Error: game_id {game_id:?} not found!");
                Err(Status::BadRequest)
            } else if res.modified_count != 1 {
                error!("Could not update game_id {game_id:?}");
                Err(Status::InternalServerError)
            } else {
                Ok(Status::Ok)
            }
        } else {
            error!("No user found matching player id: {user_id:?}");
            Err(Status::Unauthorized)
        }
    })
}

fn eval(answer: Vec<char>, guess: Vec<char>) -> PlayResponse {
    if answer == guess {
        PlayResponse {
            game_over: true,
            guess: guess.iter().map(|&ch| (ch, Correctness::Correct)).collect(),
        }
    } else {
        let mut unused: HashSet<char> = HashSet::new();
        let mut guess_eval: Vec<(char, Correctness)> = guess
            .into_iter()
            .zip(answer.iter())
            .map(|(gch, &ach)| {
                (
                    gch,
                    if ach == gch {
                        Correctness::Correct
                    } else {
                        unused.insert(ach);
                        Correctness::Incorrect
                    },
                )
            })
            .collect();
        guess_eval
            .iter_mut()
            .filter(|ge| ge.1 == Correctness::Incorrect)
            .for_each(|ge| {
                if unused.remove(&ge.0) {
                    ge.1 = Correctness::IncorrectPosition;
                }
            });
        PlayResponse {
            game_over: false,
            guess: guess_eval,
        }
    }
}

#[get("/game/<game_id>/state")]
pub async fn get_state(
    cookies: &CookieJar<'_>,
    game_conn: &State<GameConn>,
    game_id: GameIdParam,
) -> Result<Json<GetStateResponse>, Status> {
    parse_user_id!(cookies, user_id, {
        match game_conn
            .0
            .find_one(doc! {ID: game_id.0, PLAYERS_ID_FIELDNAME: user_id}, None)
            .await
        {
            Ok(Some(game)) => {
                let player = game
                    .players
                    .iter()
                    .find_map(|player| {
                        if player.id == user_id {
                            Some(player.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap();
                let guesses: Vec<Vec<(char, Correctness)>> = player
                    .guesses
                    .iter()
                    .map(|guess| guess.guess.clone())
                    .collect();
                let game_over = guesses.len() == 6
                    || guesses
                        .iter()
                        .last()
                        .map(|guess| {
                            guess
                                .iter()
                                .all(|&(_, correctness)| correctness == Correctness::Correct)
                        })
                        .unwrap_or(false);
                let resp = GetStateResponse { game_over, guesses };
                Ok(Json::from(resp))
            }
            Ok(None) => {
                error!("Could not find game_id {game_id:?} with user_id {user_id}");
                Err(Status::BadRequest)
            }
            Err(error) => {
                error!("Error occurred when trying to get game_id {game_id:?} for user_id {user_id}: {error:?}");
                Err(Status::InternalServerError)
            }
        }
    })
}

#[post("/game/<game_id>/play", data = "<guess>")]
pub async fn play(
    cookies: &CookieJar<'_>,
    game_conn: &State<GameConn>,
    game_id: GameIdParam,
    guess: Json<PlayRequest>,
) -> Result<Json<PlayResponse>, Status> {
    let Json(PlayRequest { guess }) = guess;
    info!("guess: {guess:?}");

    parse_user_id!(cookies, user_id, {
        match game_conn
            .0
            .find_one(doc! {ID: game_id.0, PLAYERS_ID_FIELDNAME: user_id}, None)
            .await
        {
            Ok(Some(game)) => {
                let resp = eval(game.answer.chars().collect(), guess);
                let guess = Guess {
                    guess: resp.guess.clone(),
                    submit_time: chrono::Utc::now().timestamp_millis() as u64,
                };
                match game_conn
                    .0
                    .update_one(
                        doc! {ID: game_id.0, PLAYERS_ID_FIELDNAME: user_id},
                        doc! {PUSH: {PLAYERS_GUESSES_FIELDNAME: &guess}},
                        None,
                    )
                    .await
                {
                    Ok(update_res) => {
                        if update_res.modified_count == 1 {
                            Ok(Json::from(resp))
                        } else if update_res.matched_count == 1 {
                            Err(Status::Conflict)
                        } else {
                            Err(Status::InternalServerError)
                        }
                    }
                    Err(error) => {
                        error!("Error occurred: {error:?}");
                        Err(Status::InternalServerError)
                    }
                }
            }
            Ok(None) => {
                error!("game_id: {game_id:?} not found with registered user_id: {user_id}");
                Err(Status::BadRequest)
            }
            Err(error) => {
                error!("Error occurred while accessing game id {game_id:?}: {error:?}");
                Err(Status::InternalServerError)
            }
        }
    })
}

#[get("/")]
pub fn index() -> &'static str {
    "Hello, world!"
}
