fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("src/policy.proto").unwrap_or_else(|err| {
        eprintln!("ERROR : Build rs failed due to {err}");
    });
    Ok(())
}
