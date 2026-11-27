//! Diana binary entry point: argument parsing and classified exit codes.

// GNU malloc supplies natural alignment directly on the normal allocation path.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
use mimalloc as _;

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> std::process::ExitCode {
    diana::cli::run()
}
