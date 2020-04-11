use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn get_vulkan_sdk_path() -> String {
    let vulkan_sdk_path =
        std::env::var("VULKAN_SDK").expect("The environment variable $VULKAN_SDK is blank.");
    if vulkan_sdk_path.len() == 0 {
        panic!("The environment variable $VULKAN_SDK is blank.");
    }
    vulkan_sdk_path
}

fn gen_material_code() {
    println!("cargo:rerun-if-changed=misc/*");
    let material_defs =
        csv::Reader::from_path("misc/materials.csv").expect("Failed to open misc/materials.csv");

    fn parse_number(from: &str, min: i32, max: i32) -> i32 {
        let initial: i32 = from
            .trim()
            .parse()
            .expect("Malformed number in materials.csv");
        if initial < min || initial > max {
            panic!(
                "The value {} in materials.csv is outside the range {}-{}.",
                initial, min, max
            );
        }
        initial
    }

    fn parse_rgb(r: &str, g: &str, b: &str) -> (i32, i32, i32) {
        (
            parse_number(r, 0x00, 0xFF),
            parse_number(g, 0x00, 0xFF),
            parse_number(b, 0x00, 0xFF),
        )
    }

    #[derive(Debug)]
    struct Material {
        index: i32,
        albedo: (i32, i32, i32),
        emission: (i32, i32, i32),
    }

    let mut correct_index = 0;
    let mut materials = Vec::new();
    for item in material_defs.into_records() {
        let item = item.expect("Failed to read materail from materials.csv");
        if item.len() < 8 {
            println!(
                "Material number {} in materials.csv is improperly formatted.",
                correct_index
            );
        }
        let index = parse_number(&item[0], 0, 0xFFFF);
        if index != correct_index {
            println!(
                "Material number {} is incorrectly labeled as being material number {}.",
                correct_index, index
            );
        }
        let albedo = parse_rgb(&item[1], &item[2], &item[3]);
        let emission = {
            let mul = parse_number(&item[7], 0, 9);
            let (r, g, b) = parse_rgb(&item[4], &item[5], &item[6]);
            (r * mul, g * mul, b * mul)
        };
        materials.push(Material {
            index,
            albedo,
            emission,
        });
        correct_index += 1;
    }

    let mut glsl_header = File::create("shaders/glsl/GEN_MATERIALS.glsl")
        .expect("Failed to open shaders/glsl/GEN_MATERIALS.glsl for writing");

    writeln!(glsl_header, "vec3 get_material_albedo(uint material) {{").unwrap();
    writeln!(glsl_header, "\tswitch(material) {{").unwrap();
    for material in &materials {
        writeln!(
            glsl_header,
            "\t\tcase {}: return vec3({}, {}, {});",
            material.index,
            material.albedo.0 as f32 / 255.0,
            material.albedo.1 as f32 / 255.0,
            material.albedo.2 as f32 / 255.0,
        )
        .unwrap();
    }
    writeln!(glsl_header, "\t}}\n}}\n").unwrap();

    writeln!(glsl_header, "vec3 get_material_emission(uint material) {{").unwrap();
    writeln!(glsl_header, "\tswitch(material) {{").unwrap();
    for material in &materials {
        writeln!(
            glsl_header,
            "\t\tcase {}: return vec3({}, {}, {});",
            material.index,
            material.emission.0 as f32 / 255.0,
            material.emission.1 as f32 / 255.0,
            material.emission.2 as f32 / 255.0,
        )
        .unwrap();
    }
    writeln!(glsl_header, "\t}}\n}}\n").unwrap();

    let mut rust_materials = File::create("src/render/GEN_MATERIALS.rs")
        .expect("Failed to open src/render/GEN_MATERIALS.rs for writing");
    writeln!(
        rust_materials,
        r#"
#[derive(Clone)]
pub struct Material {{
    pub albedo: (f32, f32, f32),
    pub emission: (f32, f32, f32),
    pub power: f32,
}}

impl Material {{
    pub fn black() -> Self {{
        Self {{
            albedo: (0.0, 0.0, 0.0),
            emission: (0.0, 0.0, 0.0),
            power: 0.0,
        }}
    }}

	pub fn add(&mut self, other: &Self) {{
		self.albedo.0 += other.albedo.0;
		self.albedo.1 += other.albedo.1;
		self.albedo.2 += other.albedo.2;
		self.emission.0 += other.emission.0;
		self.emission.1 += other.emission.1;
        self.emission.2 += other.emission.2;
        self.power += other.power;
	}}

	pub fn multiply(&mut self, factor: f32) {{
		self.albedo.0 *= factor;
		self.albedo.1 *= factor;
		self.albedo.2 *= factor;
		self.emission.0 *= factor;
		self.emission.1 *= factor;
        self.emission.2 *= factor;
        self.power *= factor;
    }}

    pub fn pack(&self) -> u32 {{
        let ar = (self.albedo.0 / self.power * 0x7F as f32) as u32;
        let ag = (self.albedo.1 / self.power * 0x7F as f32) as u32;
        let ab = (self.albedo.2 / self.power * 0x7F as f32) as u32;
        let albedo = ar << 14 | ag << 7 | ab;
        albedo
    }}
}}

#[rustfmt::skip]"#
    )
    .unwrap();
    writeln!(
        rust_materials,
        "pub const MATERIALS: [Material; {}] = [",
        materials.len()
    )
    .unwrap();
    for (index, material) in materials.iter().enumerate() {
        writeln!(
            rust_materials,
            concat!(
                "\tMaterial {{\n",
                "\t\talbedo:   ({:.9}, {:.9}, {:.9}),\n",
                "\t\temission: ({:.9}, {:.9}, {:.9}),\n",
                "\t\tpower: {:.1},\n",
                "\t}},",
            ),
            material.albedo.0 as f32 / 255.0,
            material.albedo.1 as f32 / 255.0,
            material.albedo.2 as f32 / 255.0,
            material.emission.0 as f32 / 255.0,
            material.emission.1 as f32 / 255.0,
            material.emission.2 as f32 / 255.0,
            if index == 0 { 0.0 } else { 1.0 }
        )
        .unwrap();
    }
    writeln!(rust_materials, "];",).unwrap();
}

fn compile_shaders() {
    let vulkan_sdk_path = get_vulkan_sdk_path();

    println!("cargo:rerun-if-changed=shaders/glsl/*");
    // the spirv folder is ignored by git, so it may be missing when cloning the repo.
    fs::create_dir_all("shaders/spirv/").expect("Failed to create folder shaders/spirv/");

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
        // Assume other extensions to be auxiliary / header files.
        if ![
            Some(OsStr::new("vert")),
            Some(OsStr::new("frag")),
            Some(OsStr::new("comp")),
        ]
        .contains(&file_name.extension())
        {
            continue;
        }
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

fn main() {
    gen_material_code();
    compile_shaders();
}
