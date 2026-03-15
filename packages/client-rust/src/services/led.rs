use crate::utils::shell::run_shell;

pub async fn show(index: u32) {
    let cmd = format!("ubus call led show '{{\"L\":{}}}'", index);
    let _ = run_shell(&cmd).await;
}

pub async fn show_rgb(index: u32, rgb: &str) {
    let cmd = format!("ubus call led show '{{\"L\":{},\"rgb\":\"{}\"}}'", index, rgb);
    let _ = run_shell(&cmd).await;
}

pub async fn shut(index: u32) {
    let cmd = format!("ubus call led shut '{{\"L\":{}}}'", index);
    let _ = run_shell(&cmd).await;
}
