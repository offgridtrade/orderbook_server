fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new().compile_protos(&["proto/client.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/client.proto");
    Ok(())
}
