use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

use winapi::shared::minwindef::DWORD;
use winapi::shared::winerror::ERROR_ALREADY_EXISTS;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::fileapi::ReadFile;
use winapi::um::handleapi::CloseHandle;
use winapi::um::namedpipeapi::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, WaitNamedPipeW,
};
use winapi::um::synchapi::CreateMutexW;
use winapi::um::winbase::{
    PIPE_ACCESS_INBOUND, PIPE_READMODE_MESSAGE, PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES,
    PIPE_WAIT,
};

const MUTEX_NAME: &str = "Local\\RenameAll_SingleInstance_Mutex_v1\0";
const PIPE_NAME: &str = "\\\\.\\pipe\\RenameAll_SingleInstance_Pipe_v1\0";
const PIPE_PATH_STR: &str = r"\\.\pipe\RenameAll_SingleInstance_Pipe_v1";

pub enum InstanceRole {
    Primary(Receiver<Vec<PathBuf>>),
    Secondary,
}

/// Checks if an instance is already running.
/// If running: sends paths to the primary instance and returns InstanceRole::Secondary.
/// If primary: starts IPC pipe server and returns InstanceRole::Primary(rx).
pub fn check_single_instance(paths_to_send: &[PathBuf]) -> InstanceRole {
    let mutex_name_w: Vec<u16> = MUTEX_NAME.encode_utf16().collect();
    let mutex_handle = unsafe { CreateMutexW(null_mut(), 0, mutex_name_w.as_ptr()) };

    let is_already_running = if mutex_handle.is_null() {
        false
    } else {
        unsafe { GetLastError() == ERROR_ALREADY_EXISTS }
    };

    if is_already_running {
        // Send paths to the existing primary instance
        send_to_primary(paths_to_send);
        InstanceRole::Secondary
    } else {
        // We are the primary instance
        let (tx, rx) = channel();
        start_pipe_server(tx);
        InstanceRole::Primary(rx)
    }
}

/// Client: sends paths to the running primary instance over the named pipe
fn send_to_primary(paths: &[PathBuf]) {
    let mut payload = String::new();
    for p in paths {
        payload.push_str(&p.to_string_lossy());
        payload.push('\n');
    }

    let pipe_name_w: Vec<u16> = PIPE_NAME.encode_utf16().collect();

    // Try connecting with retry (up to 2.5 seconds in case the primary instance is just starting)
    for _ in 0..50 {
        unsafe {
            WaitNamedPipeW(pipe_name_w.as_ptr(), 100);
        }

        if let Ok(mut stream) = OpenOptions::new().write(true).open(PIPE_PATH_STR) {
            let _ = stream.write_all(payload.as_bytes());
            let _ = stream.flush();
            return;
        }

        thread::sleep(Duration::from_millis(50));
    }
}

/// Server: listens for paths from incoming secondary instances
fn start_pipe_server(tx: Sender<Vec<PathBuf>>) {
    thread::spawn(move || {
        let pipe_name_w: Vec<u16> = PIPE_NAME.encode_utf16().collect();

        loop {
            let h_pipe = unsafe {
                CreateNamedPipeW(
                    pipe_name_w.as_ptr(),
                    PIPE_ACCESS_INBOUND,
                    PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                    PIPE_UNLIMITED_INSTANCES,
                    8192,
                    8192,
                    0,
                    null_mut(),
                )
            };

            if h_pipe.is_null() || h_pipe == winapi::um::handleapi::INVALID_HANDLE_VALUE {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            let connected = unsafe { ConnectNamedPipe(h_pipe, null_mut()) };
            let is_connected = connected != 0
                || unsafe { GetLastError() == winapi::shared::winerror::ERROR_PIPE_CONNECTED };

            if is_connected {
                let mut buffer = [0u8; 8192];
                let mut bytes_read: DWORD = 0;
                let success = unsafe {
                    ReadFile(
                        h_pipe,
                        buffer.as_mut_ptr() as *mut _,
                        buffer.len() as DWORD,
                        &mut bytes_read,
                        null_mut(),
                    )
                };

                if success != 0 && bytes_read > 0 {
                    let text = String::from_utf8_lossy(&buffer[..bytes_read as usize]);
                    let paths: Vec<PathBuf> = text
                        .lines()
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(PathBuf::from)
                        .collect();

                    if !paths.is_empty() {
                        let _ = tx.send(paths);
                    }
                }

                unsafe {
                    DisconnectNamedPipe(h_pipe);
                }
            }

            unsafe {
                CloseHandle(h_pipe);
            }
        }
    });
}
