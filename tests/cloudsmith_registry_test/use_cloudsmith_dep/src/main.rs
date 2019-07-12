use helloworld::hello_world;

fn main() {
	let x = hello_world(String::from("matthiaskrgr"));
    println!("print: {:?}", x);
    assert_eq!(x, String::from("Hello world from: matthiaskrgr"));
}
