use std::fs;
use std::path::Path;

fn main() {
    let proto = Path::new("proto/ibank/v1/ibank.proto");
    let generated = Path::new("src/generated/ibank.v1.rs");
    let descriptor = Path::new("src/generated/ibank_descriptor.bin");

    println!("cargo:rerun-if-changed={}", proto.display());
    println!("cargo:rerun-if-changed={}", generated.display());
    println!("cargo:rerun-if-changed={}", descriptor.display());

    if !generated.exists() {
        panic!(
            "missing generated gRPC source '{}'; commit generated artifacts",
            generated.display()
        );
    }
    if !descriptor.exists() {
        panic!(
            "missing generated descriptor '{}'; run scripts/regenerate_descriptor.sh",
            descriptor.display()
        );
    }

    if let (Ok(proto_meta), Ok(gen_meta), Ok(desc_meta)) = (
        fs::metadata(proto),
        fs::metadata(generated),
        fs::metadata(descriptor),
    ) {
        if let (Ok(proto_mtime), Ok(gen_mtime), Ok(desc_mtime)) = (
            proto_meta.modified(),
            gen_meta.modified(),
            desc_meta.modified(),
        ) {
            if proto_mtime > gen_mtime {
                println!(
                    "cargo:warning=proto '{}' is newer than generated Rust stubs '{}'",
                    proto.display(),
                    generated.display()
                );
            }
            if proto_mtime > desc_mtime {
                println!(
                    "cargo:warning=proto '{}' is newer than descriptor '{}'; run scripts/regenerate_descriptor.sh",
                    proto.display(),
                    descriptor.display()
                );
            }
        }
    }
}
