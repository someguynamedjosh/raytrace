fn main() {
    let mut builder = cc::Build::new();

    match std::env::var("VULKAN_SDK") {
        Ok(path) => {
            let sdk_path = std::path::Path::new(&path);
            builder.include(sdk_path.join("include/vulkan"));
            println!(
                "cargo:rustc-link-search={}",
                sdk_path.join("lib").to_str().unwrap()
            );
        }
        Err(_) => println!("Warning: $VULKAN_SDK is blank."),
    }

    println!("cargo:rustc-link-lib=vulkan");

    builder.file("main.c").compile("crend");
}
