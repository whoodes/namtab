fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = if std::env::args().nth(1).as_deref() == Some("view") {
        project_x_protocol::experimental_native::view_schema()
    } else {
        project_x_protocol::experimental_native::schema()
    };
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
