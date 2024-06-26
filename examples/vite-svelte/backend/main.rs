use salvo::prelude::*;
use std::cell::RefCell;
use std::fs::read_to_string;
use std::path::Path;
use ssr_rs::Ssr;

thread_local! {
    static SSR: RefCell<Ssr<'static, 'static>> = RefCell::new(
        Ssr::from(
            read_to_string(Path::new("./dist/server/server.js").to_str().unwrap()).unwrap(),
            "render", // Make sure the exported function is named "render"
            "esm" // Use ESM module
        ).unwrap_or_else(|err| {
            eprintln!("Failed to initialize SSR: {}", err);
            std::process::exit(1);
        })
    )
}

#[handler]
async fn index(res: &mut Response) {
    let result = SSR.with(|ssr| {
        let mut ssr = ssr.borrow_mut();
        ssr.render_to_string(None).unwrap_or_else(|err| {
            eprintln!("Error rendering to string: {}", err);
            String::new()
        })
    });

    if result.is_empty() {
        eprintln!("Rendered result is empty");
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        res.render(Text::Plain("Internal Server Error"));
        return;
    }

    //println!("Rendered result: {}", result); // For debugging

    let result: serde_json::Value = match serde_json::from_str(&result) {
        Ok(val) => val,
        Err(err) => {
            eprintln!("Failed to parse JSON: {}", err);
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Text::Plain("Internal Server Error"));
            return;
        }
    };

    let html = result["html"].as_str().unwrap_or("");
    let css = result["css"].as_str().unwrap_or("");
    
    let full_html = format!(
        r#"<!DOCTYPE html>
        <html>
        <head>
            <style>{}</style>
            <link rel="stylesheet" href="/client/assets/main.css">
        </head>
        <body>
            <div id="svelte-app">{}</div>
            <script type="module" src="/client/main.js"></script>
        </body>
        </html>"#,
        css, html
    );
    res.render(Text::Html(full_html));
}

#[tokio::main]
async fn main() {
    Ssr::create_platform();
    let router = Router::new()
        .push(Router::with_path("/client/<**path>")
            .get(StaticDir::new(["./dist/client"])))
        .push(Router::with_path("/client/assets/<**path>")
            .get(StaticDir::new(["./dist/assets/client"])))
        .push(Router::with_path("/").get(index));

    let acceptor = TcpListener::new("127.0.0.1:8080").bind().await;

    tracing_subscriber::fmt().init();
    tracing::info!("Listening on http://{:?}", acceptor.local_addr());

    Server::new(acceptor).serve(router).await;
}
