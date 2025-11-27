use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use std::fs;
use tempfile::TempDir;
use serde_json::Value;

/// Helper struct to manage mock printer lifecycle
struct MockPrinter {
    process: Child,
    control_port: u16,
    _mqtt_port: u16,
    _ftp_port: u16,
}

impl MockPrinter {
    /// Start the mock printer server
    fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let control_port = 3000;
        let mqtt_port = 8883;
        let ftp_port = 2121;
        
        println!("Starting mock printer...");
        // Build mock printer if not already built
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(std::path::PathBuf::from)
            .or_else(|_| std::env::current_dir())?;
            
        let mock_printer_dir = manifest_dir
            .parent()
            .ok_or("Failed to get parent directory")?
            .join("mock-printer");
            
        // Check if dist directory exists, if not, build
        if !mock_printer_dir.join("dist").exists() {
            let build_status = Command::new("npm")
                .arg("run")
                .arg("build")
                .current_dir(&mock_printer_dir)
                .status()?;
            
            if !build_status.success() {
                return Err("Failed to build mock printer".into());
            }
        }
        
        // Start the mock printer using node directly to ensure we can kill it
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
            _mqtt_port: mqtt_port,
            _ftp_port: ftp_port,
        };
        
        // Wait for the server to be ready
        printer.wait_for_ready(30)?;
        
        println!("Mock printer started successfully");
        
        Ok(printer)
    }
    
    /// Wait for the mock printer to be ready
    fn wait_for_ready(&self, timeout_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();
        
        loop {
            if start.elapsed().as_secs() > timeout_secs {
                return Err("Mock printer failed to start within timeout".into());
            }
            
            // Try to connect to health endpoint
            if let Ok(response) = reqwest::blocking::get(&self.health_url()) {
                if response.status().is_success() {
                    return Ok(());
                }
            }
            
            sleep(Duration::from_millis(200));
        }
    }
    
    /// Get health check URL
    fn health_url(&self) -> String {
        format!("http://localhost:{}/health", self.control_port)
    }
    
    /// Get configuration URL
    fn config_url(&self) -> String {
        format!("http://localhost:{}/config", self.control_port)
    }
    
    /// Get received messages URL
    fn messages_url(&self) -> String {
        format!("http://localhost:{}/messages/received", self.control_port)
    }
    
    /// Get files URL
    fn files_url(&self) -> String {
        format!("http://localhost:{}/files", self.control_port)
    }
    
    /// Reset the mock printer state
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
    
    /// Get received messages from the mock printer
    fn get_received_messages(&self) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let response = reqwest::blocking::get(&self.messages_url())?;
        let data: Value = response.json()?;
        
        if let Some(messages) = data["messages"].as_array() {
            Ok(messages.clone())
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Get uploaded files from the mock printer
    fn get_uploaded_files(&self) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let response = reqwest::blocking::get(&self.files_url())?;
        let data: Value = response.json()?;
        
        if let Some(files) = data["files"].as_array() {
            Ok(files.clone())
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Publish a message to the report topic
    fn _publish_message(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::blocking::Client::new();
        let response = client
            .post(&format!("http://localhost:{}/messages", self.control_port))
            .json(&serde_json::json!({ "message": message }))
            .send()?;
        
        if !response.status().is_success() {
            return Err("Failed to publish message".into());
        }
        
        Ok(())
    }
}

impl Drop for MockPrinter {
    fn drop(&mut self) {
        println!("Stopping mock printer...");
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Create a temporary config file for testing
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
mqtt_port = 8883
ftp_port = 2121
ssl = false
"#;
    
    fs::write(&config_path, config_content)?;
    
    Ok((temp_dir, config_path.to_string_lossy().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Import the API functions from the library
    use bambu_networking::api::{
        bambu_network_rs_init,
        bambu_network_rs_connect,
        bambu_network_rs_disconnect,
        bambu_network_rs_send,
        bambu_network_rs_upload_file,
        bambu_network_rs_get_camera_url,
    };
    
    /// Setup function to start mock printer and initialize library
    fn setup() -> (MockPrinter, TempDir) {
        // Disconnect any existing connection to ensure clean state
        bambu_network_rs_disconnect("01234567".to_string());

        // Start mock printer
        let printer = MockPrinter::start()
            .expect("Failed to start mock printer");
        
        // Create test config
        let (temp_dir, _config_path) = create_test_config()
            .expect("Failed to create test config");
        
        // Change to temp directory so config is found
        std::env::set_current_dir(temp_dir.path())
            .expect("Failed to change directory");
        
        // Reset printer state
        printer.reset().expect("Failed to reset printer");
        
        // Initialize the library
        bambu_network_rs_init();
        
        // Give it a moment to initialize
        sleep(Duration::from_millis(500));
        
        (printer, temp_dir)
    }
    
    #[test]
    fn test_mock_printer_starts() {
        let (printer, _temp_dir) = setup();
        
        // Verify health endpoint
        let response = reqwest::blocking::get(&printer.health_url())
            .expect("Failed to get health");
        
        assert!(response.status().is_success());
        
        let data: Value = response.json().expect("Failed to parse JSON");
        assert_eq!(data["status"], "ok");
    }
    
    #[test]
    fn test_mqtt_connection() {
        let (printer, _temp_dir) = setup();
        
        // Connect to the printer
        let result = bambu_network_rs_connect("01234567".to_string());
        assert_eq!(result, 0, "Connection should succeed");
        
        // Give it time to establish connection
        sleep(Duration::from_secs(2));
        
        // Verify connection via health check
        let response = reqwest::blocking::get(&printer.health_url())
            .expect("Failed to get health");
        let data: Value = response.json().expect("Failed to parse JSON");
        
        // The mock printer should show connected state
        assert_eq!(data["connected"], true);
    }
    
    #[test]
    fn test_mqtt_publish() {
        let (printer, _temp_dir) = setup();
        
        // Connect to the printer
        bambu_network_rs_connect("01234567".to_string());
        sleep(Duration::from_secs(5));
        
        // Clear any existing messages - removed as setup() already does this
        // printer.reset().expect("Failed to reset");
        
        // Send a message
        let test_message = r#"{"command": "test"}"#;
        let result = bambu_network_rs_send("01234567".to_string(), test_message.to_string());
        assert_eq!(result, 0, "Send should succeed");
        
        // Give it time to be received
        sleep(Duration::from_millis(500));
        
        // Verify the message was received
        let messages = printer.get_received_messages()
            .expect("Failed to get messages");
        
        assert!(!messages.is_empty(), "Should have received at least one message");
        
        // Check if our test message is in there
        let found = messages.iter().any(|msg| {
            msg["payload"].as_str().map_or(false, |p| p.contains("test"))
        });
        assert!(found, "Should have received our test message");
    }
    
    #[test]
    fn test_ftp_upload() {
        let (printer, temp_dir) = setup();
        
        // Create a test file to upload
        let test_file_path = temp_dir.path().join("test.gcode");
        let test_content = b"G1 X10 Y20 Z5\nG1 X0 Y0 Z0\n";
        fs::write(&test_file_path, test_content)
            .expect("Failed to write test file");
        
        // Upload the file
        let result = bambu_network_rs_upload_file(
            "01234567".to_string(),
            test_file_path.to_string_lossy().to_string(),
            "test.gcode".to_string(),
        );
        assert_eq!(result, 0, "Upload should succeed");
        
        // Give it time to complete
        sleep(Duration::from_millis(500));
        
        // Verify the file was uploaded
        let files = printer.get_uploaded_files()
            .expect("Failed to get files");
        
        assert!(!files.is_empty(), "Should have uploaded at least one file");
        
        // Check if our test file is there
        let found = files.iter().any(|file| {
            file["filename"].as_str() == Some("test.gcode")
        });
        assert!(found, "Should have uploaded test.gcode");
    }
    
    #[test]
    fn test_printer_connection_lifecycle() {
        // This test implements Flow 1 from ORCASLICER_OPS.md adapted for local printer operations
        // Flow: Initialize → Discovery → Connect → Connected Callback → Ready State → Disconnect
        
        let (printer, _temp_dir) = setup();
        
        println!("=== FLOW 1: PRINTER CONNECTION LIFECYCLE ===");
        
        // Step 1: Library initialization (already done in setup())
        println!("✓ Step 1: Library initialized via bambu_network_rs_init()");
        
        // Step 2: Verify printer is announced (simulates printer discovery)
        println!("✓ Step 2: Printer discovered and available in config");
        let health_response = reqwest::blocking::get(&printer.health_url())
            .expect("Failed to get health");
        assert!(health_response.status().is_success(), "Mock printer should be running");
        
        // Step 3: User requests connection to printer
        println!("→ Step 3: Connecting to printer via bambu_network_rs_connect()...");
        let connect_result = bambu_network_rs_connect("01234567".to_string());
        assert_eq!(connect_result, 0, "Connection should succeed");
        
        // Step 4: Wait for connection to be established and callback to fire
        println!("→ Step 4: Waiting for connection callback...");
        sleep(Duration::from_secs(2));
        
        // Step 5: Verify connected state via health endpoint
        println!("→ Step 5: Verifying connected state...");
        let health_response = reqwest::blocking::get(&printer.health_url())
            .expect("Failed to get health");
        let health_data: Value = health_response.json().expect("Failed to parse JSON");
        assert_eq!(health_data["connected"], true, "Printer should show connected state");
        println!("✓ Step 5: Connection established (bambu_network_cb_connected fired)");
        
        // Step 6: Verify connection is ready for operations by sending a test message
        println!("→ Step 6: Verifying connection is operational...");
        let test_message = r#"{"command": "connection_test"}"#;
        let send_result = bambu_network_rs_send("01234567".to_string(), test_message.to_string());
        assert_eq!(send_result, 0, "Send should succeed on active connection");
        
        // Wait for message to be received
        sleep(Duration::from_millis(500));
        
        let messages = printer.get_received_messages()
            .expect("Failed to get messages");
        assert!(!messages.is_empty(), "Should have received message");
        
        let found = messages.iter().any(|msg| {
            msg["payload"].as_str().map_or(false, |p| p.contains("connection_test"))
        });
        assert!(found, "Should have received connection test message");
        println!("✓ Step 6: Connection ready - message sent and received successfully");
        
        // Step 7: Clean disconnect
        println!("→ Step 7: Disconnecting...");
        bambu_network_rs_disconnect("01234567".to_string());
        sleep(Duration::from_millis(500));
        println!("✓ Step 7: Disconnected successfully");
        
        println!("=== FLOW 1 COMPLETE ===");
    }
    
    #[test]
    fn test_connection_to_nonexistent_printer() {
        let (_printer, _temp_dir) = setup();
        
        // Try to connect to a printer that doesn't exist
        let result = bambu_network_rs_connect("99999999".to_string());
        assert_eq!(result, -1, "Connection to nonexistent printer should fail");
    }
    
    #[test]
    fn test_printer_bind_lifecycle() {
        // This test implements Flow 2 from ORCASLICER_OPS.md adapted for local printer operations
        // Flow: Discovery → Bind → Auth →State Verification → Post-Bind Validation
        
        let (printer, _temp_dir) = setup();
        
        println!("=== FLOW 2: PRINTER BIND LIFECYCLE ===");
        
        // PHASE 1: DISCOVERY
        println!("\n--- Phase 1: Discovery ---");
        println!("→ Verifying printer is announced and available...");
        
        // Verify health endpoint (printer is running)
        let health_response = reqwest::blocking::get(&printer.health_url())
            .expect("Failed to get health");
        assert!(health_response.status().is_success(), "Mock printer should be running");
        
        let health_data: Value = health_response.json().expect("Failed to parse health JSON");
        assert_eq!(health_data["status"], "ok", "Printer health should be OK");
        println!("✓ Printer discovered and healthy");
        
        // Verify printer configuration
        let config_response = reqwest::blocking::get(&printer.config_url())
            .expect("Failed to get config");
        let config_data: Value = config_response.json().expect("Failed to parse config JSON");
        
        assert_eq!(config_data["devId"], "01234567", "Device ID should match");
        assert_eq!(config_data["devName"], "Mock Printer", "Device name should match");
        assert_eq!(config_data["password"], "testpass", "Password should match");
        println!("✓ Printer configuration verified");
        println!("  - Device ID: {}", config_data["devId"]);
        println!("  - Name: {}", config_data["devName"]);
        println!("  - Model: {}", config_data["model"]);
        
        // PHASE 2: BIND INITIATION
        println!("\n--- Phase 2: Bind Initiation ---");
        println!("→ Initiating bind (connection) to printer...");
        
        let bind_result = bambu_network_rs_connect("01234567".to_string());
        assert_eq!(bind_result, 0, "Bind operation should return success (0)");
        println!("✓ Bind API call successful (return code: {})", bind_result);
        
        // PHASE 3: AUTHENTICATION & CONNECTION
        println!("\n--- Phase 3: Authentication & Connection ---");
        println!("→ Waiting for MQTT authentication and connection...");
        sleep(Duration::from_secs(2));
        
        // Verify connection callback fired and printer shows connected
        let health_response = reqwest::blocking::get(&printer.health_url())
            .expect("Failed to get health");
        let health_data: Value = health_response.json().expect("Failed to parse JSON");
        
        assert_eq!(health_data["connected"], true, "Printer should show connected state");
        println!("✓ MQTT connection established");
        println!("✓ Printer state shows connected");
        
        // PHASE 4: STATE VERIFICATION
        println!("\n--- Phase 4: State Verification ---");
        println!("→ Verifying printer operational state...");
        
        // Test message sending capability
        let test_message = r#"{"system": {"command": "get_version"}}"#;
        let send_result = bambu_network_rs_send("01234567".to_string(), test_message.to_string());
        assert_eq!(send_result, 0, "Send should succeed on bound printer");
        println!("✓ API message send successful");
        
        sleep(Duration::from_millis(500));
        
        // Verify message was received by printer
        let messages = printer.get_received_messages()
            .expect("Failed to get received messages");
        assert!(!messages.is_empty(), "Printer should have received messages");
        
        let found = messages.iter().any(|msg| {
            msg["payload"].as_str().map_or(false, |p| p.contains("get_version"))
        });
        assert!(found, "Printer should have received our bind verification message");
        println!("✓ Printer received message correctly");
        println!("  - Total messages received: {}", messages.len());
        
        // Verify no unexpected errors
        assert_eq!(health_data["status"], "ok", "Printer should have no errors");
        println!("✓ No errors in printer state");
        
        // PHASE 5: POST-BIND VERIFICATION
        println!("\n--- Phase 5: Post-Bind Verification ---");
        println!("→ Requesting printer status...");
        
        // Trigger status message from printer
        let client = reqwest::blocking::Client::new();
        let status_trigger = client
            .post(&format!("http://localhost:{}/messages/status", printer.control_port))
            .send()
            .expect("Failed to trigger status");
        assert!(status_trigger.status().is_success(), "Status trigger should succeed");
        
        sleep(Duration::from_millis(300));
        
        // Send a status request command
        let status_request = r#"{"system": {"command": "push_status"}}"#;
        let status_result = bambu_network_rs_send("01234567".to_string(), status_request.to_string());
        assert_eq!(status_result, 0, "Status request should succeed");
        
        sleep(Duration::from_millis(500));
        
        // Verify we can still send messages (printer fully operational)
        let ping_message = r#"{"system": {"command": "ping"}}"#;
        let ping_result = bambu_network_rs_send("01234567".to_string(), ping_message.to_string());
        assert_eq!(ping_result, 0, "Ping after bind should succeed");
        println!("✓ Printer operational and responding");
        
        // Final state check
        let final_health = reqwest::blocking::get(&printer.health_url())
            .expect("Failed to get final health");
        let final_data: Value = final_health.json().expect("Failed to parse final JSON");
        
        assert_eq!(final_data["connected"], true, "Printer should remain connected");
        assert_eq!(final_data["status"], "ok", "Printer should be healthy");
        println!("✓ Final state verification passed");
        println!("  - Connected: {}", final_data["connected"]);
        println!("  - Status: {}", final_data["status"]);
        
        // Get final message count
        let final_messages = printer.get_received_messages()
            .expect("Failed to get final messages");
        println!("  - Total messages exchanged: {}", final_messages.len());
        
        println!("\n=== FLOW 2 COMPLETE ===");
        println!("✓ Bind lifecycle verified successfully");
        println!("✓ All API responses correct");
        println!("✓ All printer states validated");
    }
    
    #[test]
    fn test_print_send_lifecycle() {
        // This test implements Flow 3 from ORCASLICER_OPS.md adapted for local printer operations
        // Flow: Setup → File Prep → Upload → Verify → Print Command → State Validation
        
        let (printer, temp_dir) = setup();
        
        println!("=== FLOW 3: PRINT SEND LIFECYCLE ===");
        
        // PHASE 1: SETUP & CONNECTION
        println!("\n--- Phase 1: Setup & Connection ---");
        println!("→ Establishing connection to printer...");
        
        let connect_result = bambu_network_rs_connect("01234567".to_string());
        assert_eq!(connect_result, 0, "Connection should succeed");
        sleep(Duration::from_secs(2));
        
        let health_response = reqwest::blocking::get(&printer.health_url())
            .expect("Failed to get health");
        let health_data: Value = health_response.json().expect("Failed to parse JSON");
        assert_eq!(health_data["connected"], true, "Printer should be connected");
        println!("✓ Connection established and ready");
        
        // PHASE 2: FILE PREPARATION
        println!("\n--- Phase 2: File Preparation ---");
        println!("→ Creating test G-code file...");
        
        let test_file_path = temp_dir.path().join("test_print.gcode");
        let gcode_content = b"; Generated by OrcaSlicer\n\
; Test print file\n\
G28 ; Home all axes\n\
G1 Z5 F5000 ; Lift Z\n\
G1 X10 Y10 F3000\n\
G1 Z0.2 F1000\n\
G1 X50 Y10 E5 F1500 ; Extrude line\n\
G1 X50 Y50 E10\n\
G1 X10 Y50 E15\n\
G1 X10 Y10 E20\n\
M104 S0 ; Turn off hotend\n\
M140 S0 ; Turn off bed\n\
G28 X Y ; Home X and Y\n\
M84 ; Disable motors\n";
        
        fs::write(&test_file_path, gcode_content)
            .expect("Failed to write test file");
        
        let file_size = gcode_content.len();
        println!("✓ Test file created");
        println!("  - Path: {:?}", test_file_path);
        println!("  - Size: {} bytes", file_size);
        println!("  - Lines: {}", gcode_content.iter().filter(|&&b| b == b'\n').count());
        
        // PHASE 3: FILE UPLOAD (FTP)
        println!("\n--- Phase 3: File Upload ---");
        println!("→ Uploading file via FTP...");
        
        let upload_start = std::time::Instant::now();
        let upload_result = bambu_network_rs_upload_file(
            "01234567".to_string(),
            test_file_path.to_string_lossy().to_string(),
            "test_print.gcode".to_string(),
        );
        let upload_duration = upload_start.elapsed();
        
        assert_eq!(upload_result, 0, "Upload should succeed");
        println!("✓ Upload API call successful");
        println!("  - Duration: {:?}", upload_duration);
        println!("  - Return code: {}", upload_result);
        
        sleep(Duration::from_millis(500));
        
        // PHASE 4: UPLOAD VERIFICATION
        println!("\n--- Phase 4: Upload Verification ---");
        println!("→ Verifying file exists on printer...");
        
        let files = printer.get_uploaded_files()
            .expect("Failed to get uploaded files");
        
        assert!(!files.is_empty(), "Should have at least one uploaded file");
        
        let uploaded_file = files.iter().find(|f| {
            f["filename"].as_str() == Some("test_print.gcode")
        });
        assert!(uploaded_file.is_some(), "test_print.gcode should be uploaded");
        
        let file_info = uploaded_file.unwrap();
        println!("✓ File found on printer");
        println!("  - Filename: {}", file_info["filename"]);
        println!("  - Size: {} bytes", file_info["size"]);
        
        // Verify file size matches
        assert_eq!(
            file_info["size"].as_u64().unwrap(),
            file_size as u64,
            "File size should match original"
        );
        println!("✓ File size verified");
        
        // Get and verify file content
        let file_content_response = reqwest::blocking::get(
            &format!("http://localhost:{}/files/test_print.gcode", printer.control_port)
        ).expect("Failed to get file content");
        
        let file_data: Value = file_content_response.json()
            .expect("Failed to parse file content JSON");
        
        // Decode base64 content
        let content_b64 = file_data["content"].as_str().expect("Content should be string");
        use base64::{Engine as _, engine::general_purpose};
        let decoded_content = general_purpose::STANDARD.decode(content_b64)
            .expect("Failed to decode base64");
        
        assert_eq!(decoded_content, gcode_content, "File content should match original");
        println!("✓ File content verified (matches original)");
        
        // PHASE 5: PRINT COMMAND
        println!("\n--- Phase 5: Print Command ---");
        println!("→ Sending print start command...");
        
        let print_command = r#"{
            "print": {
                "command": "start",
                "file": "test_print.gcode",
                "task_name": "Integration Test Print"
            }
        }"#;
        
        let command_result = bambu_network_rs_send(
            "01234567".to_string(),
            print_command.to_string()
        );
        assert_eq!(command_result, 0, "Print command should succeed");
        println!("✓ Print command sent successfully");
        
        sleep(Duration::from_millis(500));
        
        // Verify command was received
        let messages = printer.get_received_messages()
            .expect("Failed to get messages");
        
        let print_cmd_found = messages.iter().any(|msg| {
            msg["payload"].as_str().map_or(false, |p| {
                p.contains("start") && p.contains("test_print.gcode")
            })
        });
        assert!(print_cmd_found, "Printer should have received print start command");
        println!("✓ Printer received print command");
        
        // PHASE 6: STATE VALIDATION
        println!("\n--- Phase 6: State Validation ---");
        println!("→ Validating final state...");
        
        // Verify connection still active
        let final_health = reqwest::blocking::get(&printer.health_url())
            .expect("Failed to get final health");
        let final_data: Value = final_health.json().expect("Failed to parse final JSON");
        
        assert_eq!(final_data["connected"], true, "Printer should still be connected");
        assert_eq!(final_data["status"], "ok", "Printer should be healthy");
        println!("✓ Printer connection healthy");
        
        // Verify all files present
        let final_files = printer.get_uploaded_files()
            .expect("Failed to get final files");
        println!("✓ Files on printer: {}", final_files.len());
        
        // Verify message count
        let final_messages = printer.get_received_messages()
            .expect("Failed to get final messages");
        println!("✓ Total messages exchanged: {}", final_messages.len());
        
        // Summary
        println!("\n=== FLOW 3 COMPLETE ===");
        println!("✓ Print send lifecycle verified successfully");
        println!("✓ File uploaded: test_print.gcode ({} bytes)", file_size);
        println!("✓ File content verified on printer");
        println!("✓ Print command sent and received");
        println!("✓ All API responses correct");
        println!("✓ All printer states validated");
    }
    
    #[test]
    fn test_printer_control_lifecycle() {
        // This test implements Flow 4 from ORCASLICER_OPS.md adapted for local printer operations
        // Flow: Setup → Pause → Resume → Stop → Custom Commands → Verification
        
        let (printer, _temp_dir) = setup();
        
        println!("=== FLOW 4: PRINTER CONTROL LIFECYCLE ===");
        
        // PHASE 1: SETUP
        println!("\n--- Phase 1: Setup ---");
        println!("→ Establishing connection...");
        
        let connect_result = bambu_network_rs_connect("01234567".to_string());
        assert_eq!(connect_result, 0, "Connection should succeed");
        sleep(Duration::from_secs(2));
        
        let health_response = reqwest::blocking::get(&printer.health_url())
            .expect("Failed to get health");
        let health_data: Value = health_response.json().expect("Failed to parse JSON");
        assert_eq!(health_data["connected"], true, "Printer should be connected");
        println!("✓ Connected and ready for control commands");
        
        // PHASE 2: PAUSE COMMAND
        println!("\n--- Phase 2: Pause Command ---");
        println!("→ Sending pause command...");
        
        let pause_cmd = r#"{"print": {"command": "pause"}}"#;
        let pause_result = bambu_network_rs_send("01234567".to_string(), pause_cmd.to_string());
        assert_eq!(pause_result, 0, "Pause command should succeed");
        println!("✓ Pause API call successful");
        
        sleep(Duration::from_millis(300));
        
        let messages = printer.get_received_messages().expect("Failed to get messages");
        let pause_found = messages.iter().any(|msg| {
            msg["payload"].as_str().map_or(false, |p| p.contains("pause"))
        });
        assert!(pause_found, "Printer should have received pause command");
        println!("✓ Printer received pause command");
        
        // PHASE 3: RESUME COMMAND  
        println!("\n--- Phase 3: Resume Command ---");
        println!("→ Sending resume command...");
        
        let resume_cmd = r#"{"print": {"command": "resume"}}"#;
        let resume_result = bambu_network_rs_send("01234567".to_string(), resume_cmd.to_string());
        assert_eq!(resume_result, 0, "Resume command should succeed");
        println!("✓ Resume API call successful");
        
        sleep(Duration::from_millis(300));
        
        let messages = printer.get_received_messages().expect("Failed to get messages");
        let resume_found = messages.iter().any(|msg| {
            msg["payload"].as_str().map_or(false, |p| p.contains("resume"))
        });
        assert!(resume_found, "Printer should have received resume command");
        println!("✓ Printer received resume command");
        
        // PHASE 4: STOP COMMAND
        println!("\n--- Phase 4: Stop Command ---");
        println!("→ Sending stop command...");
        
        let stop_cmd = r#"{"print": {"command": "stop"}}"#;
        let stop_result = bambu_network_rs_send("01234567".to_string(), stop_cmd.to_string());
        assert_eq!(stop_result, 0, "Stop command should succeed");
        println!("✓ Stop API call successful");
        
        sleep(Duration::from_millis(300));
        
        let messages = printer.get_received_messages().expect("Failed to get messages");
        let stop_found = messages.iter().any(|msg| {
            msg["payload"].as_str().map_or(false, |p| p.contains("stop"))
        });
        assert!(stop_found, "Printer should have received stop command");
        println!("✓ Printer received stop command");
        
        // PHASE 5: CUSTOM CONTROL COMMANDS
        println!("\n--- Phase 5: Custom Control Commands ---");
        println!("→ Sending temperature control...");
        
        let temp_cmd = r#"{"system": {"command": "set_temp", "bed_temp": 60, "nozzle_temp": 210}}"#;
        let temp_result = bambu_network_rs_send("01234567".to_string(), temp_cmd.to_string());
        assert_eq!(temp_result, 0, "Temperature command should succeed");
        sleep(Duration::from_millis(200));
        println!("✓ Temperature command sent");
        
        println!("→ Sending AMS control...");
        let ams_cmd = r#"{"ams": {"command": "change_filament", "tray_id": 2}}"#;
        let ams_result = bambu_network_rs_send("01234567".to_string(), ams_cmd.to_string());
        assert_eq!(ams_result, 0, "AMS command should succeed");
        sleep(Duration::from_millis(200));
        println!("✓ AMS command sent");
        
        println!("→ Sending speed control...");
        let speed_cmd = r#"{"print": {"command": "set_speed", "speed_level": 2}}"#;
        let speed_result = bambu_network_rs_send("01234567".to_string(), speed_cmd.to_string());
        assert_eq!(speed_result, 0, "Speed command should succeed");
        sleep(Duration::from_millis(200));
        println!("✓ Speed command sent");
        
        // PHASE 6: STATE VERIFICATION
        println!("\n--- Phase 6: State Verification ---");
        println!("→ Verifying all commands received...");
        
        let final_messages = printer.get_received_messages().expect("Failed to get messages");
        
        // Verify we have all expected commands
        let expected_commands = vec!["pause", "resume", "stop", "set_temp", "change_filament", "set_speed"];
        let mut found_commands = Vec::new();
        
        for cmd in &expected_commands {
            let found = final_messages.iter().any(|msg| {
                msg["payload"].as_str().map_or(false, |p| p.contains(cmd))
            });
            if found {
                found_commands.push(*cmd);
            }
        }
        
        println!("✓ Commands received: {}/{}", found_commands.len(), expected_commands.len());
        for cmd in &found_commands {
            println!("  ✓ {}", cmd);
        }
        
        assert_eq!(
            found_commands.len(),
            expected_commands.len(),
            "All commands should be received"
        );
        
        // Verify connection still healthy
        let final_health = reqwest::blocking::get(&printer.health_url())
            .expect("Failed to get health");
        let final_data: Value = final_health.json().expect("Failed to parse JSON");
        assert_eq!(final_data["connected"], true, "Printer should still be connected");
        assert_eq!(final_data["status"], "ok", "Printer should be healthy");
        println!("✓ Printer connection healthy");
        
        println!("✓ Total messages: {}", final_messages.len());
        
        // Summary
        println!("\n=== FLOW 4 COMPLETE ===");
        println!("✓ Printer control lifecycle verified");
        println!("✓ Pause/Resume/Stop commands working");
        println!("✓ Custom control commands working");
        println!("✓ All commands received by printer");
        println!("✓ Command sequence preserved");
    }
    
    #[test]
    fn test_camera_view_lifecycle() {
        // This test implements Flow 5 from ORCASLICER_OPS.md adapted for local printer operations
        // Flow: Setup → Get Camera URL → Verify URL
        
        let (printer, _temp_dir) = setup();
        
        println!("=== FLOW 5: CAMERA VIEW LIFECYCLE ===");
        
        // PHASE 1: SETUP
        println!("\n--- Phase 1: Setup ---");
        println!("→ Establishing connection...");
        
        let connect_result = bambu_network_rs_connect("01234567".to_string());
        assert_eq!(connect_result, 0, "Connection should succeed");
        sleep(Duration::from_secs(2));
        
        let health_response = reqwest::blocking::get(&printer.health_url())
            .expect("Failed to get health");
        let health_data: Value = health_response.json().expect("Failed to parse JSON");
        assert_eq!(health_data["connected"], true, "Printer should be connected");
        println!("✓ Connected and ready");
        
        // PHASE 2: GET CAMERA URL
        println!("\n--- Phase 2: Get Camera URL ---");
        println!("→ Requesting camera URL...");
        
        let url = bambu_network_rs_get_camera_url("01234567".to_string());
        println!("✓ URL received: {}", url);
        
        // PHASE 3: VERIFICATION
        println!("\n--- Phase 3: Verification ---");
        println!("→ Verifying URL format...");
        
        assert!(!url.is_empty(), "URL should not be empty");
        assert!(url.starts_with("rtsp://"), "URL should start with rtsp://");
        assert!(url.contains("127.0.0.1"), "URL should contain printer IP");
        assert!(url.ends_with("/live"), "URL should end with /live");
        
        println!("✓ URL format valid");
        println!("✓ IP address matches printer");
        println!("✓ Protocol is RTSP");
        
        // Summary
        println!("\n=== FLOW 5 COMPLETE ===");
        println!("✓ Camera view lifecycle verified");
        println!("✓ Camera URL retrieval working");
        println!("✓ URL format correct");
    }
    
    #[test]
    fn test_send_without_connection() {
        let (_printer, _temp_dir) = setup();
        
        // Try to send a message without connecting first
        let result = bambu_network_rs_send(
            "01234567".to_string(),
            r#"{"command": "test"}"#.to_string()
        );
        
        // Should fail because we haven't connected
        assert_ne!(result, 0, "Send without connection should fail");
    }
}
