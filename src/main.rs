#[macro_use]
extern crate rocket;

use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};

use diesel::connection::SimpleConnection;
use rocket::{Build, Rocket};
use rocket::fairing::AdHoc;
use rocket_sync_db_pools::database;
use tokio::time::interval;

use structs::{Segment, Sponsor};

use crate::routes::skip_segments;

mod models;
mod routes;
mod schema;
mod structs;

#[database("sponsorblock")]
pub struct Db(diesel::PgConnection);

async fn run_migrations(rocket: Rocket<Build>) -> Rocket<Build> {
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");
    Db::get_one(&rocket)
        .await
        .expect("Failed to get a database connection")
        .run(|c| {
            MigrationHarness::run_pending_migrations(c, MIGRATIONS)
                .expect("Failed to run migrations");
        }).await;

    rocket
}

static mut LAST_UPDATE: Option<SystemTime> = None;

#[launch]
fn rocket() -> Rocket<Build> {
    rocket::build()
        .attach(Db::fairing())
        .attach(AdHoc::on_ignite("Diesel Migrations", run_migrations))
        .attach(AdHoc::on_liftoff("background database", |rocket| {
            Box::pin(async move {
                let mut interval = interval(Duration::from_secs(30));
                let path = Path::new("mirror/sponsorTimes.csv");

                // Get an actual DB connection
                let db = Db::get_one(rocket).await.unwrap();

                tokio::spawn(async move {
                    loop {
                        interval.tick().await;
                        let last_update = unsafe { LAST_UPDATE };

                        // see if file exists
                        if path.exists() && (last_update.is_none() || last_update.unwrap().elapsed().unwrap_or_default().as_secs() > 60) {

                            // Check last modified time
                            let last_modified = path.metadata().unwrap().modified().unwrap();

                            // Check if file was modified since last update
                            if last_update.is_none() || last_modified > last_update.unwrap() {

                                // Use COPY FROM to import the CSV file
                                let start = Instant::now();
                                println!("Importing database...");
                                // Execute a query of some kind
                                db.run(move |c| {
                                    let result = c.batch_execute("BEGIN TRANSACTION; DROP TABLE IF EXISTS \"sponsorTimesTemp\"; CREATE UNLOGGED TABLE \"sponsorTimesTemp\"(LIKE \"sponsorTimes\" INCLUDING defaults INCLUDING constraints INCLUDING indexes); COPY \"sponsorTimesTemp\" FROM '/mirror/sponsorTimes.csv' DELIMITER ',' CSV HEADER; DROP TABLE \"sponsorTimes\"; ALTER TABLE \"sponsorTimesTemp\" RENAME TO \"sponsorTimes\"; COMMIT;");
                                    if result.is_err() {
                                        eprintln!("Failed to import database: {}", result.err().unwrap());
                                    }
                                }).await;
                                println!("Imported database in {}ms", start.elapsed().as_millis());

                                unsafe {
                                    LAST_UPDATE = Some(last_modified);
                                }
                            }

                            sleep(Duration::from_secs(60));
                        }
                    }
                });
            })
        })
        ).mount("/", routes![skip_segments])
}
