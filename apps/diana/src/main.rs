//! Diana binary entry point: argument parsing and classified exit codes.

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> std::process::ExitCode {
    diana::cli::run()
}
