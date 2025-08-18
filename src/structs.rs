use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Anime {
    pub id: String,
    pub title: String,
    pub url: String,
    pub typ: String,
}

#[derive(Debug, Serialize)]
pub struct Episode {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct VideoSource {
    pub server: String,
    pub url: String,
    pub quality: Option<String>,
}