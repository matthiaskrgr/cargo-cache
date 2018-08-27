fn pad_strings(indent_lvl: i8, beginning: &str, end: &str) -> String {
    const MAX_WIDTH: i8 = 37;
    let len_padding: i8 = (MAX_WIDTH + indent_lvl * 2) - (beginning.len() as i8);
    //assert!(len_padding > 0); // this fires
    let mut formatted_line = beginning.to_string();
    formatted_line.push_str(&String::from(" ").repeat(len_padding as usize));
    formatted_line
}

fn main() {
    let output_is =
        pad_strings(2, "Size of 1938493989 crate source checkouts: ", "bla").to_string();
}
