use std::ptr::write;
use std::str::from_utf8;

use actix_web::web;
use actix_web::web::Query;
use chrono::{Duration, Utc};
use log::info;
use magnet_url::Magnet;
use rbatis::crud::CRUD;
use rbatis::utils::error_util::ToResult;
use rbatis::{Page, PageRequest};
use tinytemplate::TinyTemplate;
use xml::writer::{EmitterConfig, EventWriter, Result, XmlEvent};

use crate::global;
use crate::model::Tv;
use crate::resolver::Domp4Resolver;
use crate::{ApiRequest, TvSeed};

pub struct TorznabProvider {}

#[derive(serde::Serialize)]
struct Context {
    name: String,
}

impl TorznabProvider {
    pub fn new() -> Self {
        TorznabProvider {}
    }

    pub fn caps(&self) -> String {
        r#"<caps>
   <server version="1.1" title="..." strapline="..."
         email="..." url="http://indexer.local/"
         image="http://indexer.local/content/banner.jpg" />
   <limits max="100" default="50" />
   <retention days="400" />
   <registration available="yes" open="yes" />

   <searching>
      <search available="yes" supportedParams="q" />
      <tv-search available="yes" supportedParams="q,rid,tvdbid,season,ep" />
      <movie-search available="no" supportedParams="q,imdbid,genre" />
      <audio-search available="no" supportedParams="q" />
      <book-search available="no" supportedParams="q" />
   </searching>

   <categories>
      <category id="5000" name="TV">
      </category>
   </categories>

   <groups>
      <group id="1" name="alt.binaries...." description="..." lastupdate="..." />
   </groups>

   <genres>
      <genre id="1" categoryid="5000" name="Kids" />
   </genres>

   <tags>
      <tag name="anonymous" description="Uploader is anonymous" />
      <tag name="trusted" description="Uploader has high reputation" />
      <tag name="internal" description="Uploader is an internal release group" />
   </tags>
</caps>
        "#
        .to_string()
    }

    pub async fn search(&self, info: &Query<ApiRequest>) -> String {
        let mut wrapper;
        if info.tvdbid.is_some() {
            let tvdbid = info.tvdbid.as_ref().unwrap().clone();
            wrapper = global::RB.new_wrapper().eq("tvdbid", tvdbid);
        } else {
            wrapper = global::RB.new_wrapper();
        }

        let tv: Option<Tv> = global::RB.fetch_by_wrapper(wrapper).await.unwrap();

        let tv = tv.unwrap();
        let tv_id = tv.id.unwrap();

        let req = page(info.offset.unwrap_or(0_u64), info.limit.unwrap_or(50_u64));

        let mut wrapper = global::RB.new_wrapper().eq(TvSeed::tv_id(), tv_id);

        if info.ep.is_some() {
            wrapper = wrapper.eq(TvSeed::ep(), info.ep.unwrap_or(-1));
        }

        let seeds: Page<TvSeed> = global::RB
            .fetch_page_by_wrapper(wrapper, &req)
            .await
            .unwrap();

        let mut target: Vec<u8> = Vec::new();
        let mut writer = EmitterConfig::new()
            .perform_indent(true)
            .create_writer(&mut target);

        writer.write(
            XmlEvent::start_element("rss")
                .attr("version", "2.0")
                .ns("torznab", "http://torznab.com/schemas/2015/feed"),
        );
        writer.write(XmlEvent::start_element("channel"));
        Self::createElementWithChars(&mut writer, "title", "wkavu");
        Self::createElementWithChars(&mut writer, "description", "wkavu");
        Self::createElementWithChars(&mut writer, "language", "zh-CN");
        Self::createElementWithChars(&mut writer, "category", "search");

        for seed in seeds.records {
            let name = seed.name.unwrap();
            let url = seed.url.unwrap();

            writer.write(XmlEvent::start_element("item"));
            Self::createElementWithChars(&mut writer, "title", &name);
            Self::createElementWithChars(
                &mut writer,
                "guid",
                xml::escape::escape_str_attribute(&url).as_ref(),
            );
            Self::createElementWithChars(&mut writer, "pubDate", &pub_date());
            Self::createElementWithChars(&mut writer, "size", "0");
            Self::createElementWithChars(
                &mut writer,
                "link",
                xml::escape::escape_str_attribute(&url).as_ref(),
            );
            Self::createElementWithChars(&mut writer, "category", "5000");

            writer.write(
                XmlEvent::start_element("enclosure")
                    .attr("url", xml::escape::escape_str_attribute(&url).as_ref())
                    .attr("type", "application/x-bittorrent")
                    .attr("length", "0"),
            );
            writer.write(XmlEvent::end_element());

            writer.write(
                XmlEvent::start_element("torznab:attr")
                    .attr("name", "category")
                    .attr("value", "5000"),
            );
            writer.write(XmlEvent::end_element());

            writer.write(XmlEvent::end_element());
        }

        writer.write(XmlEvent::end_element());
        writer.write(XmlEvent::end_element());

        std::str::from_utf8(&target).unwrap().to_string()
    }

    fn createElementWithChars(writer: &mut EventWriter<&mut Vec<u8>>, name: &str, text: &str) {
        writer.write(XmlEvent::start_element(name));
        writer.write(XmlEvent::characters(text));
        writer.write(XmlEvent::end_element());
    }

    pub async fn handle(&self, info: &web::Query<ApiRequest>) -> String {
        if info.t == "caps" {
            self.caps()
        } else {
            self.search(info).await
        }
    }
}

fn page(offset: u64, page_size: u64) -> PageRequest {
    if offset >= 1000 {
        return PageRequest::new(9999, 9999);
    }
    let page_no = offset / page_size;
    PageRequest::new(page_no + 1, page_size)
}

fn pub_date() -> String {
    let time = Utc::now() - Duration::days(1);
    time.to_rfc2822()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        let page = page(0, 50);

        assert_eq!(page.page_no, 1);
    }

    #[test]
    fn test_second() {
        let page = page(50, 50);

        assert_eq!(page.page_no, 2);
    }

    #[test]
    fn test_big() {
        let page = page(1000, 50);

        assert_eq!(page.page_no, 9999);
    }

    #[test]
    fn test_pub_date() {
        let date = pub_date();
    }

    #[test]
    fn test_caps(){
        let xml = caps();
    }
}
