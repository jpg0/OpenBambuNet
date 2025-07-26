use std::sync::Once;

use crate::bambu_farm::bambu_farm_client::BambuFarmClient;
use crate::bambu_farm::{
    ConnectRequest, PrinterOptionRequest, RecvMessage, SendMessageRequest, UploadFileRequest,
};
use crate::main;
use std::time::Duration;
use tokio::time::sleep;
use tonic::Request;

static INIT: Once = Once::new();

fn initialize() {
    INIT.call_once(|| {
        tokio::spawn(async {
            main().await.unwrap();
        });
    });
}

#[tokio::test]
async fn test_main() {
    initialize();
    sleep(Duration::from_secs(1)).await;
    let mut client = BambuFarmClient::connect("http://127.0.0.1:47403")
        .await
        .unwrap();
    let request = Request::new(PrinterOptionRequest {});
    let mut stream = client
        .get_available_printers(request)
        .await
        .unwrap()
        .into_inner();
    let message = stream.message().await.unwrap().unwrap();
    assert_eq!(message.options.len(), 1);
    assert_eq!(message.options[0].dev_name, "Test Printer");
}

#[tokio::test]
async fn test_connect() {
    initialize();
    sleep(Duration::from_secs(1)).await;
    let mut client = BambuFarmClient::connect("http://127.0.0.1:47403")
        .await
        .unwrap();
    let request = Request::new(ConnectRequest {
        dev_id: "dev_id".into(),
    });
    let mut stream = client.connect_printer(request).await.unwrap().into_inner();
    let message: RecvMessage = stream.message().await.unwrap().unwrap();
    assert!(message.connected);
    assert_eq!(message.dev_id, "dev_id");
    assert_eq!(message.data, "{}");
}

#[tokio::test]
async fn test_send() {
    initialize();
    sleep(Duration::from_secs(1)).await;
    let mut client = BambuFarmClient::connect("http://127.0.0.1:47403")
        .await
        .unwrap();
    let request = Request::new(ConnectRequest {
        dev_id: "dev_id".into(),
    });
    let _ = client.connect_printer(request).await.unwrap().into_inner();
    let request = Request::new(SendMessageRequest {
        dev_id: "dev_id".into(),
        data: "test_data".into(),
    });
    let response = client.send_message(request).await.unwrap().into_inner();
    assert!(response.success);
}

#[tokio::test]
async fn test_upload() {
    initialize();
    sleep(Duration::from_secs(1)).await;
    let mut client = BambuFarmClient::connect("http://127.0.0.1:47403")
        .await
        .unwrap();
    let request = Request::new(UploadFileRequest {
        dev_id: "dev_id".into(),
        blob: "test".into(),
        remote_path: "test".into(),
    });
    let response = client.upload_file(request).await.unwrap().into_inner();
    assert!(response.success);
}
