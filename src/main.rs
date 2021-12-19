#[macro_use]
extern crate clap;
extern crate cronjob;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate rbatis;
extern crate tinytemplate;

use std::thread;

use actix_cors::Cors;
use actix_web::{App as wApp, get, HttpRequest, HttpResponse, HttpServer, post, Responder, web};
use actix_web::http::header::ContentType;
use async_std::task;
use clap::{App, Arg};
use cronjob::CronJob;
use log::{error, info, warn};
use rbatis::{Page, PageRequest};
use rbatis::core::db::db_adapter::DBPool::Sqlite;
use rbatis::crud::CRUD;

use crate::model::{OperationResponse, PageResponse, Tv, TvSeed};
use crate::resolver::{Domp4Resolver, Resolver};
use crate::torznab::TorznabProvider;

mod global;
mod model;
mod resolver;
mod torznab;

async fn root() -> HttpResponse {
    HttpResponse::Ok().body("server is up!")
}

#[derive(serde::Deserialize, Debug)]
pub struct ApiRequest {
    t: String,
    q: Option<String>,
    tvdbid: Option<String>,
    season: Option<i32>,
    ep: Option<i32>,
    offset: Option<u64>,
    limit: Option<u64>,
}

#[derive(serde::Deserialize, Debug)]
pub struct TvsRequest {
    perPage: Option<u64>,
    page: Option<u64>,
}

#[derive(serde::Deserialize, Debug)]
pub struct SeedsRequest {
    perPage: Option<u64>,
    page: Option<u64>,
}

#[derive(serde::Deserialize, Debug)]
pub struct SeedsPathRequest {
    tvid: u64,
}

#[derive(serde::Deserialize)]
struct TvDeleteRequest {
    id: u64,
}

#[derive(serde::Deserialize)]
struct TvAddRequest {
    name: String,
    tvdbid: String,
    tvname: String,
    url: String,
}

async fn api(info: web::Query<ApiRequest>, req: HttpRequest) -> HttpResponse {
    info!(
        "query api with params:{:?} req:{:?}",
        info,
        req.query_string()
    );

    let provider = TorznabProvider::new();

    let mut builder = HttpResponse::Ok();
    builder.set(ContentType::xml());

    let result = provider.handle(&info).await;

    builder.body(result)
}

async fn refresh() -> HttpResponse {
    task::spawn(async {
        let tvs: Vec<Tv> = global::RB.fetch_list().await.unwrap();
        let resolver = Resolver::new();
        for tv in tvs {
            resolver.fetch_by_tv(tv.id.unwrap()).await;
        }
    });
    HttpResponse::Ok().json(OperationResponse::success())
}

async fn tv_list(tvs_request: web::Query<TvsRequest>) -> HttpResponse {
    let wrapper = global::RB.new_wrapper();
    let page = PageRequest::new(tvs_request.page.unwrap_or(1_u64), tvs_request.perPage.unwrap_or(10_u64));
    let tv_page: Page<Tv> = global::RB.fetch_page_by_wrapper(wrapper, &page).await.unwrap();

    let response = PageResponse::from(tv_page);
    HttpResponse::Ok().json(response)
}

async fn seed_list(seeds_path_request: web::Path<SeedsPathRequest>, seeds_request: web::Query<SeedsRequest>) -> HttpResponse {
    let wrapper = global::RB.new_wrapper().eq(TvSeed::tv_id(), seeds_path_request.tvid);
    let page = PageRequest::new(seeds_request.page.unwrap_or(1_u64), seeds_request.perPage.unwrap_or(10_u64));
    let seed_page: Page<TvSeed> = global::RB.fetch_page_by_wrapper(wrapper, &page).await.unwrap();

    let response = PageResponse::from(seed_page);
    HttpResponse::Ok().json(response)
}

async fn tv_add(tv_add: web::Json<TvAddRequest>) -> HttpResponse {
    let new_tv = Tv {
        id: None,
        tvdbid: Some(tv_add.tvdbid.to_string()),
        tvname: Some(tv_add.tvname.to_string()),
        url: Some(tv_add.url.to_string()),
        name: Some(tv_add.name.to_string()),
    };

    global::RB.save(&new_tv, &[]).await.unwrap();

    HttpResponse::Ok().json(OperationResponse::success())
}

async fn tv_delete(tv_delete_request: web::Json<TvDeleteRequest>) -> HttpResponse {
    let wrapper = global::RB.new_wrapper().eq(Tv::id(), tv_delete_request.id);
    global::RB.remove_by_wrapper::<Tv>(wrapper).await.unwrap();
    HttpResponse::Ok().json(OperationResponse::success())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

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

    let db_url = matches.value_of("db").unwrap_or("sqlite://:memory:").to_string();
    let static_folder = matches.value_of("static-folder").unwrap_or("./webapp").to_string();

    global::RB.link(&db_url).await.unwrap();

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
        cron.minutes("0");
        cron.start_job();
    });

    HttpServer::new(move || {
        let cors = Cors::permissive();
        wApp::new()
            .wrap(cors)
            .route("/health", web::get().to(root))
            .route("/api", web::get().to(api))
            .route("/admin/fetch", web::get().to(refresh))
            .route("/admin/tvs", web::get().to(tv_list))
            .route("/admin/seeds/tvid/{tvid}", web::get().to(seed_list))
            .route("/admin/tvs", web::post().to(tv_add))
            .route("/admin/tvs/delete", web::post().to(tv_delete))
            .service(actix_files::Files::new("/", &static_folder).index_file("index.html"))
    })
        .bind("0.0.0.0:8000")?
        .run()
        .await
}

fn on_cron(name: &str) {
    task::spawn(async {
        info!("start fetching task...");
        let tvs: Vec<Tv> = global::RB.fetch_list().await.unwrap();
        let resolver = Resolver::new();
        for tv in tvs {
            resolver.fetch_by_tv(tv.id.unwrap()).await;
        }
    });
}
