use actix_web::{get, http::StatusCode, App, HttpResponse, HttpServer};
use std::cell::RefCell;
use std::fs::read_to_string;
use std::path::Path;

use ssr_rs::Ssr;

thread_local! {
    static SSR: RefCell<Ssr> = RefCell::new({
        let ssr = Ssr::new();
        ssr.load(
            &read_to_string(Path::new("./dist/server-entry.js").to_str().unwrap()).unwrap(),
            "SSR",
            "cjs"
        ).unwrap();
        ssr
    });
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}

#[get("/")]
async fn index() -> HttpResponse {
    let result = SSR.with(|ssr| ssr.borrow_mut().render_to_string(None).unwrap());

    HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(result)
}
