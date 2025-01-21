# Использование DLL в приложении Tauri на Windows

Этот простой пример покажет как подключить и использовать сторонние dll библиотеки в приложении tauri для windows.

## Проверка dll

Перед тем как подключать dll к своему проекту, проверьте что библиотека создана и вы можете ей пользоваться.

Чтобы проверить что dll работает, можно использовать команду:
    
    Rundll32.exe {путь до библиотеки},{имя команды}

## Создание dll

Если вы хотите создать свою библиотеку на более простом языке чем Rust, можете посмотреть пример создания на Go [https://github.com/JackGarner/Go_DLL_Create_Example]

* В нашем примере мы будем использовать dll созданную на Go

## Создание проект tauri

Использовать будем tauri 2. (Полная инструкция как создать простой проект есть тут [https://v2.tauri.app/start/])

Создаем проект tauri-use-dll-examlpe

    npm create tauri-app@latest

Переходим в каталог

    cd tauri-use-dll-examlpe

Устанавливаем зависимости 

    npm install

Проверяем что запускается

    npm run tauri dev

## Настройка tauri
Переходим в src-tauri
Важно! dll должна лежать именно в этой папке, т.к. Rust считает ее основной в приложении tauri.

    cd src-tauri

Создаем каталог lib в папке src-tauri

    mkdir lib

В файле \src-tauri\tauri.conf.json добавляем зависимость на созданную папку: bundle -> resources -> ["lib/*"]
Полностью конфиг получится такой
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

Копируем dll, которую хотим использовать в каталог lib.
В моем случае это \lib\example_from_go.dll

## Зависимости Rust
Для использования dll мы будем использовать 2 дополнительные зависимости. Их необходимо добавить в файл Cargo.toml в [dependencies]:
    libc = "0.2"
    libloading = "0.8"

## Доработка Rust
Открываем файл \src-tauri\src\lib.rs
Добавляем зависимости (можно попробовать и не добавлять, но лучше добавить)
    
    use libloading::{Library, Symbol};
    use std::thread;
    use std::sync::mpsc;
    use serde_json::json;
    use std::ffi::CStr;

### Простой вызов: 
Добавляем новый метод

    fn call_simle() -> Result<u32, Box<dyn std::error::Error>> {
        unsafe {
            let lib = libloading::Library::new("lib/example_from_go.dll")?;
            let func: libloading::Symbol<unsafe extern "C" fn() -> u32> = lib.get(b"sayHello")?;
            let res = func();
            println!("{}", res);
            Ok(0)
        }
    }

Дорабатываем вызов из метода greet

    fn greet(name: &str) -> String {
        let _ = call_simle();
        format!("Hello, {}! You've been greeted from Rust!", name)
    }

В консоли вы должны увидеть "Hello world!"

### Сложный вызов: 
Если ваша dll использует какие-то сложные операции, например использование COM (ActiveX), то простой вызов скорее всего не сработает. Для таких ситуаций необходимо делать вызов через дочерный процесс.
Добавляем новый метод

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
                // serialize string to JSON
                match result {
                    Ok(value) => serde_json::to_string(&value).map_err(|e| e.to_string()),
                    Err(err) => Err(err),
                }
            }
            Err(e) => Err(format!("Failed to receive result: {:?}", e)),
        }
    }

Дорабатываем вызов из метода greet

    fn greet(name: &str) -> String {
        match call_complex() {
            Ok(json_result) => json_result,  // return as JSON string
            Err(err) => json!({ "error": err }).to_string(),  // error as JSON
        }
    }

Проверка:
Запускаем приложение 

     npm run tauri dev

Когда появится окно приложения, нажимаем кнопку Greet
В окне приложения вы должны увидеть "{"result":"hello from excel"}"