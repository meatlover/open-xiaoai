use crate::utils::shell::run_shell;

/// Show LED with a pattern index (e.g. 1 = breathing, etc.)
pub async fn show(index: u32) {
    let cmd = format!("ubus -t 1 call led show '{{\"L\":{}}}'", index);
    let _ = run_shell(&cmd).await;
}

/// Show LED ring with custom RGB color (always uses pattern 8 = solid color)
pub async fn show_rgb(rgb: &str) {
    let cmd = format!("ubus -t 1 call led show '{{\"L\":8,\"rgb\":\"{}\"}}'", rgb);
    let _ = run_shell(&cmd).await;
}

/// Turn off LED pattern
pub async fn shut(index: u32) {
    let cmd = format!("ubus -t 1 call led shut '{{\"L\":{}}}'", index);
    let _ = run_shell(&cmd).await;
}
