//! Integration tests for ObjStor


#[cfg(test)]
mod integration_tests {
    use std::fs;
    use std::path::PathBuf;

    fn cleanup_test_data() {
        let _ = fs::remove_dir_all("./data/test");
    }

    #[test]
    fn test_config_file_loading() {
        // Test that config file can be loaded
        let config_path = PathBuf::from("data/config/objstor.json");
        if config_path.exists() {
            let content = fs::read_to_string(&config_path);
            assert!(content.is_ok(), "Failed to read config file");

            let content = content.unwrap();
            assert!(
                content.contains("server"),
                "Config should contain server section"
            );
            assert!(
                content.contains("storage"),
                "Config should contain storage section"
            );
        }
    }

    #[test]
    fn test_directory_structure() {
        // Check that required directories exist or can be created
        let dirs = ["./data", "./data/config", "./data/pools", "./logs"];

        for dir in dirs {
            let path = PathBuf::from(dir);
            if !path.exists() {
                fs::create_dir_all(&path)
                    .unwrap_or_else(|_| panic!("Failed to create directory: {}", dir));
            }
            assert!(path.exists(), "Directory should exist: {}", dir);
        }
    }

    #[test]
    fn test_documentation_exists() {
        // Check that key documentation files exist
        let docs = [
            "./README.md",
            "./BUILD.md",
            "./docs/CONFIGURATION.md",
            "./docs/DOCKER.md",
        ];

        for doc in docs {
            let path = PathBuf::from(doc);
            assert!(path.exists(), "Documentation should exist: {}", doc);
        }
    }

    #[test]
    fn test_docker_files_exist() {
        // Check that Docker files exist and are valid
        let docker_files = ["./Dockerfile", "./docker-compose.yml", "./.dockerignore"];

        for file in docker_files {
            let path = PathBuf::from(file);
            assert!(path.exists(), "Docker file should exist: {}", file);
        }

        // Verify Dockerfile is not empty
        let dockerfile_content = fs::read_to_string("./Dockerfile").unwrap();
        assert!(
            dockerfile_content.contains("FROM"),
            "Dockerfile should contain FROM instruction"
        );
        assert!(
            dockerfile_content.contains("EXPOSE"),
            "Dockerfile should contain EXPOSE instruction"
        );
    }

    #[test]
    fn test_makefile_exists() {
        let path = PathBuf::from("./Makefile");
        assert!(path.exists(), "Makefile should exist");

        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("docker-build"),
            "Makefile should have docker-build target"
        );
        assert!(
            content.contains("docker-run"),
            "Makefile should have docker-run target"
        );
    }

    #[test]
    fn test_scripts_executable() {
        // Check that shell scripts are executable
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;

        let scripts = [
            "./scripts/configure.sh",
            "./scripts/docker-build.sh",
            "./scripts/docker-push.sh",
        ];

        for script in scripts {
            let path = PathBuf::from(script);
            if path.exists() {
                let metadata = fs::metadata(&path).unwrap();
                let permissions = metadata.permissions();
                #[cfg(unix)]
                let is_executable = permissions.mode() & 0o111 != 0;
                #[cfg(not(unix))]
                let is_executable = true;
                assert!(is_executable, "Script should be executable: {}", script);
            }
        }
    }

    #[test]
    fn test_configuration_examples() {
        // Check that example configuration exists
        let examples = [
            "./examples/storage.example.json",
            "./examples/prometheus.yml",
            "./examples/docker-compose-metrics.yml",
        ];

        for example in examples {
            let path = PathBuf::from(example);
            assert!(path.exists(), "Example should exist: {}", example);
        }
    }

    #[test]
    fn test_rust_compilation() {
        // Test that the project compiles
        use std::process::Command;

        let output = Command::new("cargo").args(&["check", "--quiet"]).output();

        match output {
            Ok(output) => {
                let success = output.status.success();
                assert!(success, "Project should compile without errors");
            }
            Err(_) => {
                // If cargo is not available, skip this test
                println!("Warning: cargo not found, skipping compilation test");
            }
        }
    }
}
