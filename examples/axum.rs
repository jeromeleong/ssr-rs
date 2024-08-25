use axum::{response::Html, routing::get, Router};
use ssr_rs::Ssr;
use std::cell::RefCell;
use std::fs::read_to_string;
use std::path::Path;
use std::time::Instant;

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

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new().route("/", get(root));

    // run our app with hyper, listening globally on port 8080
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> Html<String> {
    let start = Instant::now();
    let result = SSR.with(|ssr| ssr.borrow_mut().render_to_string(None));
    println!("Elapsed: {:?}", start.elapsed());
    Html(result.unwrap())
}
