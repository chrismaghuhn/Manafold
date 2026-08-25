//! Thin process entry point for the temporary M2.H semantic adapter.
//!
//! Reads the trusted-key environment variable once at startup and hands
//! the standard streams to the library session loop. A missing or
//! non-UTF-8 key leaves every trusted command uniformly unauthorized.

use std::io::{self, BufReader};
use std::process;

const TRUSTED_KEY_ENV: &str = "MTGML_M2_ADAPTER_TRUSTED_KEY";

fn main() {
    let trusted_key = std::env::var(TRUSTED_KEY_ENV).ok();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = BufReader::new(stdin.lock());
    let mut output = stdout.lock();
    let code = m2_semantic_adapter::run(&mut input, &mut output, trusted_key);
    process::exit(code);
}
