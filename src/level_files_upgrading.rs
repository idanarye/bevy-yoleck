/// Upgrade a level file to the most recent Yoleck level format
pub fn upgrade_level_file(mut level: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let parts = level
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("Level file must be an array"))?;
    let mut format_version = parts
        .get(0)
        .ok_or_else(|| anyhow::anyhow!("Level file array must not be empty"))?
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Level file header must be an object"))?
        .get("format_version")
        .ok_or_else(|| anyhow::anyhow!("Level file header must have a `format_version` field"))?
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("`format_version` must be a non-negative number"))?;

    for (upgrade_to, upgrade_fn) in [(2, upgrade_level_file_1_to_2)] {
        if format_version < upgrade_to {
            upgrade_fn(parts)?;
            format_version = upgrade_to;
        }
    }

    parts[0].as_object_mut().expect("already verified")["format_version"] = format_version.into();

    Ok(level)
}

fn upgrade_level_file_1_to_2(parts: &mut [serde_json::Value]) -> anyhow::Result<()> {
    let entities = parts
        .get_mut(2)
        .ok_or_else(|| anyhow::anyhow!("Level file must have entites list as third element"))?
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("Entity list must be array"))?;

    for entity in entities.iter_mut() {
        let entity_type = entity
            .pointer("/0/type")
            .ok_or_else(|| anyhow::anyhow!("Entity must have a header with a `type` field"))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Entity `type` must be a string"))?
            .to_owned();
        let entity_data = entity
            .get_mut(1)
            .ok_or_else(|| anyhow::anyhow!("Entity must have data"))?;
        let orig_data = entity_data.take();
        *entity_data = serde_json::Value::Object(Default::default());
        entity_data[entity_type] = orig_data;
    }
    Ok(())
}
