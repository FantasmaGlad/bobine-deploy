use std::sync::Arc;

use bobine_deploy_engine::{execute, Sink};
use bobine_deploy_proto::{Command, Error, ErrorKind, Event, CATALOG, PROTOCOL_VERSION};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

#[cfg(test)]
pub const META_METHODS: &[&str] = &["describe"];

#[cfg(test)]
pub fn methods() -> Vec<String> {
    CATALOG
        .iter()
        .map(|c| c.name.to_string())
        .chain(META_METHODS.iter().map(|m| m.to_string()))
        .collect()
}

struct Notifier {
    id: Value,
    out: mpsc::UnboundedSender<Value>,
}

impl Sink for Notifier {
    fn event(&self, event: Event) {
        let _ = self.out.send(json!({
            "jsonrpc": "2.0",
            "method": "event",
            "params": { "id": self.id, "event": event },
        }));
    }
}

pub fn main() -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("bobine-deploy rpc : démarrage impossible : {e}");
            return 1;
        }
    };
    runtime.block_on(serve(tokio::io::stdin(), tokio::io::stdout()));
    0
}

pub async fn serve<R, W>(input: R, mut output: W)
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (tx, mut rx) = mpsc::unbounded_channel::<Value>();
    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let mut line = message.to_string();
            line.push('\n');
            if output.write_all(line.as_bytes()).await.is_err() {
                break;
            }
            let _ = output.flush().await;
        }
    });

    let mut lines = BufReader::new(input).lines();
    let mut tasks = tokio::task::JoinSet::new();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let tx = tx.clone();
        tasks.spawn(handle(line, Arc::new(tx)));
    }
    while tasks.join_next().await.is_some() {}
    drop(tx);
    let _ = writer.await;
}

async fn handle(line: String, out: Arc<mpsc::UnboundedSender<Value>>) {
    let request: Value = match serde_json::from_str(&line) {
        Ok(value) => value,
        Err(e) => {
            let _ = out.send(error_response(Value::Null, -32700, &format!("JSON illisible : {e}")));
            return;
        }
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        let _ = out.send(error_response(
            id,
            -32600,
            "Requête invalide : champ « method » absent.",
        ));
        return;
    };
    if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        let _ = out.send(error_response(
            id,
            -32600,
            "Requête invalide : « jsonrpc » doit valoir « 2.0 ».",
        ));
        return;
    }
    let reply = match method {
        "describe" => Ok(json!({
            "protocol": PROTOCOL_VERSION,
            "version": bobine_deploy_engine::app_info().version,
            "methods": CATALOG.iter().map(|c| json!({ "name": c.name, "summary": c.summary })).collect::<Vec<_>>(),
        })),
        name if CATALOG.iter().any(|c| c.name == name) => {
            run(name, request.get("params").cloned(), id.clone(), out.clone()).await
        }
        other => Err((-32601, format!("Méthode inconnue : {other}."))),
    };
    let _ = out.send(match reply {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => error_response(id, code, &message),
    });
}

async fn run(
    method: &str,
    params: Option<Value>,
    id: Value,
    out: Arc<mpsc::UnboundedSender<Value>>,
) -> Result<Value, (i64, String)> {
    let mut object = match params {
        None | Some(Value::Null) => serde_json::Map::new(),
        Some(Value::Object(map)) => map,
        Some(_) => return Err((-32602, "Paramètres invalides : un objet JSON est attendu.".to_string())),
    };
    object.insert("command".to_string(), Value::String(method.to_string()));
    let command: Command = serde_json::from_value(Value::Object(object))
        .map_err(|e| (-32602, format!("Paramètres invalides pour « {method} » : {e}")))?;
    if let Command::Install(request) = &command {
        if request.script_path.is_some() {
            return Err((
                -32602,
                "« script_path » est réservé à la ligne de commande.".to_string(),
            ));
        }
    }
    let notifier = Notifier {
        id,
        out: (*out).clone(),
    };
    execute(command, &notifier)
        .await
        .map(|output| serde_json::to_value(output).unwrap_or(Value::Null))
        .map_err(|e: Error| (error_code(e.kind), e.message))
}

fn error_code(kind: ErrorKind) -> i64 {
    match kind {
        ErrorKind::Unreachable => -32001,
        ErrorKind::Refused => -32002,
        ErrorKind::Usage => -32602,
        ErrorKind::Failed => -32000,
    }
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests;
