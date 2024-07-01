use ssr_rs::Ssr;
use std::fs::read_to_string;
use std::time::Instant;

fn main() {
    let source = read_to_string("./tests/assets/react-17-iife.js").unwrap();

    let start = Instant::now();
    let mut ssr = Ssr::new();
    ssr.load(&source, "", "cjs").unwrap();
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
