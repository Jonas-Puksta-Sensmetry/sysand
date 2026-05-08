use anyhow::Result;
use sysand_core::index::do_index_init;

pub fn command_index_init() -> Result<()> {
    do_index_init()?;
    Ok(())
}
