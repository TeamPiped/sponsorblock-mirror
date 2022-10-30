#[macro_use]
extern crate rocket;

use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};

use diesel::connection::SimpleConnection;
use rocket::{Build, Request, Response, Rocket};
use rocket::fairing::{AdHoc, Fairing, Info, Kind};
use rocket::http::Header;
use rocket_sync_db_pools::database;
use tokio::time::interval;

use structs::{Segment, Sponsor};

use crate::routes::{skip_segments, skip_segments_by_id, fake_is_user_vip, fake_user_info};

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

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
        if request.method() == rocket::http::Method::Options {
            response.set_streamed_body(tokio::io::empty());
            response.set_status(rocket::http::Status::Ok);
        }
    }
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
                                let res = db.run(move |c| {
                                    let result = c.batch_execute("BEGIN; DROP TABLE IF EXISTS \"sponsorTimesTemp\"; CREATE UNLOGGED TABLE \"sponsorTimesTemp\"(LIKE \"sponsorTimes\" INCLUDING defaults INCLUDING constraints INCLUDING indexes); COPY \"sponsorTimesTemp\" FROM '/mirror/sponsorTimes.csv' DELIMITER ',' CSV HEADER; DROP TABLE \"sponsorTimes\"; ALTER TABLE \"sponsorTimesTemp\" RENAME TO \"sponsorTimes\"; COMMIT;");
                                    if result.is_err() {
                                        c.batch_execute("ROLLBACK;").unwrap();
                                        eprintln!("Failed to import database: {}", result.err().unwrap());
                                        return false;
                                    }
                                    println!("Imported database in {}ms", start.elapsed().as_millis());
                                    // Vacuum the database
                                    let result = c.batch_execute("VACUUM \"sponsorTimes\";");
                                    if result.is_err() {
                                        eprintln!("Failed to vacuum database: {}", result.err().unwrap());
                                        return false;
                                    }

                                    true
                                }).await;

                                if res {
                                    unsafe { LAST_UPDATE = Some(last_modified) };
                                }
                            }

                            sleep(Duration::from_secs(60));
                        }
                    }
                });
            })
        })
        ).attach(CORS)
        .mount("/", routes![skip_segments, skip_segments_by_id, fake_is_user_vip, fake_user_info])
}
