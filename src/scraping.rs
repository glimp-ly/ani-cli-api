use crate::models::{Anime, Episode, VideoSource};
use reqwest::Client;
use scraper::{Html, Selector};
use std::error::Error;
use std::net::SocketAddr;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::TokioAsyncResolver;
use reqwest::dns::{Resolve, Resolving};
use hyper::client::connect::dns::Name;
use std::io;
use std::sync::Arc;
use headless_chrome::{Browser, LaunchOptions};
use std::time::Duration;
use anyhow::{anyhow, Context, Result};
use std::net::IpAddr;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::ffi::{OsStr, OsString};
use tokio::task;

static DNS_CACHE: Lazy<Mutex<Option<IpAddr>>> = Lazy::new(|| Mutex::new(None));

pub async fn get_animeflv_ip() -> Result<IpAddr> {
    // Verificar caché
    {
        let cache = DNS_CACHE.lock().unwrap();
        if let Some(ip) = *cache {
            return Ok(ip);
        }
    }
    
    // Resolver si no está en caché
    let resolver = TokioAsyncResolver::tokio(
        ResolverConfig::google(),
        ResolverOpts::default()
    )?;
    
    let response = resolver.lookup_ip("animeflv.net").await?;
    let ip = response.iter().next().ok_or_else(|| anyhow!("No se encontró IP"))?;
    
    // Actualizar caché
    {
        let mut cache = DNS_CACHE.lock().unwrap();
        *cache = Some(ip);
    }
    
    Ok(ip)
}

const BASE_URL: &str = "https://animeflv.net";

// Estructura personalizada para resolución DNS
#[derive(Clone)]
struct CustomResolver;

impl Resolve for CustomResolver {
    fn resolve(&self, name: Name) -> Resolving {
        // Crear un futuro asíncrono
        let fut = async move {
            // Configurar DNS de Google
            let resolver = TokioAsyncResolver::tokio(
                ResolverConfig::google(),
                ResolverOpts::default()
            ).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            
            let response = resolver.lookup_ip(name.as_str()).await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            
            let addrs: Vec<SocketAddr> = response.iter()
                .map(|ip| SocketAddr::new(ip, 443))
                .collect();
            
            Ok(Box::new(addrs.into_iter()) as Box<dyn Iterator<Item = SocketAddr> + Send>)
        };
        
        Box::pin(fut)
    }
}

pub async fn custom_client() -> Result<Client, Box<dyn Error>> {
    // Crear cliente con resolución DNS personalizada
    let client = Client::builder()
        .dns_resolver(Arc::new(CustomResolver))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()?;
    
    Ok(client)
}

pub async fn search_anime(query: &str) -> Result<Vec<Anime>, Box<dyn Error>> {
    let client = custom_client().await?;
    let url = format!("{}/browse?q={}", BASE_URL, query);
    
    let html = client.get(&url).send().await?.text().await?;
    
    let document = Html::parse_document(&html);

    // Selector corregido:
    let selector = Selector::parse("ul.ListAnimes > li > article.Anime").unwrap();
    let mut results = Vec::new();

    for element in document.select(&selector) {
        // Obtener el enlace
        let link = element.select(&Selector::parse("a").unwrap()).next().unwrap();
        let url = link.value().attr("href").unwrap().to_string();
        let id = url.split('/').last().unwrap().to_string();
        
        // Obtener título
        let title = link.select(&Selector::parse("h3.Title").unwrap())
            .next()
            .unwrap()
            .text()
            .collect::<String>();
        
        // Obtener tipo
        let typ = element.select(&Selector::parse("span.Type").unwrap())
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_else(|| "Desconocido".to_string());

        results.push(Anime {
            id,
            title,
            url,
            typ,
        });
    }

    Ok(results)
}

