use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

const RUN_COMMAND: u32 = 0;
const GET_TREE: u32 = 4;
const MAGIC: &[u8; 6] = b"i3-ipc";

fn socket() -> Option<PathBuf> {
    std::env::var_os("SWAYSOCK").map(PathBuf::from)
}

pub fn available() -> bool {
    socket().is_some_and(|p| p.exists())
}

fn request(kind: u32, payload: &str) -> Option<Value> {
    let mut stream = UnixStream::connect(socket()?).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(500))).ok()?;
    stream.set_write_timeout(Some(Duration::from_millis(500))).ok()?;
    let mut message = MAGIC.to_vec();
    message.extend(u32::try_from(payload.len()).ok()?.to_ne_bytes());
    message.extend(kind.to_ne_bytes());
    message.extend(payload.as_bytes());
    stream.write_all(&message).ok()?;
    let mut header = [0u8; 14];
    stream.read_exact(&mut header).ok()?;
    if &header[..6] != MAGIC {
        return None;
    }
    let length = u32::from_ne_bytes(header[6..10].try_into().ok()?) as usize;
    let mut body = vec![0; length];
    stream.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

struct View {
    id: i64,
    pid: u32,
    focused: bool,
}

fn views(node: &Value, out: &mut Vec<View>) {
    if let (Some(id), Some(pid)) = (node["id"].as_i64(), node["pid"].as_i64().and_then(|p| u32::try_from(p).ok())) {
        out.push(View { id, pid, focused: node["focused"].as_bool().unwrap_or(false) });
    }
    for key in ["nodes", "floating_nodes"] {
        for child in node[key].as_array().into_iter().flatten() {
            views(child, out);
        }
    }
}

fn tree() -> Option<Vec<View>> {
    let root = request(GET_TREE, "")?;
    let mut out = Vec::new();
    views(&root, &mut out);
    Some(out)
}

pub fn active_pid() -> Option<u32> {
    tree()?.into_iter().find(|v| v.focused).map(|v| v.pid).filter(|p| *p != 0)
}

fn nearest(views: &[View], chain: &[u32]) -> Option<i64> {
    views
        .iter()
        .filter_map(|v| chain.iter().position(|c| *c == v.pid).map(|depth| (depth, v.id)))
        .min_by_key(|(depth, _)| *depth)
        .map(|(_, id)| id)
}

pub fn focus_pid_in(chain: &[u32]) -> bool {
    let Some(id) = tree().and_then(|views| nearest(&views, chain)) else {
        return false;
    };
    request(RUN_COMMAND, &format!("[con_id={id}] focus"))
        .and_then(|reply| reply.get(0)?.get("success")?.as_bool())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{nearest, views};
    use serde_json::json;

    fn sample() -> serde_json::Value {
        json!({
            "id": 1, "type": "root", "nodes": [{
                "id": 2, "type": "output", "name": "DP-1", "nodes": [{
                    "id": 3, "type": "workspace", "name": "1",
                    "nodes": [
                        {"id": 10, "type": "con", "pid": 500, "app_id": "foot", "focused": false},
                        {"id": 11, "type": "con", "pid": 900, "app_id": "firefox", "focused": true}
                    ],
                    "floating_nodes": [
                        {"id": 12, "type": "floating_con", "pid": 700, "app_id": "kitty", "focused": false}
                    ]
                }]
            }]
        })
    }

    #[test]
    fn reads_every_view_with_its_pid_including_floating_ones() {
        let mut out = Vec::new();
        views(&sample(), &mut out);
        let ids: Vec<(i64, u32, bool)> = out.iter().map(|v| (v.id, v.pid, v.focused)).collect();
        assert_eq!(ids, vec![(10, 500, false), (11, 900, true), (12, 700, false)]);
    }

    #[test]
    fn speaks_the_i3_ipc_framing_to_the_sway_socket() {
        use std::io::{Read, Write};
        let path = std::env::temp_dir().join(format!("codenotch-sway-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut header = [0u8; 14];
            stream.read_exact(&mut header).unwrap();
            let length = u32::from_ne_bytes(header[6..10].try_into().unwrap()) as usize;
            let kind = u32::from_ne_bytes(header[10..14].try_into().unwrap());
            let mut payload = vec![0; length];
            stream.read_exact(&mut payload).unwrap();
            let body = sample().to_string();
            let mut reply = super::MAGIC.to_vec();
            reply.extend((body.len() as u32).to_ne_bytes());
            reply.extend(kind.to_ne_bytes());
            reply.extend(body.as_bytes());
            stream.write_all(&reply).unwrap();
            (&header[..6] == super::MAGIC, kind, payload.len())
        });
        std::env::set_var("SWAYSOCK", &path);
        assert!(super::available());
        assert_eq!(super::active_pid(), Some(900));
        assert_eq!(server.join().unwrap(), (true, super::GET_TREE, 0));
        std::env::remove_var("SWAYSOCK");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn focuses_the_view_nearest_the_session() {
        let mut out = Vec::new();
        views(&sample(), &mut out);
        assert_eq!(nearest(&out, &[42, 700, 500]), Some(12));
        assert_eq!(nearest(&out, &[42, 500]), Some(10));
        assert_eq!(nearest(&out, &[42, 43]), None);
    }
}
