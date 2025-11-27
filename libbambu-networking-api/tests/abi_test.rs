use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use std::fs;
use tempfile::TempDir;
use serde_json::Value;

use bambu_networking::ffi_wrapper;

/// Helper struct to manage mock printer lifecycle
struct MockPrinter {
    process: Child,
    control_port: u16,
}

impl MockPrinter {
    fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let control_port = 3001; // Use different port than integration_test.rs
        let mqtt_port = 8884;
        let ftp_port = 2122;
        
        println!("Starting mock printer for ABI test...");
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(std::path::PathBuf::from)
            .or_else(|_| std::env::current_dir())?;
            
        let mock_printer_dir = manifest_dir
            .parent()
            .ok_or("Failed to get parent directory")?
            .join("mock-printer");
            
        let process = Command::new("node")
            .arg("dist/index.js")
            .current_dir(&mock_printer_dir)
            .env("DEV_ID", "01234567")
            .env("PASSWORD", "testpass")
            .env("MQTT_PORT", mqtt_port.to_string())
            .env("FTP_PORT", ftp_port.to_string())
            .env("CONTROL_PORT", control_port.to_string())
            .env("USE_TLS", "false")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        let printer = MockPrinter {
            process,
            control_port,
        };
        
        printer.wait_for_ready(30)?;
        Ok(printer)
    }
    
    fn wait_for_ready(&self, timeout_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed().as_secs() > timeout_secs {
                return Err("Mock printer failed to start within timeout".into());
            }
            if let Ok(response) = reqwest::blocking::get(&self.health_url()) {
                if response.status().is_success() {
                    return Ok(());
                }
            }
            sleep(Duration::from_millis(200));
        }
    }
    
    fn health_url(&self) -> String {
        format!("http://localhost:{}/health", self.control_port)
    }
    
    fn reset(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::blocking::Client::new();
        let response = client
            .post(&format!("http://localhost:{}/reset", self.control_port))
            .send()?;
        if !response.status().is_success() {
            return Err("Failed to reset mock printer".into());
        }
        Ok(())
    }
    
    fn get_received_messages(&self) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let response = reqwest::blocking::get(&format!("http://localhost:{}/messages/received", self.control_port))?;
        let data: Value = response.json()?;
        if let Some(messages) = data["messages"].as_array() {
            Ok(messages.clone())
        } else {
            Ok(Vec::new())
        }
    }
}

impl Drop for MockPrinter {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

fn create_test_config() -> Result<(TempDir, String), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let config_path = temp_dir.path().join("openbambu.toml");
    let config_content = r#"
[[printers]]
dev_id = "01234567"
name = "Mock Printer"
model = "x1c"
host = "127.0.0.1"
password = "testpass"
mqtt_port = 8884
ftp_port = 2122
ssl = false
"#;
    fs::write(&config_path, config_content)?;
    Ok((temp_dir, config_path.to_string_lossy().to_string()))
}

#[test]
fn test_abi_flow() {
    // 1. Setup
    let printer = MockPrinter::start().expect("Failed to start mock printer");
    let (temp_dir, _config_path) = create_test_config().expect("Failed to create config");
    std::env::set_current_dir(temp_dir.path()).expect("Failed to change dir");
    printer.reset().expect("Failed to reset printer");

    // 2. Initialize via Shim -> C++ API -> Rust Impl
    ffi_wrapper::init();
    sleep(Duration::from_millis(500));

    // 3. Connect via Shim -> C++ API -> Rust Impl
    let result = ffi_wrapper::connect("01234567", "127.0.0.1", "testpass", false)
        .expect("Connect failed");
    assert_eq!(result, 0, "Connection should succeed");
    sleep(Duration::from_secs(2));

    // 4. Verify connection
    let response = reqwest::blocking::get(&printer.health_url()).expect("Failed to get health");
    let data: Value = response.json().expect("Failed to parse JSON");
    assert_eq!(data["connected"], true, "Printer should be connected");

    // 5. Send message via Shim -> C++ API -> Rust Impl
    let msg = r#"{"command":"abi_test"}"#;
    let result = ffi_wrapper::send("01234567", msg).expect("Send failed");
    assert_eq!(result, 0, "Send should succeed");
    sleep(Duration::from_millis(500));

    // 6. Verify message received
    let messages = printer.get_received_messages().expect("Failed to get messages");
    let found = messages.iter().any(|m| m["payload"].as_str().map_or(false, |p| p.contains("abi_test")));
    assert!(found, "Should have received abi_test message");

    // 7. Disconnect via Shim -> C++ API -> Rust Impl
    let result = ffi_wrapper::disconnect("01234567").expect("Disconnect failed");
    assert_eq!(result, 0, "Disconnect should succeed");
}
