use ssr_rs::Ssr;
use std::fs::read_to_string;

#[test]
fn renders_svelte_exported_as_esm() {
    let source = read_to_string("./tests/assets/svelte-4-esm.js").unwrap();

    let mut ssr = Ssr::new();
    ssr.load(&source, "render", "esm").unwrap();

    let html = ssr.render_to_string(None).unwrap();

    assert_eq!(html, "<div></div>");
}
