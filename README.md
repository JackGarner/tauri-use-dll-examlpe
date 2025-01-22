# Using DLLs in a Tauri Application on Windows.

This simple example will show how to connect and use third-party DLL libraries in a Tauri application for Windows.

## Checking the DLL

Before connecting the DLL to your project, ensure that the library is created and you can use it.

To check if the DLL works, you can use the command:
    
    Rundll32.exe {path to the library},{command name}

## Creating a DLL

If you want to create your own library in a simpler language than Rust, you can take a look at an example of creating one in Go [https://github.com/JackGarner/Go_DLL_Create_Example].

* In our example, we will use a DLL created in Go.

## Creating a Tauri Project

We will be using Tauri 2. (Complete instructions on how to create a simple project can be found here [https://v2.tauri.app/start/].)

Create a Tauri project named tauri-use-dll-examlpe:

    npm create tauri-app@latest

Navigate to the directory:

    cd tauri-use-dll-examlpe

Install dependencies:

    npm install

Check if it runs:

    npm run tauri dev

## Configuring Tauri
Navigate to src-tauri.
Important! The DLL must be located in this folder, as Rust considers it the main directory in the Tauri application.

    cd src-tauri

Create a lib directory in the src-tauri folder:

    mkdir lib

In the file \src-tauri\tauri.conf.json, add a dependency on the created folder: bundle -> resources -> ["lib/*"].
The complete config will look like this:

    {
        "$schema": "https://schema.tauri.app/config/2",
        "productName": "tauri-use-dll-examlpe",
        "version": "0.1.0",
        "identifier": "com.plorum.tauri-use-dll-examlpe",
        "build": {
            "beforeDevCommand": "npm run dev",
            "devUrl": "http://localhost:1420",
            "beforeBuildCommand": "npm run build",
            "frontendDist": "../dist"
        },
        "app": {
            "windows": [
            {
                "title": "tauri-use-dll-examlpe",
                "width": 800,
                "height": 600
            }
            ],
            "security": {
            "csp": null
            }
        },
        "bundle": {
            "active": true,
            "targets": "all",
            "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"],
            "resources":["lib/*"]
        }
    }

Copy the DLL you want to use into the lib directory.
In my case, this is \lib\example_from_go.dll.

## Rust Dependencies
To use the DLL, we will use 2 additional dependencies. You need to add them to the Cargo.toml file in [dependencies]:

    libc = "0.2"
    libloading = "0.8"

## Rust Modifications
Open the file \src-tauri\src\lib.rs.
Add the dependencies (you can try not adding, but it's better to add):
    
    use libloading::{Library, Symbol};
    use std::thread;
    use std::sync::mpsc;
    use serde_json::json;
    use std::ffi::CStr;

### Simple Call: 
Add a new method:

    fn call_simle() -> Result<u32, Box<dyn std::error::Error>> {
        unsafe {
            let lib = libloading::Library::new("lib/example_from_go.dll")?;
            let func: libloading::Symbol<unsafe extern "C" fn() -> u32> = lib.get(b"sayHello")?;
            let res = func();
            println!("{}", res);
            Ok(0)
        }
    }

Modify the call in the greet method:

    fn greet(name: &str) -> String {
        let _ = call_simle();
        format!("Hello, {}! You've been greeted from Rust!", name)
    }

Check:
Run the application:

     npm run tauri dev

When the application window appears, click the Greet button.
In the console, you should see "Hello world!"

### Complex Call: 
If your DLL uses complex operations, such as COM (ActiveX), then a simple call will likely not work. For such situations, you need to make the call through a child process.
Add a new method:

    fn call_complex() -> Result<String, String> {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            let result = std::panic::catch_unwind(|| unsafe {
                let dll_path = Path::new("lib/example_from_go.dll");
                let dll = match Library::new(dll_path) {
                    Ok(dll) => dll,
                    Err(e) => {
                        tx.send(Err(format!("Failed to load DLL: {}", e))).unwrap();
                        return;
                    }
                };

                let read_excel_file: Symbol<unsafe extern "C" fn(*const c_char) -> *const c_char> =
                    match dll.get(b"ReadExcelFile") {
                        Ok(func) => func,
                        Err(e) => {
                            tx.send(Err(format!("Failed to load ReadExcelFile function: {}", e)))
                                .unwrap();
                            return;
                        }
                    };

                let file_path =
                    CString::new("D:/Sandbox/Codding/GO/Go_DLL_Create_Example/file.xlsx").expect("CString::new failed");
                let result_ptr = read_excel_file(file_path.as_ptr());
                if result_ptr.is_null() {
                    tx.send(Err("ReadExcelFile returned null pointer".to_string()))
                        .unwrap();
                    return;
                }

                // Convert C string to Rust string
                let result_str = CStr::from_ptr(result_ptr).to_string_lossy().into_owned();

                // Return data in JSON format
                let json_result = json!({ "result": result_str });

                tx.send(Ok(json_result)).unwrap();
            });

            if let Err(err) = result {
                tx.send(Err(format!("Thread panicked: {:?}", err))).unwrap();
            }
        });

        handle.join().unwrap();

        match rx.recv() {
            Ok(result) => {
                // serialize string to JSON
                match result {
                    Ok(value) => serde_json::to_string(&value).map_err(|e| e.to_string()),
                    Err(err) => Err(err),
                }
            }
            Err(e) => Err(format!("Failed to receive result: {:?}", e)),
        }
    }

Modify the call in the greet method:

    fn greet(name: &str) -> String {
        match call_complex() {
            Ok(json_result) => json_result,  // return as JSON string
            Err(err) => json!({ "error": err }).to_string(),  // error as JSON
        }
    }

Check:
Run the application:

     npm run tauri dev

When the application window appears, click the Greet button.
In the application window, you should see "{"result":"hello from excel"}".
