use ssr_rs::Ssr;
use std::fs::read_to_string;
use std::sync::Once;

static INIT: Once = Once::new();

fn prepare() {
    INIT.call_once(|| {
        Ssr::create_platform();
    })
}

#[test]
fn renders_svelte_exported_as_esm() {
    prepare();

    let source = read_to_string("./tests/assets/svelte-4-esm.js").unwrap();

    let mut js = Ssr::from(&source, "", "esm").unwrap();

    let html = js.render_to_string(None).unwrap();

    assert_eq!(html, "<div></div>");
}
