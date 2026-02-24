extern crate feature_probe;

use feature_probe::Probe;

fn main() {
    let probe = Probe::new();

    if probe.probe_type("u128") {
        enable_cfg("int_128");
    }

    if probe.probe_type("::std::ops::RangeInclusive<u64>") {
        enable_cfg("inclusive_range");
    }
}

/// Enables `--cfg feature` for the current build.
fn enable_cfg(feature: &str) {
    println!("cargo:rustc-cfg={}", feature);
}
