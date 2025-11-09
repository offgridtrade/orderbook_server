fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new().compile_protos(&["proto/messages.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/messages.proto");
    Ok(())
}
