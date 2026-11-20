// A small control page on http://127.0.0.1:PORT. Only answers requests addressed to localhost, and only
// accepts commands sent from its own page, so other websites can't drive it.
use crate::ipc::Cmd;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

const PAGE: &str = include_str!("../ui.html");

pub fn serve(port: u16, tx: Sender<Cmd>) {
    let l = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(e) => { eprintln!("windowlight: no control page, port {port}: {e}"); return; }
    };
    eprintln!("windowlight: control page http://127.0.0.1:{port}/");
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let tx = tx.clone();
            std::thread::spawn(move || { let _ = handle(s, port, &tx); });
        }
    });
}

fn handle(mut s: TcpStream, port: u16, tx: &Sender<Cmd>) -> std::io::Result<()> {
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut r = BufReader::new(s.try_clone()?);
    let mut first = String::new();
    r.read_line(&mut first)?;
    let (mut host, mut origin, mut len) = (String::new(), None::<String>, 0usize);
    loop {
        let mut h = String::new();
        if r.read_line(&mut h)? == 0 || h.trim().is_empty() { break; }
        let Some((k, v)) = h.split_once(':') else { continue };
        let v = v.trim().to_string();
        match k.to_ascii_lowercase().as_str() {
            "host" => host = v,
            "origin" => origin = Some(v),
            "content-length" => len = v.parse().unwrap_or(0).min(4096),
            _ => {}
        }
    }
    let ok_hosts = [format!("127.0.0.1:{port}"), format!("localhost:{port}")];
    if !ok_hosts.contains(&host) { return reply(&mut s, "403 Forbidden", "text/plain", "wrong host"); }
    let mut parts = first.split_whitespace();
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    match (method, path) {
        ("GET", "/") => reply(&mut s, "200 OK", "text/html; charset=utf-8", PAGE),
        ("GET", "/state") => reply(&mut s, "200 OK", "application/json", &ask(tx, "state")),
        ("POST", "/cmd") => {
            if origin.is_some_and(|o| !ok_hosts.iter().any(|h| o == format!("http://{h}"))) {
                return reply(&mut s, "403 Forbidden", "text/plain", "wrong origin");
            }
            let mut body = vec![0u8; len];
            r.read_exact(&mut body)?;
            let line = String::from_utf8_lossy(&body).lines().next().unwrap_or("").to_string();
            reply(&mut s, "200 OK", "text/plain; charset=utf-8", &ask(tx, &line))
        }
        _ => reply(&mut s, "404 Not Found", "text/plain", "not here"),
    }
}

fn ask(tx: &Sender<Cmd>, line: &str) -> String {
    let (rtx, rrx) = channel();
    if tx.send(Cmd { line: line.to_string(), reply: Some(rtx) }).is_err() { return "stopped".into(); }
    rrx.recv_timeout(Duration::from_secs(2)).unwrap_or_else(|_| "busy".into())
}

fn reply(s: &mut TcpStream, status: &str, kind: &str, body: &str) -> std::io::Result<()> {
    write!(s, "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}", body.len())
}
