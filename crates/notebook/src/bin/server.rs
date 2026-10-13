use notebook_core::{configure_network, Store};
use serde_json::{json, Value};
use std::{fs, io::Read, path::PathBuf};
use tiny_http::{Header, Method, Response, Server, StatusCode};
fn main() {
    let root = std::env::var("QINGJIAN_DATA_DIR").unwrap_or(".data".into());
    let addr = std::env::var("QINGJIAN_DEV_ADDR").unwrap_or("127.0.0.1:3415".into());
    if !addr.starts_with("127.0.0.1:") {
        panic!("开发服务只能监听本机地址")
    };
    let mut store = Store::new(&PathBuf::from(root)).expect("无法打开笔记目录");
    configure_network(&store.root).expect("无法配置安全连接");
    let server = Server::http(&addr).expect("无法启动本地服务");
    println!("轻笺已启动：http://{addr}");
    for mut request in server.incoming_requests() {
        let path = request.url().split('?').next().unwrap_or("/").to_owned();
        if let Some(action) = path.strip_prefix("/api/") {
            let origin = request
                .headers()
                .iter()
                .find(|h| h.field.equiv("Origin"))
                .map(|h| h.value.as_str());
            let content_type = request
                .headers()
                .iter()
                .find(|h| h.field.equiv("Content-Type"))
                .map(|h| h.value.as_str())
                .unwrap_or("");
            if request.method() != &Method::Post
                || !content_type.starts_with("application/json")
                || origin
                    .map(|o| o != format!("http://{addr}"))
                    .unwrap_or(false)
            {
                let _ = request.respond(
                    Response::from_string("请求来源或格式不允许").with_status_code(StatusCode(403)),
                );
                continue;
            };
            let mut data = vec![];
            let result = request
                .as_reader()
                .take((256 << 20) + 1)
                .read_to_end(&mut data);
            let response = if result.is_err() || data.len() > 256 << 20 {
                json!({"error":"请求超过大小限制"})
            } else {
                match serde_json::from_slice::<Value>(&data) {
                    Ok(q) => match store.dispatch(action, q) {
                        Ok(v) => json!({"data":v}),
                        Err(e) => json!({"error":e}),
                    },
                    Err(_) => json!({"error":"请求格式无效"}),
                }
            };
            let response = Response::from_string(response.to_string())
                .with_header(
                    Header::from_bytes("Content-Type", "application/json; charset=utf-8").unwrap(),
                )
                .with_header(Header::from_bytes("Cache-Control", "no-store").unwrap());
            let _ = request.respond(response);
            continue;
        };
        if request.method() != &Method::Get {
            let _ = request.respond(Response::empty(StatusCode(405)));
            continue;
        };
        let rel = if path == "/" {
            "index.html"
        } else {
            path.trim_start_matches('/')
        };
        if rel.contains("..") || rel.contains('\\') {
            let _ = request.respond(Response::empty(StatusCode(404)));
            continue;
        };
        let file = PathBuf::from("dist").join(rel);
        let mime = match file.extension().and_then(|e| e.to_str()) {
            Some("html") => "text/html; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
            Some("svg") => "image/svg+xml",
            _ => "application/octet-stream",
        };
        match fs::read(&file) {
            Ok(b) => {
                let _ = request.respond(
                    Response::from_data(b)
                        .with_header(Header::from_bytes("Content-Type", mime).unwrap())
                        .with_header(
                            Header::from_bytes("X-Content-Type-Options", "nosniff").unwrap(),
                        ),
                );
            }
            Err(_) => {
                let _ = request.respond(Response::empty(StatusCode(404)));
            }
        }
    }
}
