use log::info;
#[macro_use]
extern crate rocket;
use mongodb::{
    options::{ClientOptions, ResolverConfig},
    Client,
};
use rocket::Config;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use crate::{
    game::{GameConn, DB, GAMES_COLLECTION},
    user::{UserConn, USERS_COLLECTION},
};
mod game;
mod mongo_utils;
mod routes;
mod user;

use crate::routes::*;

const MONGO_URI: &'static str = "MONGO_URI";
const SECRET_KEY: &'static str = "SECRET_KEY";

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}

#[launch]
async fn rocket() -> _ {
    setup_logger().unwrap();
    let client_uri = format!(
        "mongodb://{}",
        std::env::var(MONGO_URI).unwrap_or("localhost:27017".to_owned())
    );
    let secret_key = std::env::var(SECRET_KEY).unwrap();

    let options =
        ClientOptions::parse_with_resolver_config(client_uri, ResolverConfig::cloudflare())
            .await
            .unwrap();
    let client = Client::with_options(options).unwrap();

    info!("Connected to mongodb!");
    info!("Databases:");
    for name in client.list_database_names(None, None).await {
        info!("- {name:?}");
    }

    rocket::build()
        .configure(Config::figment().merge(("secret_key", secret_key)))
        .manage(GameConn(client.database(DB).collection(GAMES_COLLECTION)))
        .manage(UserConn(client.database(DB).collection(USERS_COLLECTION)))
        .mount(
            "/",
            routes![
                index,
                create_game,
                manage_game,
                register,
                user_id,
                play,
                get_state
            ],
        )
}
