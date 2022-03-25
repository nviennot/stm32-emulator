use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Region {
   pub name: String,
   pub start: u32,
   pub size: u32,
   pub load: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Cpu {
    pub svd: String,
    pub vector_table: u32,
}

#[derive(Debug, Deserialize)]
pub struct Config {
   pub cpu: Cpu,
   pub regions: Vec<Region>,
}
