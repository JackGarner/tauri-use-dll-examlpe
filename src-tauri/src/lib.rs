// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use libc::c_char;
use libloading::{Library, Symbol};
use serde_json::json;
use std::ffi::{CStr, CString};
use std::path::Path;
use std::sync::mpsc;
use std::thread;

#[tauri::command]
fn greet(name: &str) -> String {
    let _ = call_simle();

    match call_complex() {
        Ok(json_result) => json_result,  // Возвращаем JSON строку
        Err(err) => json!({ "error": err }).to_string(),  // Возвращаем ошибку в JSON формате
    }
}

fn call_simle() -> Result<u32, Box<dyn std::error::Error>> {
    unsafe {
        let lib = libloading::Library::new("lib/example_from_go.dll")?;
        let func: libloading::Symbol<unsafe extern "C" fn() -> u32> = lib.get(b"sayHello")?;
        let res = func();
        println!("{}", res);
        Ok(0)
    }
}

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

            // Конвертируем C строку в Rust строку
            let result_str = CStr::from_ptr(result_ptr).to_string_lossy().into_owned();

            // Возвращаем данные в формате JSON
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
            // Сериализация результата в строку JSON
            match result {
                Ok(value) => serde_json::to_string(&value).map_err(|e| e.to_string()),
                Err(err) => Err(err),
            }
        }
        Err(e) => Err(format!("Failed to receive result: {:?}", e)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
