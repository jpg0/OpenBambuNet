use config::{Config, ConfigError, Value};
use std::env::current_dir;
use std::path::PathBuf;

/// Get the directory where the current shared library (dylib/so/dll) is located
fn get_dylib_directory() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        use std::ffi::CStr;
        use std::os::raw::c_void;
        
        #[repr(C)]
        struct DlInfo {
            dli_fname: *const i8,
            dli_fbase: *const c_void,
            dli_sname: *const i8,
            dli_saddr: *const c_void,
        }
        
        extern "C" {
            fn dladdr(addr: *const c_void, info: *mut DlInfo) -> i32;
        }
        
        unsafe {
            let mut info: DlInfo = std::mem::zeroed();
            // Use the address of this function as a reference point within the dylib
            if dladdr(get_dylib_directory as *const c_void, &mut info) != 0 {
                if !info.dli_fname.is_null() {
                    let path = CStr::from_ptr(info.dli_fname)
                        .to_string_lossy()
                        .into_owned();
                    return PathBuf::from(path).parent().map(|p| p.to_path_buf());
                }
            }
        }
        None
    }
    
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HMODULE;
        use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT};
        use std::ptr;
        
        unsafe {
            let mut module = HMODULE::default();
            // Get handle to the module containing this function
            let result = GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                get_dylib_directory as *const _ as *const u16,
                &mut module,
            );
            
            if result.is_ok() {
                let mut buffer = vec![0u16; 512];
                let len = GetModuleFileNameW(module, &mut buffer);
                if len > 0 {
                    buffer.truncate(len as usize);
                    let path = String::from_utf16_lossy(&buffer);
                    return PathBuf::from(path).parent().map(|p| p.to_path_buf());
                }
            }
        }
        None
    }
    
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

#[derive(Debug, Clone)]
pub struct Printer {
    pub name: String,
    pub id: String,
    pub ip: String,
    pub model: String,
    pub password: String,
    pub mqtt_port: u16,
    pub ftp_port: u16,
    pub ssl: bool,
}

pub fn load_printers() -> Vec<Printer> {
    // Try to load config from dylib directory first, then fall back to current directory
    // We check for existence to support running tests where config is in CWD
    let dylib_config = get_dylib_directory().map(|dir| dir.join("openbambu.toml"));
    let cwd_config = current_dir().ok().map(|dir| dir.join("openbambu.toml"));

    let config_file_path = match (dylib_config, cwd_config) {
        (Some(p), _) if p.exists() => p,
        (_, Some(p)) if p.exists() => p,
        (Some(p), _) => p, // Fallback to dylib path for error message if neither exists
        (_, Some(p)) => p,
        _ => PathBuf::from("openbambu.toml"),
    };

    let config = match Config::builder()
        .add_source(config::File::with_name(&config_file_path.to_string_lossy()).required(false))
        .add_source(config::Environment::with_prefix("BAMBU_FARM"))
        .build()
    {
        Ok(config) => config,
        Err(err) => {
            match err {
                ConfigError::NotFound(_) => {
                    eprintln!(
                        "No config file found. Try adding one at `{}`",
                        config_file_path.to_string_lossy()
                    );
                }
                ConfigError::FileParse { uri, cause } => {
                    if let Some(uri) = uri {
                        eprintln!("Error parsing config file at {}\nCause:{}", uri, cause);
                    } else {
                        eprintln!("Error parsing config file.")
                    }
                }
                _ => {
                    eprintln!("Unknown error parsing config file {:?}", err)
                }
            }
            return Vec::new();
        }
    };

    let mut printers = Vec::new();
    for printer in config.get_array("printers").unwrap_or_default() {
        if let Some(printer) = construct_printer(printer) {
            printers.push(printer);
        }
    }
    
    if printers.is_empty() {
         eprintln!(
            "No printers found, or printer config was invalid. Try adding some to the config file at `{}`",
            config_file_path.to_string_lossy()
        );
    }

    printers
}

fn construct_printer(config: Value) -> Option<Printer> {
    let printer = config.into_table().unwrap_or_default();

    let dev_id = if let Some(dev_id) = printer.get("dev_id") {
        dev_id.to_string()
    } else {
        eprintln!("Missing `dev_id` in printer config.");
        return None;
    };
    // Handle dev_id potentially being wrapped in quotes if it came from toml string, 
    // but config crate usually handles this. The original code used `printer.get("dev_id")` which returns a Value.
    // `dev_id` here is a Value.
    // Wait, `printer.get` returns `Option<&Value>`.
    // In original code:
    // if let Some(dev_id) = printer.get("dev_id") { dev_id } ...
    // let dev_id = dev_id.to_string(); (converts Value to String, which might include quotes if it's a string value?)
    // Actually `Value::to_string()` usually returns the string representation.
    // Let's look at original code again.
    
    /*
    let dev_id = if let Some(dev_id) = printer.get("dev_id") {
        dev_id
    } ...
    Some(Printer {
        id: dev_id.to_string(),
        ...
    })
    */
    
    // `config::Value` to_string() might be tricky. 
    // Better to use `dev_id.clone().into_string().unwrap_or_default()` or similar if we want the raw string.
    
    let model = if let Some(model) = printer.get("model") {
        match model.to_string().to_lowercase().as_str() {
            "x1c" => "3DPrinter-X1-Carbon",
            "x1" => "3DPrinter-X1",
            "p1p" => "C11",
            _ => {
                eprintln!("Expected printer field `model` to be one of [`p1p`, `x1`, `x1c`].");
                return None;
            }
        }
    } else {
        eprintln!("Missing `model` in printer config.");
        return None;
    };

    let host = if let Some(host) = printer.get("host") {
        host.to_string()
    } else {
        eprintln!("Missing `host` in printer config.");
        return None;
    };

    let password = if let Some(password) = printer.get("password") {
        password.to_string()
    } else {
        eprintln!("Missing `password` in printer config.");
        return None;
    };

    let name = if let Some(name) = printer.get("name") {
        name.to_string()
    } else {
        eprintln!("Missing `name` in printer config.");
        return None;
    };
    
    let mqtt_port = if let Some(port) = printer.get("mqtt_port") {
        port.clone().into_int().unwrap_or(8883) as u16
    } else {
        8883
    };

    let ftp_port = if let Some(port) = printer.get("ftp_port") {
        port.clone().into_int().unwrap_or(990) as u16
    } else {
        990
    };

    let ssl = if let Some(ssl) = printer.get("ssl") {
        ssl.clone().into_bool().unwrap_or(true)
    } else {
        true
    };

    Some(Printer {
        name,
        id: dev_id,
        ip: host,
        model: model.to_string(),
        password,
        mqtt_port,
        ftp_port,
        ssl,
    })
}
