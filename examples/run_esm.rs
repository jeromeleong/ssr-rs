use ssr_rs::Ssr;
use std::fs::read_to_string;
use std::time::Instant;

fn main() {
    let source = read_to_string("./tests/assets/svelte-4-esm.js").unwrap();

    let start = Instant::now();
    let mut ssr = Ssr::new();
    ssr.load(&source, "render", "esm").unwrap();
    let duration = start.elapsed();
    println!("Ssr creation and loading took: {:?}", duration);

    let start = Instant::now();
    println!("{}", ssr.render_to_string(None).unwrap());
    let duration = start.elapsed();
    println!("First render took: {:?}", duration);

    let start = Instant::now();
    println!("{}", ssr.render_to_string(None).unwrap());
    let duration = start.elapsed();
    println!("Second render took: {:?}", duration);
}
