use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=./build.rs");

    let out_dir = "./generated/";
    let root = "./src/";

    let bridges: Vec<PathBuf> = std::fs::read_dir(root)
        .unwrap()
        .map(|res| res.unwrap().path())
        .collect();
    for path in &bridges {
        let path = path.file_name().unwrap().to_str().unwrap().to_string();
        println!("cargo:rerun-if-changed={root}{path}");
    }

    swift_bridge_build::parse_bridges(bridges)
        .write_all_concatenated(out_dir, env!("CARGO_PKG_NAME"));
}
