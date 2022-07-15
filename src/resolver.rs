use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use headless_chrome::{Browser, Element};
use log::info;
use magnet_url::Magnet;
use rbatis::crud::CRUD;
use regex::Regex;
use scraper::{Html, Selector};

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::global;
use crate::model::{Tv, TvSeed};

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
            let resolver = DefaultResolver::new();
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

    let re = Regex::new(r"EP(\d+)").unwrap();
    let option = re.captures(name);
    if option.is_some() {
        let captures = option.unwrap();
        if captures.len() > 0 {
            let result: i64 = captures[1].parse().unwrap();
            return Ok(result);
        }
    }

    Err(ResolveError::EpParseFailure(name.to_string()).into())
}

#[async_trait]
trait CommonResolver {
    fn new() -> Self;
    async fn fetch(&self, tv: &Tv) -> Result<Vec<Data>>;
    async fn normalize(&self, tv: &Tv, datas: Vec<Data>) -> Result<Vec<Data>>;
}

#[derive(Error, Debug)]
pub enum ResolveError {
    #[error("Can't parse ep for name: {0}")]
    EpParseFailure(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolverDefine {
    id: String,
    name: String,
    domains: Vec<String>,
    timeout: u64,
    search: ResolverSearchDefine,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolverRowSelectorDefine {
    attr: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolverRowsDefine {
    selector: String,
    title: ResolverRowSelectorDefine,
    url: ResolverRowSelectorDefine,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolverSearchDefine {
    wait: String,
    rows: ResolverRowsDefine,
}

#[derive(RustEmbed)]
#[folder = "define/"]
struct Define;

pub struct DefaultResolver {
    pub defines: Vec<ResolverDefine>,
}

#[async_trait]
impl CommonResolver for DefaultResolver {
    fn new() -> Self {
        let mut defines = vec![];
        for file in Define::iter() {
            let yaml = Define::get(file.as_ref()).unwrap();
            let yaml_content = std::str::from_utf8(yaml.data.as_ref());
            let define: ResolverDefine = serde_yaml::from_str(yaml_content.unwrap()).unwrap();
            info!("load config for {}", define.id);
            defines.push(define);
        }
        DefaultResolver { defines }
    }

    async fn fetch(&self, tv: &Tv) -> Result<Vec<Data>> {
        let url = tv.url.as_ref().unwrap();
        let selected_define = self.defines.iter().find(|d| {
            for domain in &d.domains {
                if url.starts_with(domain) {
                    return true;
                }
            }
            return false;
        });
        let selected_define = selected_define.unwrap();

        info!("starting fetch...");
        let mut data = vec![];

        let browser = Browser::default().unwrap();

        let tab = browser.wait_for_initial_tab().unwrap();
        info!("browser tab is ready");

        tab.navigate_to(&url).unwrap();

        tab.wait_for_element_with_custom_timeout(
            &selected_define.search.wait,
            Duration::from_secs(selected_define.timeout),
        )
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

        let selector = Selector::parse(&selected_define.search.rows.selector).unwrap();
        let list = document.select(&selector);

        for item in list {
            let title = item.value().attr(&selected_define.search.rows.title.attr);
            let url = item.value().attr(&selected_define.search.rows.url.attr);

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

                let ep = extra_ep(&clean_up_name).expect("can't extra ep");

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
    use std::fs;

    use super::*;

    #[test]
    fn test_parse_yml() {
        let paths = fs::read_dir("./define").unwrap();

        for path in paths {
            let result = std::fs::read_to_string(path.unwrap().path()).unwrap();
            let define: ResolverDefine = serde_yaml::from_str(&result).unwrap();
            println!("Define:{:?}", define);
        }
    }

    #[test]
    fn test_load() {
        let _resolver = DefaultResolver::new();
    }
}
