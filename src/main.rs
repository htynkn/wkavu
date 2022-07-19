#[macro_use]
extern crate clap;
extern crate cronjob;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate rbatis;
#[macro_use]
extern crate rust_embed;
extern crate tinytemplate;

use async_std::task;
use std::path::Path;

use actix_cors::Cors;
use actix_web::{web, App as wApp, HttpServer};
use clap::{App, Arg};
use cronjob::CronJob;
use env_logger::Env;
use log::info;
use rbatis::core::db::db_adapter::DBPool::Sqlite;
use rbatis::crud::CRUD;

use crate::model::Tv;
use crate::resolver::Resolver;

mod global;
mod http;
mod model;
mod resolver;
mod torznab;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if Path::new("log4rs.yml").exists() {
        log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    } else {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    }

    info!("Starting...");

    let matches = App::new("wkavu")
        .version("1.0")
        .arg(
            Arg::with_name("db")
                .long("db")
                .value_name("DB_URL")
                .env("DB_URL")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("static-folder")
                .long("static-folder")
                .value_name("STATIC_FOLDER")
                .env("STATIC_FOLDER")
                .takes_value(true),
        )
        .get_matches();

    let db_url = matches.value_of("db").unwrap_or("sqlite://:memory:");
    let static_folder = matches
        .value_of("static-folder")
        .unwrap_or("./webapp")
        .to_string();

    global::RB.link(db_url).await.unwrap();

    let db_pool = global::RB.get_pool().unwrap();
    if let Sqlite(pool, _) = db_pool {
        info!("Migration running...");
        sqlx::migrate!("./migrations").run(pool).await.unwrap();
    }

    let tvs: Vec<Tv> = global::RB.fetch_list().await.unwrap();
    info!("tv show size:{}", tvs.len());

    std::thread::spawn(|| {
        let mut cron = CronJob::new("Fetch", on_cron);
        cron.seconds("0");
        cron.minutes("0,30");
        cron.start_job();
    });

    HttpServer::new(move || {
        let cors = Cors::permissive();
        wApp::new()
            .wrap(cors)
            .route("/", web::get().to(http::root))
            .route("/health", web::get().to(http::health))
            .route("/api", web::get().to(http::api))
            .route("/admin/fetch", web::get().to(http::refresh))
            .route("/admin/tvs", web::get().to(http::tv_list))
            .route("/admin/seeds/tvid/{tvid}", web::get().to(http::seed_list))
            .route("/admin/tvs", web::post().to(http::tv_add))
            .route("/admin/tvs/delete", web::post().to(http::tv_delete))
            .service(actix_files::Files::new("/", &static_folder).index_file("index.html"))
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}

fn on_cron(_name: &str) {
    task::spawn(async {
        info!("start fetching task...");

        let want = global::WANT.lock().unwrap().clone();
        global::WANT.lock().unwrap().clear();
        for tvdbid in want.iter() {
            let resolver = Resolver::new();
            let wrapper = global::RB.new_wrapper().eq(Tv::tvdbid(), tvdbid);

            let tv: Option<Tv> = global::RB.fetch_by_wrapper(wrapper).await.unwrap();
            if tv.is_some() {
                let id = tv.unwrap().id.unwrap();
                resolver.fetch_by_tv(id).await;
            }
        }
    });
}
