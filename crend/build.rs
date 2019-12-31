fn main() {
    cc::Build::new()
        .file("main.c")
        .compile("crend");
}