use super::*;

async fn exchange(requests: &str) -> Vec<Value> {
    let (client, server) = tokio::io::duplex(1 << 20);
    let reader = tokio::spawn(async move {
        let mut lines = BufReader::new(client).lines();
        let mut all = Vec::new();
        while let Ok(Some(line)) = lines.next_line().await {
            all.push(serde_json::from_str::<Value>(&line).unwrap());
        }
        all
    });
    serve(requests.as_bytes(), server).await;
    reader.await.unwrap()
}

#[tokio::test]
async fn describe_version_and_errors() {
    let replies = exchange(concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"describe"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"version"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":3,"method":"inconnue"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":4,"method":"host-key","params":{"port":22}}"#,
        "\n",
        "pas du json\n",
    ))
    .await;
    let by_id = |id: i64| replies.iter().find(|r| r["id"] == json!(id)).unwrap().clone();
    assert_eq!(by_id(1)["result"]["protocol"], json!(PROTOCOL_VERSION));
    assert_eq!(by_id(1)["result"]["methods"].as_array().unwrap().len(), CATALOG.len());
    assert_eq!(by_id(2)["result"]["result"], "version");
    assert_eq!(by_id(3)["error"]["code"], -32601);
    assert_eq!(by_id(4)["error"]["code"], -32602);
    assert!(replies.iter().any(|r| r["error"]["code"] == -32700));
}

#[tokio::test]
async fn a_simulated_installation_streams_events_then_answers() {
    let replies = exchange(concat!(
        r#"{"jsonrpc":"2.0","id":"i","method":"install","params":{"credentials":{"host":"203.0.113.10","username":"admin"},"simulate":true}}"#,
        "\n",
    ))
    .await;
    let events: Vec<&Value> = replies.iter().filter(|r| r["method"] == "event").collect();
    assert!(events.iter().all(|e| e["params"]["id"] == "i"));
    assert!(events
        .iter()
        .any(|e| e["params"]["event"]["event"] == "install-progress"));
    let answer = replies.iter().find(|r| r["id"] == "i").unwrap();
    assert_eq!(answer["result"]["result"], "install");
}

#[tokio::test]
async fn a_remote_driver_cannot_run_an_arbitrary_target_script() {
    let replies = exchange(concat!(
        r#"{"jsonrpc":"2.0","id":5,"method":"install","params":{"credentials":{"host":"h","username":"u"},"script_path":"/tmp/x"}}"#,
        "\n",
    ))
    .await;
    assert_eq!(replies[0]["error"]["code"], -32602);
}
