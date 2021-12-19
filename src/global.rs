use rbatis::rbatis::Rbatis;

lazy_static! {
    pub static ref RB: Rbatis = Rbatis::new();
}
