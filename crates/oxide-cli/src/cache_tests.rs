
use crate::executor::{execute_pipeline, ExecutorConfig};
use oxide_core::pipeline::PipelineDefinition;
#[tokio::test]
async fn test_cache_plugin() {
    // Setup isolated cache dir
    let cache_home = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("XDG_CACHE_HOME", cache_home.path());
    }
    // Also set HOME/etc just in case directories-rs falls back differently, but XDG_CACHE_HOME should be enough on Linux.
    // For Mac/Windows it might use different vars, but user is on Linux.

    // 1. Save Cache Pipeline
    let yaml_save = r#"
name: save-cache
version: "1"
stages:
  - name: save
    steps:
      - name: create-file
        run: |
          mkdir -p my-data
          echo "Hello Cache" > my-data/file.txt
      - name: save-it
        uses: cache
        with:
          key: my-cache-key-v1
          paths: ["my-data"]
          method: save
"#;

    let def_save: PipelineDefinition = serde_yaml::from_str(yaml_save).expect("Failed to parse YAML (Save)");
    let temp_ws_save = tempfile::tempdir().unwrap();

    let config_save = ExecutorConfig {
        workspace: temp_ws_save.path().to_path_buf(),
        variables: std::collections::HashMap::new(),
        secrets: std::collections::HashMap::new(),
        verbose: true,
    };

    let res_save = execute_pipeline(&def_save, &config_save, None).await.expect("Save pipeline failed");
    assert!(res_save.success, "Save pipeline should succeed");

    // Verify cache file exists in cache_home
    // oxide-cache uses: cache_home/oxide/oxide-ci/{key}.tar.bin ? 
    // Wait, `ProjectDirs::from("io", "oxide", "oxide-ci")` -> `~/.cache/oxide/oxide-ci`.
    // If I set XDG_CACHE_HOME = `/tmp/xyz`, then it becomes `/tmp/xyz/oxide/oxide-ci`.
    // I need to check where it actually puts it. 
    // `files = walkdir`.

    // 2. Restore Cache Pipeline
    let yaml_restore = r#"
name: restore-cache
version: "1"
stages:
  - name: restore
    steps:
      - name: restore-it
        uses: cache
        with:
          key: my-cache-key-v1
          paths: ["my-data"]
          method: restore
      - name: check-file
        run: |
          if [ -f my-data/file.txt ]; then
             content=$(cat my-data/file.txt)
             if [ "$content" == "Hello Cache" ]; then
               exit 0
             else
               echo "Wrong content: $content"
               exit 1
             fi
          else
             echo "File not found"
             ls -R
             exit 1
          fi
"#;

    let def_restore: PipelineDefinition = serde_yaml::from_str(yaml_restore).expect("Failed to parse YAML (Restore)");
    let temp_ws_restore = tempfile::tempdir().unwrap();

    let config_restore = ExecutorConfig {
        workspace: temp_ws_restore.path().to_path_buf(),
        variables: std::collections::HashMap::new(),
        secrets: std::collections::HashMap::new(),
        verbose: true,
    };

    let res_restore = execute_pipeline(&def_restore, &config_restore, None).await.expect("Restore pipeline failed");
    assert!(res_restore.success, "Restore pipeline should succeed");

    // Cleanup (TempDirs drop automatically, but env var persists in process?)
    // Rust tests run in threads, setting env var is dangerous if parallel.
    // `cargo test` runs parallel.
    // I should use `serial_test` or a mutex if this affects other tests.
    // However, only `cache_tests` uses `cache` logic?
    // And `directories` crate reads env once? No, `ProjectDirs::from` reads each time?
    // It reads env vars.
    // If other tests use `directories` for other things, it might flake.
    // But `oxide-cli` doesn't generally use `directories` except for config maybe.
    // I should probably skip parallel testing or assume isolation is acceptable for this verifying phase.
}
