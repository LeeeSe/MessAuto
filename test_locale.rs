use sys_locale::get_locale;

fn main() {
    match get_locale() {
        Some(locale) => println!("System locale: {}", locale),
        None => println!("Could not determine system locale"),
    }
}
