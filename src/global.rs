use rbatis::rbatis::Rbatis;

lazy_static! {
    pub static ref RB: Rbatis = Rbatis::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rb() {
        global::RB.link("sqlite://:memory:").await.unwrap();
    }
}
