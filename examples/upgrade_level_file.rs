use std::fs::File;
use std::io::BufReader;

use bevy_yoleck::level_files_upgrading::upgrade_level_file;

fn main() -> anyhow::Result<()> {
    let file = File::open("assets/levels2d/example.yol")?;
    let reader = BufReader::new(file);
    let mut level: serde_json::Value = serde_json::from_reader(reader)?;
    // println!("Old: {:#?}", level);
    level = upgrade_level_file(level)?;
    println!("New: {:?}", level);
    Ok(())
}
