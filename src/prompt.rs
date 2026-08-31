use std::io::{self, Write};

pub fn confirm(prompt: &str) -> bool {
    print!("{prompt} [y/N]\n> ");
    io::stdout().flush().unwrap();

    let mut inp = String::new();
    io::stdin().read_line(&mut inp).unwrap();

    matches!(inp.trim(), "y" | "Y")
}
