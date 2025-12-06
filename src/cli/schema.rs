use crate::config::Config;
use schemars::schema_for;

pub fn execute() -> anyhow::Result<()> {
    let schema = schema_for!(Config);
    let json = serde_json::to_string_pretty(&schema)?;
    println!("{}", json);
    Ok(())
}
