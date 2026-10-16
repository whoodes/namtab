#[path = "../mk1/mod.rs"]
mod mk1;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    mk1::serve(mk1::Config::from_args()?)
}
