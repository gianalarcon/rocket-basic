use rocket::http::uri::Origin;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::serde::json::{json, Value};

#[macro_use]
extern crate rocket;

const TAURI_RELEASES_PREFIX: Origin<'static> = uri!("/tauri-releases");

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
fn google_keep_desktop_api(
    platform: &str,
    current_version: &str,
    msg: Option<&str>,
) -> Result<Value, Status> {
    if let Some(msg) = msg {
        print!("{msg}");
        return Err(Status::NoContent);
    }
    Ok(json!({
        "notes": "IT WORKS"
    }))
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount(TAURI_RELEASES_PREFIX, routes![google_keep_desktop_api])
}
