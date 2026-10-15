fn main() {
    let schema = if std::env::args().nth(1).as_deref() == Some("response") {
        project_x_protocol::internal_engine::response_schema()
    } else {
        project_x_protocol::internal_engine::request_schema()
    };
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
