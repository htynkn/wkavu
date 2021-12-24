use rbatis::Page;

#[crud_table]
#[derive(Clone, Debug)]
pub struct Tv {
    pub id: Option<i64>,
    pub tvdbid: Option<String>,
    pub tvname: Option<String>,
    pub url: Option<String>,
    pub name: Option<String>,
}

#[crud_table]
#[derive(Clone, Debug)]
pub struct TvSeed {
    pub id: Option<i64>,
    pub tv_id: Option<i64>,
    pub ep: Option<i64>,
    pub url: Option<String>,
    pub name: Option<String>,
}

impl_field_name_method!(TvSeed { id, tv_id, ep });
impl_field_name_method!(Tv { id });

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct PageResponse<T> {
    pub status: u8,
    pub msg: String,
    pub data: PageDataResponse<T>,
}

impl From<Page<Tv>> for PageResponse<TvResponse> {
    fn from(o: Page<Tv>) -> Self {
        let tvs: Vec<TvResponse> = o
            .records
            .into_iter()
            .map(|tv| TvResponse::from(tv))
            .collect();
        let x = PageResponse {
            status: 0,
            msg: "".to_string(),
            data: PageDataResponse {
                items: tvs,
                total: o.total,
            },
        };
        x
    }
}

impl From<Page<TvSeed>> for PageResponse<TvSeedResponse> {
    fn from(o: Page<TvSeed>) -> Self {
        let seeds: Vec<TvSeedResponse> = o
            .records
            .into_iter()
            .map(|seed| TvSeedResponse::from(seed))
            .collect();
        PageResponse {
            status: 0,
            msg: "".to_string(),
            data: PageDataResponse {
                items: seeds,
                total: o.total,
            },
        }
    }
}

impl From<Tv> for TvResponse {
    fn from(o: Tv) -> Self {
        TvResponse {
            id: o.id.unwrap(),
            tvdbid: o.tvdbid.unwrap(),
            tvname: o.tvname.unwrap(),
            url: o.url.unwrap(),
            name: o.name.unwrap(),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct TvResponse {
    pub id: i64,
    pub tvdbid: String,
    pub tvname: String,
    pub url: String,
    pub name: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct TvSeedResponse {
    pub id: i64,
    pub tv_id: i64,
    pub ep: i64,
    pub url: String,
    pub name: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct PageDataResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
pub struct OperationResponse {
    pub status: u8,
    pub msg: String,
}

impl OperationResponse {
    pub fn success() -> Self {
        let r: OperationResponse = Default::default();
        r
    }
}

impl From<TvSeed> for TvSeedResponse {
    fn from(s: TvSeed) -> Self {
        TvSeedResponse {
            id: s.id.unwrap(),
            tv_id: s.tv_id.unwrap(),
            ep: s.ep.unwrap(),
            url: s.url.unwrap(),
            name: s.name.unwrap(),
        }
    }
}
