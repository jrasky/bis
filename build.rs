extern crate gcc;

fn main() {
    gcc::compile_library("libbis_c.a", &["src/bis_c.c"]);
}
