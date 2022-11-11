mod suite_one;
mod suite_two;

use crate::suite_one::suite_1;
use crate::suite_two::suite_2;

fn main() -> anyhow::Result<()> {

    let toml1 = suite_1();
    let toml2 = suite_2();
    toml1.build()?;
    toml2.build()?;
    Ok(())
}
