use crate::printer::Printer;
use suppaftp::{NativeTlsFtpStream as FtpStream, NativeTlsConnector, Mode};
use std::io::Cursor;
use native_tls::TlsConnector;
use log::debug;

pub struct FtpClient;

impl FtpClient {
    pub fn upload_file(printer: &Printer, filename: &str, content: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Connecting to FTP server at {}:{} (SSL: {})", printer.ip, printer.ftp_port, printer.ssl);
        
        let addr = format!("{}:{}", printer.ip, printer.ftp_port);
        let mut stream = if printer.ssl {
            // Use implicit FTPS for port 990 - TLS must be established BEFORE reading server welcome
            debug!("Using implicit FTPS (connect_secure_implicit)");
            let connector = TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true)
                .build()?;
            
            // connect_secure_implicit is available with "deprecated" feature
            FtpStream::connect_secure_implicit(addr, NativeTlsConnector::from(connector), &printer.ip)?
        } else {
            // Plain FTP connection
            debug!("Using plain FTP");
            FtpStream::connect(addr)?
        };
        debug!("FTP connection established and ready");

        debug!("Logging in as user 'bblp'");
        stream.login("bblp", &printer.password)?;
        debug!("Login successful");
        
        debug!("Setting passive mode");
        stream.set_mode(Mode::Passive);
        debug!("Passive mode enabled");
        
        // Delete existing file if any (ignore error)
        debug!("Removing existing file if present: {}", filename);
        let _ = stream.rm(filename);
        
        debug!("Uploading file: {} ({} bytes)", filename, content.len());
        let mut reader = Cursor::new(content);
        stream.put_file(filename, &mut reader)?;
        debug!("File upload complete");
        
        debug!("Closing FTP connection");
        stream.quit()?;
        debug!("FTP connection closed successfully");
        Ok(())
    }
}

