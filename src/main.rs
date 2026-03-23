//! Downloads Intelligence Scanner CLI
//!
//! Usage: nexcore-downloads-scanner [path]
//! Default: ~/Downloads

#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "CLI tool requires output"
)]

use nexcore_downloads_scanner::ScanReport;
use std::path::PathBuf;

fn main() {
    let target = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
            PathBuf::from(format!("{home}/Downloads"))
        });

    eprintln!("Scanning: {}", target.display());

    match ScanReport::scan(&target) {
        Ok(report) => print!("{}", report.display()),
        Err(e) => eprintln!("Error: {e}"),
    }
}
