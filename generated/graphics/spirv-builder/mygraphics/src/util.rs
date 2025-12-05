pub fn enable_debug_layer() -> bool {
    std::env::var("DEBUG_LAYER")
        .map(|e| !(e == "0" || e == "false"))
        .unwrap_or(false)
}
