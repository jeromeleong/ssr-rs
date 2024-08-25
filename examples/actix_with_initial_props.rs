use actix_files as fs;
use actix_web::{get, http::StatusCode, App, HttpResponse, HttpServer};
use std::cell::RefCell;
use std::fs::read_to_string;
use std::path::Path;

use ssr_rs::Ssr;

thread_local! {
    static SSR: RefCell<Ssr> = RefCell::new({
        let ssr = Ssr::new();
        ssr.load(
            &read_to_string(Path::new("./tests/assets/react-17-iife.js").to_str().unwrap()).unwrap(),
            "",
            "cjs"
        ).unwrap();
        ssr
    });
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(fs::Files::new("/styles", "client/dist/ssr/styles/").show_files_listing())
            .service(fs::Files::new("/images", "client/dist/ssr/images/").show_files_listing())
            .service(fs::Files::new("/scripts", "client/dist/client/").show_files_listing())
            .service(index)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

#[get("/")]
async fn index() -> HttpResponse {
    let mock_props = r##"{
        "params": [
            "hello",
            "ciao",
            "こんにちは"
        ]
    }"##;

    HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(SSR.with(|ssr| ssr.borrow_mut().render_to_string(Some(mock_props)).unwrap()))
}
