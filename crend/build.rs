fn main() {
    let mut builder = cc::Build::new();

    match std::env::var("VULKAN_SDK") {
        Ok(path) => {
            let sdk_path = std::path::Path::new(&path);
            builder.include(sdk_path.join("include/vulkan"));
        },
        Err(_) => println!("Warning: $VULKAN_SDK is blank.")
    }

    builder
        .file("main.c")
        .compile("crend");
}