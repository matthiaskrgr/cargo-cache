fn pad_strings(indent_lvl: i8, beginning: &str) -> String {
    const MAX_WIDTH: i8 = 37;
    let len_padding: i8 = (MAX_WIDTH + indent_lvl * 2) - (beginning.len() as i8);
    println!("padding: {}", len_padding);
    let mut formatted_line = beginning.to_string();
    // here we cast a negative value to usize
    formatted_line.push_str(&String::from(" ").repeat(len_padding as usize));
    formatted_line
}

fn main() {
        pad_strings(2, "Size of 1938493989 crate source checkouts: ").to_string();
}
