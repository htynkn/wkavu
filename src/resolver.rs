use std::time::Duration;

use anyhow::Result;
use headless_chrome::{protocol::page::ScreenshotFormat, Browser, Element};
use log::info;
use magnet_url::Magnet;
use rbatis::crud::CRUD;
use scraper::{Html, Selector};
use select::document::Document;
use select::predicate::Name;

use crate::global;
use crate::model::{Tv, TvSeed};

pub struct Domp4Resolver {}

#[derive(Debug)]
pub struct Data {
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) ep: i64,
}

impl Data {
    fn new(name: &str, url: &str) -> Self {
        Data {
            name: name.to_string(),
            url: url.to_string(),
            ep: -1,
        }
    }
}

pub struct Resolver {}

impl Resolver {
    pub fn new() -> Self {
        Resolver {}
    }

    pub async fn fetch_by_tv(&self, tv_id: i64) {
        let wrapper = global::RB.new_wrapper().eq("id", tv_id);
        let tv: Option<Tv> = global::RB.fetch_by_wrapper(wrapper).await.unwrap();

        if tv.is_some() {
            let tv = tv.unwrap();
            let resolver = Domp4Resolver::new();
            let data = resolver.fetch(&tv).await.unwrap();
            let data = resolver.normalize(&tv, data).await.unwrap();

            info!("find {:?} for tv:{:?}", data, tv);

            if data.len() > 0 {
                let wrapper = global::RB.new_wrapper().eq("tv_id", tv.id.unwrap());
                let delete_count = global::RB
                    .remove_by_wrapper::<TvSeed>(wrapper)
                    .await
                    .unwrap();
                info!("delete seed for tv count:{}", delete_count);

                let tv_id = tv.id.unwrap();
                for d in data {
                    let seed = TvSeed {
                        id: None,
                        tv_id: Some(tv_id),
                        ep: Some(d.ep),
                        url: Some(d.url),
                        name: Some(d.name),
                    };
                    global::RB.save(&seed, &[]).await;
                }
            }
        } else {
            log::error!("found find movie with id:{}", tv_id);
        }
    }
}

use async_trait::async_trait;
use regex::Regex;

fn extra_ep(name: &str) -> Result<i64> {
    let re = Regex::new(r"第(\d+)集").unwrap();
    let option = re.captures(name);
    if option.is_some() {
        let captures = option.unwrap();
        if captures.len() > 0 {
            let result: i64 = captures[1].parse().unwrap();
            return Ok(result);
        }
    }
    Ok(-1)
}

#[async_trait]
trait CommonResolver {
    fn new() -> Self;
    async fn fetch(&self, tv: &Tv) -> Result<Vec<Data>>;
    async fn normalize(&self, tv: &Tv, datas: Vec<Data>) -> Result<Vec<Data>>;
}

#[async_trait]
impl CommonResolver for Domp4Resolver {
    fn new() -> Self {
        Domp4Resolver {}
    }

    async fn fetch(&self, tv: &Tv) -> Result<Vec<Data>> {
        info!("starting fetch...");
        let url = tv.url.as_ref().unwrap();
        let mut data = vec![];

        let browser = Browser::default().unwrap();

        let tab = browser.wait_for_initial_tab().unwrap();
        info!("browser tab is ready");

        tab.navigate_to(url).unwrap();

        tab.wait_for_element_with_custom_timeout("a.copybtn", Duration::from_secs(30))
            .unwrap();
        info!("waiting for special button");

        let root_div: Element = tab.wait_for_element("body").unwrap();
        let html = root_div
            .call_js_fn("function() { return this.innerHTML;}", true)
            .unwrap()
            .value
            .unwrap();

        let document = Html::parse_document(html.as_str().unwrap());
        info!("get doc object");

        let selector = Selector::parse("ul.down-list div.url-left a").unwrap();
        let list = document.select(&selector);

        for item in list {
            let title = item.value().attr("title");
            let url = item.value().attr("href");

            data.push(Data::new(title.unwrap(), url.unwrap()));
        }

        Ok(data)
    }

    async fn normalize(&self, tv: &Tv, datas: Vec<Data>) -> Result<Vec<Data>> {
        Ok(datas
            .into_iter()
            .map(|d| {
                let clean_up_name = str::replace(
                    &str::replace(&d.name, "HD1080p", "[HDTV-1080p]"),
                    ".mp4",
                    "",
                );

                let mut magneturl = Magnet::new(&d.url).unwrap();
                magneturl.tr.clear();
                magneturl.dn = None;

                let ep = extra_ep(&clean_up_name).unwrap_or(-1);

                let clean_up_name = if ep > 0 {
                    format!(
                        "{} S01E{} - {} - [chinese] - {} - Domp4",
                        tv.tvname.as_ref().unwrap(),
                        ep,
                        ep,
                        &clean_up_name
                    )
                } else {
                    clean_up_name
                };

                Data {
                    ep,
                    name: clean_up_name,
                    url: magneturl.to_string(),
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extra_ep() {
        assert_eq!(extra_ep("第26集").unwrap(), 26);
        assert_eq!(extra_ep("第01集").unwrap(), 1);
    }
}
