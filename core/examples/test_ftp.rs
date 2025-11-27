use open_bambu_core::ftp::FtpClient;
use open_bambu_core::printer::Printer;
use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <ip> <access_code> <serial>", args[0]);
        return Ok(());
    }

    let ip = &args[1];
    let access_code = &args[2];
    let serial = &args[3];

    println!("Testing FTP upload to {} ({})", ip, serial);

    let printer = Printer {
        id: serial.to_string(),
        ip: ip.to_string(),
        name: "Test Printer".to_string(),
        model: "X1C".to_string(),
        password: access_code.to_string(),
        mqtt_port: 8883,
        ftp_port: 990,
        ssl: true,
    };

    let content = b"Hello, Bambu!";
    let filename = "test_upload.gcode";

    println!("Attempting upload...");
    FtpClient::upload_file(&printer, filename, content)?;
    println!("Upload successful!");

    Ok(())
}
