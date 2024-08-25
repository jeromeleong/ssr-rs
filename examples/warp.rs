#![deny(warnings)]
use ssr_rs::Ssr;
use std::cell::RefCell;
use std::fs::read_to_string;
use std::path::Path;
use std::time::Instant;
use warp::{http::Response, Filter};

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
    let html = warp::path::end().map(move || {
        let start = Instant::now();
        let result = SSR.with(|ssr| ssr.borrow_mut().render_to_string(None));
        println!("Elapsed: {:?}", start.elapsed());
        Response::builder().body(result.unwrap())
    });

    let css = warp::path("styles").and(warp::fs::dir("./client/dist/ssr/styles/"));
    let scripts = warp::path("scripts").and(warp::fs::dir("./client/dist/client/"));
    let img = warp::path("images").and(warp::fs::dir("./client/dist/ssr/images/"));

    let routes = warp::get().and(html.or(css).or(scripts).or(img));

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
