use crate::mongo_utils::{ID, NOT_EQUAL, PUSH};
use crate::{
    game::GameIdParam,
    user::{User, UserConn},
};

use crate::game::{
    CreateGameRequest, CreateGameResponse, Game, GameConn, Guess, ManageGameResponse, PlayRequest,
    Player, CREATOR_FIELDNAME, PLAYERS_FIELDNAME, PLAYERS_GUESSES_FIELDNAME, PLAYERS_ID_FIELDNAME,
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
                Ok(Json::from(CreateGameResponse { game_id: game.id }))
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
                        players: game.players,
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
    let user = User {
        id: ObjectId::new(),
        name,
    };

    match user_conn.0.insert_one(&user, None).await {
        Ok(_) => {
            cookies.add_private(
                Cookie::build(COOKIE_USER_ID, user.id.to_string())
                    .http_only(true)
                    .permanent()
                    .same_site(SameSite::Lax)
                    .finish(),
            );
            Ok(Status::Ok)
        }
        Err(error) => {
            error!("Could not create new user: {user:?}: {error:?}");
            Err(Status::InternalServerError)
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
        match user_conn.0.find_one(doc! {ID: user_id}, None).await {
            Ok(Some(user)) => {
                let player = Player {
                    id: user.id,
                    name: user.name,
                    guesses: Vec::new(),
                    start_time: chrono::Utc::now().timestamp_millis() as u64,
                };

                match game_conn
                    .0
                    .update_one(
                        doc! {ID: game_id.0, CREATOR_FIELDNAME: doc!{NOT_EQUAL: user_id}},
                        doc! {PUSH: doc!{PLAYERS_FIELDNAME: &player}},
                        None,
                    )
                    .await
                {
                    Ok(res) => {
                        if res.modified_count == 1 {
                            Ok(Status::Ok)
                        } else if res.matched_count == 0 {
                            Err(Status::BadRequest)
                        } else {
                            Err(Status::InternalServerError)
                        }
                    }
                    Err(error) => {
                        error!("Error while updating game id: {game_id:?} with new player information: {player:?}: {error:?}");
                        Err(Status::InternalServerError)
                    }
                }
            }
            Ok(None) => {
                error!("No user found matching player id: {user_id:?}");
                Err(Status::Unauthorized)
            }
            Err(error) => {
                error!("Error while finding user for player id: {user_id:?}: {error:?}");
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
) -> Result<Status, Status> {
    let Json(PlayRequest { guess }) = guess;
    info!("guess: {guess:?}");
    let guess = Guess {
        guess,
        submit_time: chrono::Utc::now().timestamp_millis() as u64,
    };

    parse_user_id!(cookies, user_id, {
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
                    Ok(Status::Ok)
                } else if update_res.matched_count == 0 {
                    Err(Status::BadRequest)
                } else if update_res.matched_count == 1 {
                    Err(Status::Conflict)
                } else {
                    Err(Status::InternalServerError)
                }
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
