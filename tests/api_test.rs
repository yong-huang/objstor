//! API tests for ObjStor

#[cfg(test)]
mod api_tests {
    use std::process::Command;
    use std::time::Duration;

    fn start_server() -> Result<(), String> {
        // Check if server is already running
        let health_check = Command::new("curl")
            .args(&[
                "-s",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                "http://localhost:8080/health",
            ])
            .output();

        if let Ok(output) = health_check {
            let status_code = String::from_utf8_lossy(&output.stdout);
            if status_code == "200" {
                return Ok(()); // Server already running
            }
        }

        // Try to start server (may fail if already running or cargo not available)
        let _ = Command::new("cargo").args(&["run", "--release"]).spawn();

        // Wait for server to be ready
        std::thread::sleep(Duration::from_secs(5));

        Ok(())
    }

    #[test]
    #[ignore] // Run this manually with: cargo test -- --ignored
    fn test_health_endpoint() {
        let _ = start_server();

        let output = Command::new("curl")
            .args(&["-s", "http://localhost:8080/health"])
            .output();

        match output {
            Ok(output) => {
                assert!(output.status.success(), "curl command should succeed");
                let body = String::from_utf8_lossy(&output.stdout);
                assert!(
                    body.contains("healthy") || body.contains("status"),
                    "Health check should return status"
                );
            }
            Err(e) => {
                panic!("Failed to execute curl: {}", e);
            }
        }
    }

    #[test]
    #[ignore] // Run this manually
    fn test_metrics_endpoint() {
        let _ = start_server();

        let output = Command::new("curl")
            .args(&["-s", "http://localhost:8080/api/v1/metrics"])
            .output();

        match output {
            Ok(output) => {
                assert!(output.status.success(), "curl command should succeed");
                let body = String::from_utf8_lossy(&output.stdout);
                assert!(
                    body.contains("storage") || body.contains("{}"),
                    "Metrics should return data"
                );
            }
            Err(e) => {
                panic!("Failed to execute curl: {}", e);
            }
        }
    }

    #[test]
    fn test_json_config_validity() {
        // Test that the example config is valid JSON
        let output = Command::new("cat")
            .arg("examples/storage.example.json")
            .output();

        if let Ok(output) = output {
            let content = String::from_utf8_lossy(&output.stdout);

            // Try to parse as JSON
            let parsed = serde_json::from_str::<serde_json::Value>(&content);
            assert!(parsed.is_ok(), "Example config should be valid JSON");

            let json = parsed.unwrap();
            assert!(json.is_object(), "Config root should be an object");
        }
    }
}
