use crate::structs::{Anime, Episode, VideoSource};
use reqwest::Client;
use reqwest::dns::{Resolve, Resolving};
use scraper::{Html, Selector};
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::TokioAsyncResolver;
use hyper::client::connect::dns::Name;
use headless_chrome::{Browser, LaunchOptions};
use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;
use tokio::task;
use fantoccini::{ClientBuilder, Locator};
use std::process::Command;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::ffi::{OsStr, OsString};
use std::time::Duration;
use std::io;
use std::sync::Arc;
use std::net::IpAddr;
use serde_json::{json, Map, Value};

static DNS_CACHE: Lazy<Mutex<Option<IpAddr>>> = Lazy::new(|| Mutex::new(None));

pub async fn get_animeav1_ip() -> Result<IpAddr> {
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
    
    let response = resolver.lookup_ip("animeav1.com").await?;
    let ip = response.iter().next().ok_or_else(|| anyhow!("No se encontró IP"))?;
    
    // Actualizar caché
    {
        let mut cache = DNS_CACHE.lock().unwrap();
        *cache = Some(ip);
    }
    
    Ok(ip)
}

const BASE_URL: &str = "https://animeav1.com";

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
    let url = format!("{}/catalogo?search={}", BASE_URL, query);
    
    let html = client.get(&url).send().await?.text().await?;
    let document = Html::parse_document(&html);

    // Selector corregido: cada resultado está en un <article class="group/item">
    let selector = Selector::parse("article.group\\/item").unwrap();
    let mut results = Vec::new();

    for element in document.select(&selector) {
        // Enlace
        let link = element.select(&Selector::parse("a[href]").unwrap()).next().unwrap();
        let url = link.value().attr("href").unwrap().to_string();
        let id = url.split('/').last().unwrap_or("").to_string();

        // Título
        let title = element
            .select(&Selector::parse("h3.line-clamp-2").unwrap())
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_else(|| "Sin título".to_string());

        // Tipo (ej: TV Anime, OVA, etc.)
        let typ = element
            .select(&Selector::parse("div.text-2xs").unwrap())
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
    // Construye la URL destino
    let page_url = format!("{}/media/{}", BASE_URL, anime_id);

    // (opcional) resolver manual a IP si lo usas
    let ip = get_animeav1_ip().await?;

    // extrae dominio de BASE_URL para el host resolver rule
    let domain = BASE_URL
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("animeav1.com")
        .to_string();

    let episodes = task::spawn_blocking(move || -> Result<Vec<Episode>> {
        // args del navegador
        let host_rule = format!("--host-resolver-rules=MAP {} {}", domain, ip);
        let args_os: Vec<OsString> = vec![
            OsString::from("--no-sandbox"),
            OsString::from(host_rule),
        ];
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

        // Navegar
        tab.navigate_to(&page_url).context("Error al navegar a la URL")?;

        // Esperar a que aparezcan los artículos de episodios
        // Nota: hay que escapar la barra de la clase "group/item" como group\/item
        tab.wait_for_element_with_custom_timeout(
            r#"article.group\/item a[href^="/media/"]"#,
            Duration::from_secs(20),
        )
        .context("Timeout esperando episodios")?;

        // HTML renderizado
        let html = tab.get_content().context("Error al obtener contenido")?;
        std::fs::write("episodes_page.html", &html).ok(); // útil para debug

        // Parseo
        let document = Html::parse_document(&html);
        let article_sel = Selector::parse(r#"article.group\/item"#).unwrap();
        let link_sel = Selector::parse(r#"a[href^="/media/"]"#).unwrap();
        let num_span_sel = Selector::parse("span.text-lead.font-bold").unwrap();
        let sr_only_sel = Selector::parse("span.sr-only").unwrap();

        let mut episodes: Vec<Episode> = Vec::new();

        for art in document.select(&article_sel) {
            // Link del episodio
            let a = match art.select(&link_sel).next() {
                Some(n) => n,
                None => continue,
            };
            let href = a.value().attr("href").unwrap_or("").to_string();
            if href.is_empty() { continue; }

            // URL absoluta
            let full_url = if href.starts_with("http://") || href.starts_with("https://") {
                href.clone()
            } else {
                // construye con dominio de BASE_URL
                let dom = BASE_URL
                    .trim_start_matches("https://")
                    .trim_start_matches("http://")
                    .split('/')
                    .next()
                    .unwrap_or("animeav1.com");
                format!("https://{}{}", dom, href)
            };

            // número de episodio desde el href (último segmento)
            let number_from_href = href
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .and_then(|s| s.parse::<u32>().ok());

            // fallback: span con la cifra
            let number = number_from_href.or_else(|| {
                art.select(&num_span_sel)
                    .next()
                    .and_then(|n| n.text().collect::<String>().trim().parse::<u32>().ok())
            }).unwrap_or(0);

            // id = último segmento (suele ser el número)
            let id = href
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_string();

            // título: intenta desde el sr-only ("Ver {Titulo} {n}")
            let title = art
                .select(&sr_only_sel)
                .next()
                .map(|n| n.text().collect::<String>())
                .map(|raw| {
                    let t = raw.trim();
                    let t = t.strip_prefix("Ver ").unwrap_or(t);
                    // separa el último token numérico si coincide con nuestro number
                    if let Some(pos) = t.rfind(' ') {
                        let (maybe_title, maybe_num) = t.split_at(pos);
                        if maybe_num.trim().parse::<u32>().ok() == Some(number) {
                            maybe_title.trim().to_string()
                        } else {
                            t.to_string()
                        }
                    } else {
                        t.to_string()
                    }
                })
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| if number > 0 { format!("Episodio {}", number) } else { "Episodio".to_string() });

            episodes.push(Episode {
                id,
                number,
                title,
                url: full_url,
            });
        }

        // ordena por número ascendente (seguro y claro)
        episodes.sort_by_key(|e| e.number);
        Ok(episodes)
    })
    .await
    .map_err(|e| anyhow!("Error en spawn_blocking: {}", e))??;

    Ok(episodes)
}

/* --------------------------------------------------- */
/*        Verificaciones para el get sources          */
/* ------------------------------------------------- */

pub async fn get_video_sources(episode_id: &str) -> Result<Vec<VideoSource>, Box<dyn Error>> {
    // --- 1. Verificar dependencias ---
    fn check_bin(bin: &str) -> Result<String, Box<dyn Error>> {
        let output = Command::new("which").arg(bin).output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(format!("No se encontró `{}` en el sistema. Instálalo antes de usar este proyecto.", bin).into())
        }
    }

    let driver_path = check_bin("chromedriver")?;
    let chrome_path = check_bin("chromium-browser")
        .or_else(|_| check_bin("chromium"))
        .or_else(|_| check_bin("google-chrome"))?;

    // --- 2. Levantar chromedriver ---
    let mut chromedriver = Command::new(&driver_path)
        .arg("--port=9515")
        .spawn()?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // --- 3. Configurar capabilities ---
    let caps_value = json!({
        "goog:chromeOptions": {
            "binary": chrome_path,
            "args": ["--headless", "--no-sandbox", "--disable-gpu"]
        }
    });
    let caps: Map<String, Value> = caps_value.as_object().unwrap().clone();

    // --- 4. Conectar con fantoccini ---
    let client = ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:9515")
        .await?;

    let episode_parse = episode_id.replace("*", "/");
    let url = format!("{}/media/{}", BASE_URL, episode_parse);
    client.goto(&url).await?;

    // --- 5. Recolectar servidores ---
    client.wait().for_element(Locator::Css("div.flex.flex-1.flex-wrap button")).await?;
    let buttons = client.find_all(Locator::Css("div.flex.flex-1.flex-wrap button")).await?;

    let mut sources = Vec::new();

    for btn in buttons {
        let server_name = btn.text().await?.trim().to_string();

        // Convertir el ElementId a String
        let btn_id = btn.element_id().to_string();
        let btn_arg = json!({ "element-6066-11e4-a52e-4f735466cecf": btn_id });

        // Hacer scroll hasta el botón
        let _ = client
            .execute("arguments[0].scrollIntoView(true);", vec![btn_arg.clone()])
            .await;

        // Intentar click normal
        let click_res = btn.click().await;

        // Si falla, usar JS click
        if click_res.is_err() {
            client
                .execute("arguments[0].click();", vec![btn_arg.clone()])
                .await?;
        }

        tokio::time::sleep(Duration::from_secs(2)).await;

        // Extraer iframe
        if let Ok(iframe) = client.find(Locator::Css("iframe")).await {
            if let Some(src) = iframe.attr("src").await? {
                sources.push(VideoSource {
                    server: server_name,
                    url: src,
                    quality: None,
                });
            }
        }
    }

    client.close().await?;
    chromedriver.kill()?; // cerrar proceso de chromedriver

    Ok(sources)
}
