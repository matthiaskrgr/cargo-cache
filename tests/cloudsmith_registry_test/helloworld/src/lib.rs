pub fn hello_world(input: String) -> String {
    let msg = format!("Hello world from: {}", input);
    eprintln!("{}", msg);
    msg
}

#[cfg(test)]
mod tests {
    use super::hello_world;
    #[test]
    fn it_works() {
        assert_eq!(
            hello_world("Matthias".to_string()),
            "Hello world from: Matthias".to_string()
        );
    }
}
