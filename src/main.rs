use reqwest::Client;
use rocket::http::uri::Origin;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::serde::json::Value;
use rocket::State;

#[macro_use]
extern crate rocket;

const TAURI_RELEASES_PREFIX: Origin<'static> = uri!("/tauri-releases");
const GOOGLE_KEEP_DESKTOP_REPO: &str = "elibroftw/google-keep-desktop-app";
#[get("/")]
fn index() -> Redirect {
    let msg: Option<&str> = None;
    let platform = "linux-x64";
    let current_version = "v1.0.14";
    Redirect::to(uri!(
        TAURI_RELEASES_PREFIX,
        google_keep_desktop_api(platform, current_version, msg)
    ))
}

#[get("/google-keep-desktop/<platform>/<current_version>?<msg>")]
async fn google_keep_desktop_api(
    platform: &str,
    current_version: &str,
    msg: Option<&str>,
    client: &State<Client>,
) -> Result<Value, Status> {
    if let Some(msg) = msg {
        println!("{msg}");
        return Err(Status::NoContent);
    }
    get_lastest_release(client, GOOGLE_KEEP_DESKTOP_REPO)
        .await
        .or(Err(Status::NoContent))
}

async fn get_lastest_release(client: &State<Client>, repo: &str) -> Result<Value, reqwest::Error> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let response = client.get(&url).send().await?;
    let github_release = response.json::<Value>().await?;
    Ok(github_release)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(
            reqwest::Client::builder()
                .user_agent("reqwest")
                .build()
                .unwrap(),
        )
        .mount("/", routes![index])
        .mount(TAURI_RELEASES_PREFIX, routes![google_keep_desktop_api])
}
