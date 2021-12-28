use std::collections::HashSet;
use std::sync::Mutex;

use rbatis::rbatis::Rbatis;

lazy_static! {
    pub static ref RB: Rbatis = Rbatis::new();
}

lazy_static! {
    pub static ref WANT: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}
