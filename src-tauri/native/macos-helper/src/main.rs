mod policy;
mod protocol;
mod watchdog;

#[cfg(target_os = "macos")]
mod platform;

fn main() {
    #[cfg(target_os = "macos")]
    if platform::run().is_err() {
        std::process::exit(1);
    }
    #[cfg(not(target_os = "macos"))]
    compile_error!("The lid helper is only supported on macOS");
}
