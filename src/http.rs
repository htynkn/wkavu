use actix_web::http::header;
use actix_web::http::header::ContentType;
use actix_web::{web, HttpRequest, HttpResponse};
use async_std::task;

use log::info;

use rbatis::crud::CRUD;
use rbatis::{Page, PageRequest};

use crate::global;
use crate::model::{OperationResponse, PageResponse, Tv, TvSeed};
use crate::resolver::Resolver;
use crate::torznab::TorznabProvider;

pub async fn root() -> HttpResponse {
    HttpResponse::Found()
        .header(header::LOCATION, "index.html")
        .finish()
}

pub async fn health() -> HttpResponse {
    HttpResponse::Ok().body("server is up!")
}

#[derive(serde::Deserialize, Debug)]
pub struct ApiRequest {
    pub(crate) t: String,
    q: Option<String>,
    pub(crate) tvdbid: Option<String>,
    season: Option<i32>,
    pub(crate) ep: Option<i32>,
    pub(crate) offset: Option<u64>,
    pub(crate) limit: Option<u64>,
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
pub struct TvDeleteRequest {
    id: u64,
}

#[derive(serde::Deserialize)]
pub struct TvAddRequest {
    name: String,
    tvdbid: String,
    tvname: String,
    url: String,
}

pub async fn api(info: web::Query<ApiRequest>, req: HttpRequest) -> HttpResponse {
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

pub async fn refresh() -> HttpResponse {
    task::spawn(async {
        let tvs: Vec<Tv> = global::RB.fetch_list().await.unwrap();
        let resolver = Resolver::new();
        for tv in tvs {
            resolver.fetch_by_tv(tv.id.unwrap()).await;
        }
    });
    HttpResponse::Ok().json(OperationResponse::success())
}

pub async fn tv_list(tvs_request: web::Query<TvsRequest>) -> HttpResponse {
    let wrapper = global::RB.new_wrapper();
    let page = PageRequest::new(
        tvs_request.page.unwrap_or(1_u64),
        tvs_request.perPage.unwrap_or(10_u64),
    );
    let tv_page: Page<Tv> = global::RB
        .fetch_page_by_wrapper(wrapper, &page)
        .await
        .unwrap();

    let response = PageResponse::from(tv_page);
    HttpResponse::Ok().json(response)
}

pub async fn seed_list(
    seeds_path_request: web::Path<SeedsPathRequest>,
    seeds_request: web::Query<SeedsRequest>,
) -> HttpResponse {
    let wrapper = global::RB
        .new_wrapper()
        .eq(TvSeed::tv_id(), seeds_path_request.tvid);
    let page = PageRequest::new(
        seeds_request.page.unwrap_or(1_u64),
        seeds_request.perPage.unwrap_or(10_u64),
    );
    let seed_page: Page<TvSeed> = global::RB
        .fetch_page_by_wrapper(wrapper, &page)
        .await
        .unwrap();

    let response = PageResponse::from(seed_page);
    HttpResponse::Ok().json(response)
}

pub async fn tv_add(tv_add: web::Json<TvAddRequest>) -> HttpResponse {
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

pub async fn tv_delete(tv_delete_request: web::Json<TvDeleteRequest>) -> HttpResponse {
    let wrapper = global::RB.new_wrapper().eq(Tv::id(), tv_delete_request.id);
    global::RB.remove_by_wrapper::<Tv>(wrapper).await.unwrap();
    HttpResponse::Ok().json(OperationResponse::success())
}

#[cfg(test)]
mod tests {
    use actix_web::{http, test};

    use super::*;

    #[actix_rt::test]
    async fn test_health_ok() {
        let _req =
            test::TestRequest::with_header("content-type", "application/json; charset=UTF-8")
                .to_http_request();
        let resp = health().await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }
}
