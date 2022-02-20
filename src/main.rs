use log::{info};
#[macro_use]
extern crate rocket;

use mongodb::{
    options::{ClientOptions, ResolverConfig},
    Client,
};

use crate::{game::{DB, GameConn, GAMES_COLLECTION}, user::{UserConn, USERS_COLLECTION}};
mod game;
mod mongo_utils;
mod user;
mod routes;

use crate::routes::*;

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
    let client_uri = "mongodb://localhost:27017";
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
        .manage(GameConn(client.database(DB).collection(GAMES_COLLECTION)))
        .manage(UserConn(client.database(DB).collection(USERS_COLLECTION)))
        .mount(
            "/",
            routes![index, create_game, manage_game, register, user_id, play],
        )
}
