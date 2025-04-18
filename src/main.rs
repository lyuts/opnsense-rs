mod model;

use anyhow::anyhow;
use model::{get_apis, Api};

fn main() -> anyhow::Result<()> {
    let _apis: Vec<Api> = get_apis()?;

    let conf = ini::Ini::load_from_file(".opn.ini")?;
    let profile_data = conf.section(Some("default")).ok_or(anyhow!("Profile not found."))?;
    let _key = profile_data.get("key").ok_or(anyhow!("key is missing in the profile."))?;
    let _secret = profile_data.get("secret").ok_or(anyhow!("secret is missing in the profile."))?;
    Ok(())
}
