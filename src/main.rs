use clap::Parser;
use log::{debug, error, info};
use rusqlite::Connection;
use std::{env, process, thread::sleep, time::Duration};
use vatsim_utils::live_api::Vatsim;

const DB_FILE_NAME: &str = "vatsim_data.db";

/// Simple app to record data from VATSIM to a database file
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Enable debug logging
    #[clap(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if args.debug {
        env::set_var("RUST_LOG", "info,vatsim_data_recorder=debug");
    } else {
        env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();
    debug!("Logger initialized");

    let new_db = !std::path::Path::new(DB_FILE_NAME).exists();
    let connection = match Connection::open(DB_FILE_NAME) {
        Ok(c) => c,
        Err(e) => {
            error!("Could not open database file {}: {}", DB_FILE_NAME, e);
            process::exit(1);
        }
    };
    if new_db {
        info!("Creating new database file");
        if let Err(e) = connection.execute(
            "CREATE TABLE entries (
                id INTEGER PRIMARY KEY,
                timestamp TEXT,
                pilots TEXT,
                controllers TEXT
            )",
            (),
        ) {
            error!("Could not create the database file: {}", e);
            process::exit(1);
        }
        debug!("DB create table successful");
    }

    debug!("Configuring VATSIM API");
    let vatsim = match Vatsim::new().await {
        Ok(v) => v,
        Err(e) => {
            error!("Could not get VATSIM API data: {}", e);
            process::exit(1);
        }
    };

    info!(
        "Entering recording loop. Interrupt or exit the program when you are finished recording."
    );
    loop {
        let data = match vatsim.get_v3_data().await {
            Ok(d) => d,
            Err(e) => {
                error!("Could not get data from VATSIM: {}", e);
                process::exit(1);
            }
        };
        if let Err(e) = connection.execute(
            "INSERT INTO entries (timestamp, pilots, controllers) VALUES (?1, ?2, ?3)",
            (
                chrono::Utc::now(),
                serde_json::to_string(&data.pilots).unwrap(),
                serde_json::to_string(&data.controllers).unwrap(),
            ),
        ) {
            error!("Could not store data in database file: {}", e);
            process::exit(1);
        }
        debug!("Data stored");
        sleep(Duration::from_secs(15));
    }
}
