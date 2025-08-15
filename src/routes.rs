use axum::{
    routing::get,
    Router,
    extract::{Query, Path},
    Json,
    http::StatusCode,
};
use serde::Deserialize;
use crate::scraping;
use crate::models::{Anime, Episode, VideoSource};

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    q: String,
}

pub async fn search_anime(
    Query(params): Query<SearchParams>
) -> Result<Json<Vec<Anime>>, StatusCode> {
    match scraping::search_anime(&params.q).await {
        Ok(animes) => Ok(Json(animes)),
        Err(e) => {
            eprintln!("Error en búsqueda: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        },
    }
}

pub async fn get_anime_episodes(
    Path(anime_id): Path<String>
) -> Result<Json<Vec<Episode>>, StatusCode> {
    match scraping::get_episodes(&anime_id).await {
        Ok(episodes) => Ok(Json(episodes)),
        Err(e) => {
            eprintln!("Error obteniendo episodios: {:?}", e);
            Err(StatusCode::NOT_FOUND)
        },
    }
}

pub async fn get_episode_sources(
    Path(episode_id): Path<String>
) -> Result<Json<Vec<VideoSource>>, StatusCode> {
    match scraping::get_video_sources(&episode_id).await {
        Ok(sources) => Ok(Json(sources)),
        Err(e) => {
            eprintln!("Error obteniendo fuentes: {:?}", e);
            Err(StatusCode::NOT_FOUND)
        },
    }
}

pub fn create_routes() -> Router {
    Router::new()
        .route("/search", get(search_anime))
        .route("/anime/:anime_id/episodes", get(get_anime_episodes))
        .route("/episode/:episode_id/sources", get(get_episode_sources))
}