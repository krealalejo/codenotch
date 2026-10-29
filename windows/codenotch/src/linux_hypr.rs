use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

fn socket() -> Option<PathBuf> {
    let signature = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(PathBuf::from(runtime).join("hypr").join(signature).join(".socket.sock"))
}

pub fn available() -> bool {
    socket().is_some_and(|p| p.exists())
}

fn request(command: &str) -> Option<String> {
    let mut stream = UnixStream::connect(socket()?).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(500))).ok()?;
    stream.set_write_timeout(Some(Duration::from_millis(500))).ok()?;
    stream.write_all(command.as_bytes()).ok()?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply).ok()?;
    Some(reply)
}

fn json(command: &str) -> Option<serde_json::Value> {
    serde_json::from_str(&request(&format!("j/{command}"))?).ok()
}

pub fn cursor() -> Option<(f64, f64)> {
    let position = json("cursorpos")?;
    Some((position.get("x")?.as_f64()?, position.get("y")?.as_f64()?))
}

pub fn active_pid() -> Option<u32> {
    let pid = json("activewindow")?.get("pid")?.as_i64()?;
    u32::try_from(pid).ok().filter(|p| *p != 0)
}

fn pick<'a>(clients: &'a [serde_json::Value], chain: &[u32]) -> Option<&'a str> {
    let mut best: Option<(usize, i64, &str)> = None;
    for client in clients {
        let (Some(pid), Some(address)) = (client.get("pid").and_then(|p| p.as_i64()), client.get("address").and_then(|a| a.as_str())) else {
            continue;
        };
        let Some(depth) = u32::try_from(pid).ok().and_then(|p| chain.iter().position(|c| *c == p)) else {
            continue;
        };
        let recency = client.get("focusHistoryID").and_then(|f| f.as_i64()).unwrap_or(i64::MAX);
        if best.is_none_or(|(d, r, _)| depth < d || (depth == d && recency < r)) {
            best = Some((depth, recency, address));
        }
    }
    best.map(|(_, _, address)| address)
}

pub fn focus_pid_in(chain: &[u32]) -> bool {
    let Some(serde_json::Value::Array(clients)) = json("clients") else {
        return false;
    };
    let Some(address) = pick(&clients, chain) else { return false };
    request(&format!("dispatch focuswindow address:{address}")).is_some_and(|r| r.trim() == "ok")
}

#[cfg(test)]
mod tests {
    use super::pick;
    use serde_json::json;

    #[test]
    fn picks_the_window_nearest_the_session_then_the_most_recent() {
        let clients = vec![
            json!({"pid": 10, "address": "0xterm-old", "focusHistoryID": 4}),
            json!({"pid": 10, "address": "0xterm-new", "focusHistoryID": 1}),
            json!({"pid": 99, "address": "0xbrowser", "focusHistoryID": 0}),
            json!({"pid": 7, "address": "0xnear", "focusHistoryID": 9}),
        ];
        assert_eq!(pick(&clients, &[5, 7, 10]), Some("0xnear"));
        assert_eq!(pick(&clients, &[5, 10]), Some("0xterm-new"));
        assert_eq!(pick(&clients, &[5, 6]), None);
    }
}
