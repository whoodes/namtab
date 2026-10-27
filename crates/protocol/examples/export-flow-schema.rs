fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = if std::env::args().nth(1).as_deref() == Some("response") {
        schemars::schema_for!(project_x_protocol::flow::Response)
    } else {
        schemars::schema_for!(project_x_protocol::flow::Request)
    };
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
