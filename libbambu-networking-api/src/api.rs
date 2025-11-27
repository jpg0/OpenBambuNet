use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::Mutex;
use std::time::Duration;

use cxx::let_cxx_string;
use lazy_static::lazy_static;
use log::{debug, warn, error};
use tokio::time::sleep;
use futures::StreamExt;

use open_bambu_core::{Printer, MqttClient, FtpClient};
use open_bambu_core::printer::load_printers;

use crate::errors::{BAMBU_NETWORK_ERR_SEND_MSG_FAILED, BAMBU_NETWORK_SUCCESS};




#[cxx::bridge]
mod ffi {

    extern "Rust" {
        pub fn bambu_network_rs_init();
        pub fn bambu_network_rs_log_debug(message: String);
        pub fn bambu_network_rs_connect(device_id: String) -> i32;
        pub fn bambu_network_rs_disconnect(device_id: String) -> i32;
        pub fn bambu_network_rs_send(device_id: String, data: String) -> i32;
        pub fn bambu_network_rs_upload_file(
            device_id: String,
            local_filename: String,
            remote_filename: String,
        ) -> i32;
        pub fn bambu_network_rs_get_camera_url(device_id: String) -> String;
    }


    unsafe extern "C++" {
        include!("api.hpp");

        pub fn bambu_network_cb_printer_available(json: &CxxString);
        pub fn bambu_network_cb_message_recv(device_id: &CxxString, json: &CxxString);
        pub fn bambu_network_cb_connected(device_id: &CxxString);
    }
}

lazy_static! {
    static ref RUNTIME: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(6)
        .enable_all()
        .build()
        .unwrap();
        
    static ref PRINTERS: Mutex<Vec<Printer>> = Mutex::new(Vec::new());
    static ref CONNECTIONS: Mutex<HashMap<String, MqttClient>> = Mutex::new(HashMap::new());
}

pub fn bambu_network_rs_init() {
    let _ = env_logger::try_init();
    debug!("Initializing bambu network");
    
    // Load printers
    let printers = load_printers();
    *PRINTERS.lock().unwrap() = printers.clone();
    
    RUNTIME.spawn(async move {
        loop {
            // Announce printers
             let printers = PRINTERS.lock().unwrap().clone();
             for printer in printers {
                let_cxx_string!(
                    json = format!(
                        "{}
                    \"dev_name\": \"{}\",
                    \"dev_id\": \"{}\",
                    \"dev_ip\": \"{}\",
                    \"dev_type\": \"{}\",
                    \"dev_signal\": \"0dbm\",
                    \"connect_type\": \"lan\",
                    \"bind_state\": \"free\"
                    {}",
                        "{", printer.name, printer.id, printer.ip, printer.model, "}"
                    )
                    .trim()
                    .as_bytes()
                );
                ffi::bambu_network_cb_printer_available(&json);
             }
             sleep(Duration::from_secs(5)).await;
        }
    });
}

pub fn bambu_network_rs_log_debug(message: String) {
    debug!("cxx: {}", message);
}

pub fn bambu_network_rs_connect(device_id: String) -> i32 {
    debug!("Attempting connection to {}", device_id);
    
    let printer = {
        let printers = PRINTERS.lock().unwrap();
        printers.iter().find(|p| p.id == device_id).cloned()
    };
    
    if let Some(printer) = printer {
        RUNTIME.spawn(async move {
            match MqttClient::connect(printer.clone()).await {
                Ok(mut client) => {
                    debug!("Connected to printer {}", device_id);
                    
                    // Subscribe to report topic
                    if let Err(e) = client.subscribe().await {
                        error!("Failed to subscribe: {}", e);
                        return;
                    }
                    
                    CONNECTIONS.lock().unwrap().insert(device_id.clone(), client.clone());
                    
                    // Notify C++
                    {
                        let_cxx_string!(device_id_cxx = device_id.clone());
                        ffi::bambu_network_cb_connected(&device_id_cxx);
                    }
                    
                    // Handle incoming messages
                    let mut stream = client.get_stream();
                    while let Some(msg_opt) = stream.next().await {
                        if let Some(msg) = msg_opt {
                            let payload = msg.payload_str();
                            let_cxx_string!(id = device_id.clone());
                            let_cxx_string!(json = payload.as_bytes());
                            ffi::bambu_network_cb_message_recv(&id, &json);
                        } else {
                            // Connection lost
                            warn!("Connection lost for {}", device_id);
                            break;
                        }
                    }
                    
                    CONNECTIONS.lock().unwrap().remove(&device_id);
                },
                Err(e) => {
                    error!("Failed to connect to printer {}: {}", device_id, e);
                }
            }
        });
        0
    } else {
        error!("Printer {} not found in config", device_id);
        -1
    }
}

pub fn bambu_network_rs_disconnect(device_id: String) -> i32 {
    debug!("Disconnecting from {}", device_id);
    CONNECTIONS.lock().unwrap().remove(&device_id);
    0
}

pub fn bambu_network_rs_send(device_id: String, data: String) -> i32 {
    debug!("Sending to {}: {}", device_id, data);

    RUNTIME.block_on(async {
        let client = {
            let connections = CONNECTIONS.lock().unwrap();
            connections.get(&device_id).cloned()
        };
        
        if let Some(client) = client {
            match client.publish(data).await {
                Ok(_) => BAMBU_NETWORK_SUCCESS,
                Err(e) => {
                    error!("Failed to send message: {}", e);
                    BAMBU_NETWORK_ERR_SEND_MSG_FAILED
                }
            }
        } else {
            warn!("No connection found for {}", device_id);
            BAMBU_NETWORK_ERR_SEND_MSG_FAILED
        }
    })
}

pub fn bambu_network_rs_upload_file(
    device_id: String,
    local_filename: String,
    remote_filename: String,
) -> i32 {
    debug!("Uploading file to {}", device_id);
    
    let printer = {
        let printers = PRINTERS.lock().unwrap();
        printers.iter().find(|p| p.id == device_id).cloned()
    };
    
    if let Some(printer) = printer {
        let result = std::thread::spawn(move || {
             let mut blob = vec![];
             if let Err(e) = File::open(&local_filename).and_then(|mut f| f.read_to_end(&mut blob)) {
                 error!("Failed to read local file: {}", e);
                 return BAMBU_NETWORK_ERR_SEND_MSG_FAILED;
             }
             
             match FtpClient::upload_file(&printer, &remote_filename, &blob) {
                 Ok(_) => {
                    debug!("Successfully uploaded file to {}", remote_filename);
                    BAMBU_NETWORK_SUCCESS
                 },
                Err(e) => {
                     error!("FTP upload failed: {}", e);
                     BAMBU_NETWORK_ERR_SEND_MSG_FAILED
                 }
             }
        }).join();
        
        match result {
            Ok(code) => code,
            Err(_) => BAMBU_NETWORK_ERR_SEND_MSG_FAILED,
        }
    } else {
        error!("Printer {} not found", device_id);
        BAMBU_NETWORK_ERR_SEND_MSG_FAILED
    }
}

pub fn bambu_network_rs_get_camera_url(device_id: String) -> String {
    debug!("Getting camera URL for {}", device_id);
    
    let printer = {
        let printers = PRINTERS.lock().unwrap();
        printers.iter().find(|p| p.id == device_id).cloned()
    };
    
    if let Some(printer) = printer {
        // Construct RTSP URL: rtsp://{ip}/live
        format!("rtsp://{}/live", printer.ip)
    } else {
        error!("Printer {} not found", device_id);
        String::new()
    }
}
