use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub endpoints: Vec<String>,
}
