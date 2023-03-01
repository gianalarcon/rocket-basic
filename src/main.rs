use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lru_time_cache::LruCache;
use reqwest::Client;
use rocket::http::uri::Origin;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::serde::json::serde_json::json;
use rocket::serde::json::Value;
use rocket::State;

#[macro_use]
extern crate rocket;
type StringValueCache = Arc<Mutex<LruCache<String, Value>>>;
struct TauriGHReleaseCache {
    mutex: StringValueCache,
}
//TTL: Time to live
static RELEASE_TTL: u64 = 5 * 60;
const TAURI_RELEASES_PREFIX: Origin<'static> = uri!("/tauri-releases");
const GOOGLE_KEEP_DESKTOP_REPO: &str = "elibroftw/google-keep-desktop-app";
#[get("/")]
fn index() -> Redirect {
    let msg: Option<&str> = None;
    let platform = "windows-x86_64";
    let current_version = "v1.0.14";
    Redirect::to(uri!(
        TAURI_RELEASES_PREFIX,
        google_keep_desktop_api(platform, current_version, msg)
    ))
}

fn remove_suffix<'a>(s: &'a str, suffix: &str) -> &'a str {
    match s.strip_suffix(suffix) {
        Some(s) => s,
        None => s,
    }
}
async fn text_request(client: &State<Client>, url: &str) -> Result<String, reqwest::Error> {
    client.get(url).send().await?.text().await
}
#[get("/google-keep-desktop/<platform>/<current_version>?<msg>")]
async fn google_keep_desktop_api(
    platform: &str,
    current_version: &str,
    msg: Option<&str>,
    client: &State<Client>,
    cache: &State<TauriGHReleaseCache>,
) -> Result<Value, Status> {
    if let Some(message) = msg {
        println!("{message}");
        return Err(Status::NoContent);
    }
    let latest_release = get_lastest_release_ttl(cache, client, GOOGLE_KEEP_DESKTOP_REPO).await;

    //inputs checks
    let response = move || -> Option<_> {
        let current_version = current_version.trim_start_matches('v');
        let latest_version = latest_release["version"].as_str()?.trim_start_matches('v');
        if latest_version == current_version {
            return None;
        }
        // can do platform and addtitional version checks
        return Some(latest_release);
    }();
    response.ok_or(Status::NoContent)
}

async fn get_lastest_release_ttl(
    cache: &State<TauriGHReleaseCache>,
    client: &State<Client>,
    repo: &str,
) -> Value {
    if let Some(release) = cache.mutex.lock().unwrap().get(repo) {
        return release.clone();
    }
    let release = get_lastest_release(client, repo)
        .await
        .or_else(|error| {
            println!("{error:?}");
            Ok::<Value, reqwest::Error>(json!({}))
        })
        .unwrap();
    cache
        .mutex
        .lock()
        .unwrap()
        .insert(repo.to_string(), release.clone());
    release
}
//todo: improve ok_or(), or_else()
async fn get_lastest_release(client: &State<Client>, repo: &str) -> Result<Value, reqwest::Error> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let response = client.get(&url).send().await?;
    let github_release = response.json::<Value>().await?;
    create_tauri_response(client, &github_release)
        .await
        .ok_or(json!({}))
        .or_else(|e| Ok(e))
}

async fn create_tauri_response(client: &State<Client>, github_release: &Value) -> Option<Value> {
    let platforms_available: HashMap<&str, Vec<&str>> = HashMap::from([
        ("amd64.AppImage.tar.gz", vec!["linux-x86_64"]),
        ("app.tar.gz", vec!["darwin-x86_64", "darwin-aarch64"]),
        ("x64_en-US.msi.zip", vec!["windows-x86_64"]),
    ]);
    let mut response = json!({
        "version": github_release["tag_name"].as_str()?,
        "notes": remove_suffix(&github_release["body"].as_str()?, "See the assets to download this version and install.").trim_end_matches(['\r','\n',' ']),
        "pub_date": github_release["published_at"].as_str()?,
        "platforms": {}
    });
    let response_platforms = response["platforms"].as_object_mut()?;
    for asset in github_release["assets"].as_array()?.iter() {
        let asset = asset.as_object()?;
        let asset_name = asset["name"].as_str()?;
        let browser_download_url = asset["browser_download_url"].as_str()?;
        for (extension, os_arch) in platforms_available.iter() {
            if asset_name.ends_with(extension) {
                for os_arch in os_arch.iter() {
                    if !response_platforms.contains_key(*os_arch) {
                        response_platforms.insert(os_arch.to_string(), json!({}));
                    }
                    response_platforms[*os_arch].as_object_mut()?.insert(
                        "url".to_string(),
                        Value::String(browser_download_url.to_string()),
                    );
                }
            } else if asset_name.ends_with(&format!("{extension}.sig")) {
                let signature = match text_request(client, browser_download_url).await {
                    Ok(s) => s,
                    _ => String::new(),
                };
                for os_arch in os_arch.iter() {
                    if !response_platforms.contains_key(*os_arch) {
                        response_platforms.insert(os_arch.to_string(), json!({}));
                    }
                    response_platforms[*os_arch]
                        .as_object_mut()?
                        .insert("signature".to_string(), Value::String(signature.clone()));
                }
            }
        }
    }

    Some(response)
}

fn create_ttl_cache(ttl: u64) -> StringValueCache {
    Arc::new(Mutex::new(LruCache::with_expiry_duration(
        Duration::from_secs(ttl),
    )))
}

#[launch]
fn rocket() -> _ {
    let tauri_gh_cache = TauriGHReleaseCache {
        mutex: create_ttl_cache(RELEASE_TTL),
    };
    let client = reqwest::Client::builder()
        .user_agent("reqwest")
        .build()
        .unwrap();

    rocket::build()
        .manage(client)
        .manage(tauri_gh_cache)
        .mount("/", routes![index])
        .mount(TAURI_RELEASES_PREFIX, routes![google_keep_desktop_api])
}
