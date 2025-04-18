mod model;

use model::{get_apis, Api};

fn main() -> anyhow::Result<()> {
    let apis: Vec<Api> = get_apis()?;
    Ok(())
}
