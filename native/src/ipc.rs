// Control socket ($XDG_RUNTIME_DIR/windowlight.sock, one command per line) and the niri workspace follower.
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

/// a command for the render loop; the reply goes back on `reply` when it is set
pub struct Cmd { pub line: String, pub reply: Option<Sender<String>> }

pub fn sock_path() -> String {
    format!("{}/windowlight.sock", std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into()))
}

pub fn send(line: &str) -> std::io::Result<String> {
    let mut s = UnixStream::connect(sock_path())?;
    s.set_read_timeout(Some(Duration::from_secs(3)))?;
    writeln!(s, "{line}")?;
    let mut reply = String::new();
    BufReader::new(s).read_line(&mut reply)?;
    Ok(reply.trim_end().to_string())
}

/// start the control socket; commands arrive on the returned receiver
pub fn listen() -> Result<(Sender<Cmd>, Receiver<Cmd>), String> {
    let path = sock_path();
    if UnixStream::connect(&path).is_ok() { return Err(format!("another windowlight is already listening on {path}")); }
    let _ = std::fs::remove_file(&path);
    let l = UnixListener::bind(&path).map_err(|e| format!("{path}: {e}"))?;
    let (tx, rx) = channel();
    let tx2 = tx.clone();
    std::thread::spawn(move || {
        for conn in l.incoming().flatten() {
            let tx = tx2.clone();
            std::thread::spawn(move || {
                let Ok(mut w) = conn.try_clone() else { return };
                for line in BufReader::new(conn).lines().map_while(Result::ok) {
                    let (rtx, rrx) = channel();
                    if tx.send(Cmd { line, reply: Some(rtx) }).is_err() { return; }
                    let reply = rrx.recv_timeout(Duration::from_secs(2)).unwrap_or_else(|_| "busy".into());
                    if writeln!(w, "{reply}").is_err() { return; }
                }
            });
        }
    });
    Ok((tx, rx))
}

pub fn cleanup() { let _ = std::fs::remove_file(sock_path()); }

/// follow niri's event stream and send "workspace N" (N = 1-based index on `output`) when it changes
pub fn follow_niri(output: String, tx: Sender<Cmd>) {
    let Ok(path) = std::env::var("NIRI_SOCKET") else { eprintln!("NIRI_SOCKET isn't set; workspace moods are off"); return };
    std::thread::spawn(move || {
        let Ok(mut s) = UnixStream::connect(&path) else { eprintln!("can't reach niri at {path}"); return };
        if writeln!(s, "\"EventStream\"").is_err() { return; }
        // workspace id -> (idx, output)
        let mut ws: std::collections::HashMap<u64, (u64, String)> = Default::default();
        let mut last = 0u64;
        let mut emit = |idx: u64| {
            if idx != last { last = idx; let _ = tx.send(Cmd { line: format!("workspace {idx}"), reply: None }); }
        };
        for line in BufReader::new(s).lines().map_while(Result::ok) {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
            if let Some(list) = v.pointer("/WorkspacesChanged/workspaces").and_then(|x| x.as_array()) {
                ws.clear();
                for w in list {
                    let (Some(id), Some(idx)) = (w["id"].as_u64(), w["idx"].as_u64()) else { continue };
                    let out = w["output"].as_str().unwrap_or("").to_string();
                    if out == output && w["is_active"].as_bool() == Some(true) { emit(idx); }
                    ws.insert(id, (idx, out));
                }
            } else if let Some(id) = v.pointer("/WorkspaceActivated/id").and_then(|x| x.as_u64()) {
                if let Some((idx, out)) = ws.get(&id) { if *out == output { emit(*idx); } }
            }
        }
    });
}
