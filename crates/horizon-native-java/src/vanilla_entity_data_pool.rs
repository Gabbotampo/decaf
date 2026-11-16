//! Captured Java 25 / Minecraft 26.3 Entity class table.
const TABLE: &str = include_str!("vanilla_entity_data_pool.tsv");
pub(crate) fn row(class: &str) -> Option<(Option<&str>, Option<u16>)> {
    let line = TABLE
        .lines()
        .find(|line| line.split_once('\t').is_some_and(|(name, _)| name == class))?;
    let mut parts = line.split('\t');
    let _ = parts.next()?;
    let parent = parts.next()?;
    let max = parts.next()?;
    Some((
        (!parent.is_empty() && parent != "none").then_some(parent),
        max.parse::<u16>().ok(),
    ))
}
