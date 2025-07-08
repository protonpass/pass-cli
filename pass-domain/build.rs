use std::path::{Path, PathBuf};

fn main() {
    generate_protos();
}

fn generate_protos() {
    generate_pass_protos();
}

fn generate_pass_protos() {
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("protos");

    let files = vec![
        ("file_v1.proto", "file"),
        ("item_v1.proto", "item"),
        ("vault_v1.proto", "vault"),
    ];

    let mut mod_file_content = String::new();
    for (proto_file, mod_name) in files {
        generate_proto(proto_file, out_dir.join(mod_name));
        mod_file_content.push_str(&format!("pub mod {mod_name};\n"));
    }

    let mod_file_name = out_dir.join("mod.rs");
    std::fs::write(mod_file_name, mod_file_content).expect("Couldn't write mod file");
}

fn generate_proto(filename: &str, out_dir: PathBuf) {
    println!("cargo:rerun-if-changed=proto/{filename}");
    let proto_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("proto");
    let proto_path = proto_dir.join(filename);
    if !out_dir.exists() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(&out_dir)
            .expect("error creating out dir");
    }

    protobuf_codegen::Codegen::new()
        .protoc()
        .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
        .include(proto_dir)
        .input(proto_path)
        .out_dir(out_dir)
        .run()
        .expect("failed to generate rust from proto");
}
