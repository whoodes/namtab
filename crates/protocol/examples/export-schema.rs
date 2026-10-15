fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&project_x_protocol::schema())?
    );
    Ok(())
}
