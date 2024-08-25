use ssr_rs::Ssr;
use std::cell::RefCell;
use std::fs::read_to_string;
use std::path::Path;
use std::thread;
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

fn main() {
    let threads: Vec<_> = (0..2)
        .map(|i| {
            thread::spawn(move || {
                println!("Thread #{i} started!");
                let start = Instant::now();
                println!(
                    "result: {}",
                    SSR.with(|ssr| ssr.borrow_mut().render_to_string(None).unwrap())
                );
                println!(
                    "Thread #{i} finished! - Elapsed time: {:?}",
                    start.elapsed()
                );
            })
        })
        .collect();

    for handle in threads {
        handle.join().unwrap();
    }
}
