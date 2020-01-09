use std::fs::{self, File};
use std::path::Path;
use std::process::Command;

fn main() {
    let vulkan_sdk_path =
        std::env::var("VULKAN_SDK").expect("The environment variable $VULKAN_SDK is blank.");
    if vulkan_sdk_path.len() == 0 {
        panic!("The environment variable $VULKAN_SDK is blank.");
    }

    println!("cargo:rerun-if-changed=shaders/glsl/");

    let mut required_compiles = vec![];
    let mut total_shaders = 0;
    for entry in fs::read_dir("shaders/glsl").expect("Failed to list items in ./shaders/glsl") {
        let entry = entry.expect("Failed to list an item in ./shaders/glsl");
        let meta = entry.metadata();
        let meta = meta.expect("Failed to get metadata for a file in ./shaders/glsl");
        if !meta.is_file() {
            // TODO: Make recursive?
            continue;
        }

        let file_name = entry.path();
        let file_name = file_name.as_path().file_name();
        let file_name = file_name.expect("Failed to get file name for shader source.");
        let file_name = file_name.to_str().unwrap().to_owned();
        let source = format!("shaders/glsl/{}", file_name);
        let target = format!("shaders/spirv/{}.spirv", file_name);

        let source_modified = meta
            .modified()
            .expect("Failed to read modification date of source file.");
        let requires_compile = if let Result::Ok(target_file) = File::open(&target) {
            // If the output file exists, we require recompilation if it was modified earlier
            // than its corresponding source file.
            let target_meta = target_file
                .metadata()
                .expect("Failed to read metadata of spirv file.");
            let target_modified = target_meta
                .modified()
                .expect("Failed to read modification date of target file.");
            target_modified < source_modified
        } else {
            // Otherwise, if the output does not exist, we need to compile no matter what.
            true
        };

        if requires_compile {
            required_compiles.push((source, target));
        }

        total_shaders += 1;
    }

    println!(
        "{}/{} shaders do not need to be recompiled.",
        total_shaders - required_compiles.len(),
        total_shaders
    );

    let compiler_path = Path::new(&vulkan_sdk_path);
    let compiler_path = compiler_path.join("bin/glslc");
    for (index, (source, target)) in required_compiles.iter().enumerate() {
        println!(
            "Compiling shader {} of {}.",
            index + 1,
            required_compiles.len()
        );
        let compile_result = Command::new(compiler_path.clone())
            .args(&[source, "-o", target])
            .output()
            .expect("Failed to run shader compiler! Check that your $VULKAN_SDK is correct.");
        if compile_result.stderr.len() > 0 {
            panic!(
                "\n{}\nGLSL COMPILE ERROR: Failed to compile {}:\n\n{}\n",
                "============================================================",
                source,
                String::from_utf8_lossy(&compile_result.stderr)
            );
        }
    }
}
