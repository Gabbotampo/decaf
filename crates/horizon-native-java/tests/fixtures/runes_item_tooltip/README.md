# Item tooltip fixture captures

The expected component JSON in `fixture_runes_item_tooltip.rs` was captured by running the same tooltip overrides against the real Minecraft 26.3 server classes on JDK 25. The CI fixture uses only signature stubs; the runtime supplies the modeled component and item behavior.

The real Runes test is opt-in through `RUNES_JAR` so repository tests never depend on a machine-local corpus path.
