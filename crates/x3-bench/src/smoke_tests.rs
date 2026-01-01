#[cfg(test)]
mod tests {
    #[test]
    fn smoke_test_harness_runs() {
        // Ensure TOML parsing of a minimal config works
        let toml = r#"
            [[benchmarks]]
            name = "example"
            x3_files = ["examples/hello.x3"]
            iterations = 2
        "#;

        #[derive(serde::Deserialize)]
        struct Benchmark { name: String, x3_files: Vec<String>, iterations: Option<u32> }
        #[derive(serde::Deserialize)]
        struct BenchConfig { benchmarks: Vec<Benchmark> }

        let cfg: BenchConfig = toml::from_str(toml).expect("parse toml");
        assert_eq!(cfg.benchmarks.len(), 1);
        assert_eq!(cfg.benchmarks[0].iterations, Some(2));
    }
}
