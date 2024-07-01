use ssr_rs::Ssr;
use std::fs::read_to_string;

#[test]
fn renders_react_17_exported_as_iife() {
    let source = read_to_string("./tests/assets/react-17-iife.js").unwrap();

    let mut ssr = Ssr::new();
    ssr.load(&source, "", "cjs").unwrap();

    let html = ssr.render_to_string(None).unwrap();

    assert_eq!(html, "<div></div>");
}

#[test]
fn renders_react_18_exported_as_iife() {
    let source = read_to_string("./tests/assets/react-18-iife.js").unwrap();

    let mut ssr = Ssr::new();
    ssr.load(&source, "", "cjs").unwrap();

    let html = ssr.render_to_string(None).unwrap();

    assert_eq!(html, "<div></div>");
}