pub async fn get_episodes(anime_id: &str) -> Result<Vec<Episode>> {
    let ip = get_animeflv_ip().await?;
    let domain = "www3.animeflv.net";
    let url = format!("https://{}/anime/{}", domain, anime_id);

    let episodes = task::spawn_blocking(move || -> Result<Vec<Episode>> {
        // Construimos OsString que vivirán en esta función
        let host_rule = format!("--host-resolver-rules=MAP {} {}", domain, ip);
        let mut args_os: Vec<OsString> = Vec::new();
        args_os.push(OsString::from("--no-sandbox"));
        args_os.push(OsString::from(host_rule));

        // Ahora creamos Vec<&OsStr> que referencia los OsString anteriores
        let args_refs: Vec<&OsStr> = args_os.iter().map(|s| s.as_os_str()).collect();

        let launch_opts = LaunchOptions {
            headless: true,
            sandbox: false,
            args: args_refs,
            ignore_certificate_errors: true,
            ..Default::default()
        };

        let browser = Browser::new(launch_opts).context("Error al crear el navegador")?;
        let tab = browser.new_tab().context("Error al abrir pestaña")?;

        // Navegar a la URL
        tab.navigate_to(&url).context("Error al navegar a la URL")?;

        // Esperar a que los episodios estén cargados
        tab.wait_for_element_with_custom_timeout("ul.ListCaps > li", Duration::from_secs(15))
            .context("Timeout esperando episodios")?;

        // Obtener contenido renderizado
        let html = tab.get_content().context("Error al obtener contenido")?;

        // Parseo del HTML
        let document = Html::parse_document(&html);
        let selector = Selector::parse("ul.ListCaps > li > a").unwrap();
        let p_selector = Selector::parse("p").unwrap();
        let h3_selector = Selector::parse("h3").unwrap();

        let mut episodes = Vec::new();
        for element in document.select(&selector) {
            let episode_number_text = element
                .select(&p_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let episode_number = episode_number_text
                .replace("Episodio ", "")
                .trim()
                .parse::<u32>()
                .unwrap_or(0);

            let episode_title = element
                .select(&h3_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let href = element
                .value()
                .attr("href")
                .ok_or_else(|| anyhow!("Atributo href no encontrado"))?
                .to_string();

            let full_url = if href.starts_with("http://") || href.starts_with("https://") {
                href.clone()
            } else {
                format!("https://{}{}", domain, href)
            };

            let id = full_url
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .ok_or_else(|| anyhow!("No se pudo extraer ID de la URL"))?
                .to_string();

            episodes.push(Episode {
                id,
                number: episode_number,
                title: episode_title,
                url: full_url,
            });
        }

        episodes.reverse();
        Ok(episodes)
    })
    .await
    .map_err(|e| anyhow!("Error en spawn_blocking: {}", e))??;

    Ok(episodes)
}

pub async fn get_video_sources(episode_id: &str) -> Result<Vec<VideoSource>, Box<dyn Error>> {
    let client = custom_client().await?;
    let url = format!("{}/ver/{}", BASE_URL, episode_id);
    let html = client.get(&url).send().await?.text().await?;

    let document = Html::parse_document(&html);
    
    // Selecciona todas las opciones (li) que contienen info del servidor
    let option_selector = Selector::parse("ul.CapiTnv li").unwrap();
    let iframe_selector = Selector::parse("div#video_box iframe, div.tab-pane iframe").unwrap();
    
    let mut sources = Vec::new();

    //escritra para debuggin
    std::fs::write("episodes_rendered.html", &html).context("Error al guardar HTML")?;

    // Recorremos cada <li> para obtener nombre y asociar iframe
    for (index, option) in document.select(&option_selector).enumerate() {
        let server = option.value()
            .attr("data-original-title")
            .unwrap_or("Desconocido")
            .to_string();

        // Buscar iframe correspondiente por posición
        if let Some(iframe) = document.select(&iframe_selector).nth(index) {
            if let Some(src) = iframe.value().attr("src") {
                sources.push(VideoSource {
                    server,
                    url: src.to_string(),
                    quality: None, // aquí podrías analizar calidad si la tienes
                });
            }
        }
    }

    Ok(sources)
}

