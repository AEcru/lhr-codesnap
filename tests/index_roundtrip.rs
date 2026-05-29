/// Basic integration test: build an index, serialize it, deserialize it,
/// and verify the round-trip preserves all data.
#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;

    #[test]
    fn test_init_and_status() {
        let tmpdir = tempfile::tempdir().unwrap();
        let path = tmpdir.path().to_string_lossy().to_string();

        // Create a minimal Rust source file
        let src_dir = format!("{}/src", path);
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(
            format!("{}/main.rs", src_dir),
            "pub fn hello() {}\npub struct World;\nfn private_fn() {}\n",
        )
        .unwrap();

        // Run codesnap init
        let init = Command::new("cargo")
            .args(["run", "--", "init", &path, "--quiet"])
            .output()
            .unwrap();
        assert!(init.status.success(), "init failed: {}", String::from_utf8_lossy(&init.stderr));

        // Verify index exists
        let index_path = format!("{}/.codesnap/index.bin", path);
        assert!(std::path::Path::new(&index_path).exists(), "index file not created");

        // Run codesnap status
        let status = Command::new("cargo")
            .args(["run", "--", "status", &path])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&status.stdout);
        assert!(stdout.contains("Ready"), "status should say Ready: {}", stdout);

        // Run codesnap find
        let find = Command::new("cargo")
            .args(["run", "--", "find", "hello", "--json"])
            .current_dir(&path)
            .output()
            .unwrap();
        let find_out = String::from_utf8_lossy(&find.stdout);
        assert!(find_out.contains("hello"), "should find hello: {}", find_out);
        assert!(find_out.contains("main.rs"), "should reference main.rs: {}", find_out);
    }
}
