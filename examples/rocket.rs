#[macro_use]
extern crate rocket;
use rocket::fs::FileServer;
use rocket::response::content;
use ssr_rs::Ssr;
use std::cell::RefCell;
use std::fs::read_to_string;
use std::path::Path;
use std::time::Instant;

thread_local! {
    static SSR: RefCell<Ssr> = RefCell::new({
        let mut ssr = Ssr::new();
        ssr.load(
            &read_to_string(Path::new("./tests/assets/react-17-iife.js").to_str().unwrap()).unwrap(),
            "",
            "cjs"
        ).unwrap();
        ssr
    });
}

#[get("/")]
fn index() -> content::RawHtml<String> {
    let start = Instant::now();
    let result = SSR.with(|ssr| ssr.borrow_mut().render_to_string(None));
    println!("Elapsed: {:?}", start.elapsed());
    content::RawHtml(result.unwrap())
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/styles", FileServer::from("./client/dist/ssr/styles"))
        .mount("/scripts", FileServer::from("./client/dist/client/"))
        .mount("/images", FileServer::from("./client/dist/ssr/images/"))
        .mount("/", routes![index])
}
