fn slug(title: &str) -> String {
    title.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

fn main() {
    println!("{}", slug("Hi"));
}
