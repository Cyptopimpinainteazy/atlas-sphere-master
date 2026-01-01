use std::process;

use atlas_sphere_node::service;

fn main() {
	println!("Atlas Sphere Node v{}", env!("CARGO_PKG_VERSION"));
	println!("Dual-VM execution (EVM + SVM) with native assets & atomic cross-chain operations.");
	process::exit(0);
}
