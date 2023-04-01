use std::{cmp::Ordering, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=./build.rs");

    let out_dir = "./generated/";
    println!("cargo:rerun-if-changed={out_dir}minimuxer-helpers.swift");
    let root = "./src/";

    let mut bridges: Vec<PathBuf> = std::fs::read_dir(root)
        .unwrap()
        .map(|res| res.unwrap().path())
        .collect();
    for path in &bridges {
        let path = path.file_name().unwrap().to_str().unwrap().to_string();
        println!("cargo:rerun-if-changed={root}{path}");
    }
    // Ensure we generate for lib.rs first
    bridges.sort_by(|a, b| {
        if a.file_name().unwrap().to_str().unwrap().ends_with("lib.rs") {
            Ordering::Less
        } else if b.file_name().unwrap().to_str().unwrap() == "lib.rs" {
            Ordering::Greater
        } else {
            a.cmp(b)
        }
    });

    swift_bridge_build::parse_bridges(bridges)
        .write_all_concatenated(out_dir, env!("CARGO_PKG_NAME"));

    // move the generated headers/Swift out of the crate directory
    let crate_dir = format!("{out_dir}{}", env!("CARGO_PKG_NAME"));
    for path in std::fs::read_dir(&crate_dir).unwrap() {
        let path = path
            .unwrap()
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        std::fs::rename(format!("{crate_dir}/{path}"), format!("{out_dir}{path}")).unwrap();
    }
    std::fs::remove_dir(crate_dir).unwrap();

    for path in std::fs::read_dir(out_dir).unwrap() {
        let path = path
            .unwrap()
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let out_path = format!("{out_dir}{path}");

        // remove duplicate lines from minimuxer.h (see https://github.com/chinedufn/swift-bridge/issues/207)
        if path.ends_with(&format!("{}.h", env!("CARGO_PKG_NAME"))) {
            let input = std::fs::read_to_string(&out_path)
                .unwrap()
                // sometimes swift-bridge won't add a newline after defintions
                .replace(";typedef", ";\ntypedef");
            let mut output: Vec<String> = vec![];
            for line in input.split("\n") {
                let line = line.to_owned();
                if !output.contains(&line) || line.len() <= 0 || line.starts_with("//") {
                    output.push(line);
                }
            }
            std::fs::write(out_path, output.join("\n")).unwrap();
        // Add our custom things to minimuxer.swift
        } else if path.ends_with("minimuxer.swift") {
            let input = std::fs::read_to_string(&out_path).unwrap();
            std::fs::write(
                out_path,
                format!(
                    "{input}

// tell Swift the MinimuxerError enum can be thrown
// TODO: do this through swift-bridge instead of manually
extension MinimuxerError: Error {{}}
"
                ),
            )
            .unwrap();
        }
    }
}
