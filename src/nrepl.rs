use std::io::{prelude::*, BufReader};
use std::net::{TcpListener, TcpStream, IpAddr};
use std::thread;
use std::sync::mpsc;
use crate::{VMMessage, ReplCommand, ReplResponse};
use std::time::Duration;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "op", rename_all = "kebab-case")]
enum NReplMessage {
    AddMiddleware,
    Clone { id: u64, session: Option<u64> },
    Close { session: u64 },
    Completions,
    Describe,
    Eval { code: String, session: u64, column: Option<u64>, file: Option<String>, id: u64, line: Option<u64> },
    Interrupt,
    LoadFile,
    Lookup,
    LsMiddleware,
    LsSessions,
    SideloaderProvide,
    SideloaderStart,
    Stdin,
    SwapMiddleware,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
struct CloneResponse {
    id: u64,
    new_session: u64,
    status: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
struct EvalResponse {
    id: u64,
    session: u64,
    value: String,
    status: Vec<String>,
    ns: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
struct LsSessionsResponse {
    sessions: Vec<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
struct DescribeResponse {
    aux: Vec<String>,
    ops: Vec<String>,
}

pub fn nrepl_thread(tx: mpsc::Sender<VMMessage>, whitelist: Vec<String>) {
    let listener = TcpListener::bind("127.0.0.1:7888").unwrap();

    let whitelist: Vec<IpAddr> = whitelist.iter()
        .map(|addr| addr.parse().unwrap())
        .collect();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        if whitelist.contains(&stream.peer_addr().unwrap().ip()) {
            stream.set_read_timeout(Some(Duration::from_millis(200))).unwrap();

            let stream_tx = tx.clone();
            thread::spawn(|| {
                handle_nrepl_connection(stream, stream_tx);
            });
        }
    }
}

fn handle_nrepl_connection(mut stream: TcpStream, tx: mpsc::Sender<VMMessage>) {
    let mut out_stream = stream.try_clone().unwrap();
    let mut buf_reader = BufReader::new(&mut stream);

    let mut sessions: Vec<u64> = Vec::new();

    loop {
        let mut line = String::new();
        let _ = buf_reader.read_to_string(&mut line);

        if line.len() > 0 {
            let mut b = line.as_bytes();
            let data: NReplMessage = bt_bencode::from_slice(&mut b).unwrap();

            let encoded_data: Vec<u8> = match data {
                NReplMessage::Clone { session: _, id } => {
                    let new_session = match sessions.last() {
                        Some(n) => n+1,
                        None => 1,
                    };
                    sessions.push(new_session);

                    let response = CloneResponse { id, new_session, status: vec!["done".into()] };
                    bt_bencode::to_vec(&response).unwrap()
                },
                NReplMessage::Eval { session, id, code, .. } => {
                    if sessions.contains(&session) {
                        let (repl_cmd, resp_rx) = ReplCommand::create(code);
                        tx.send(VMMessage::Command(repl_cmd)).unwrap();

                        let value = match resp_rx.recv().unwrap() {
                            ReplResponse::Empty => "()".into(),
                            ReplResponse::Return(s) => s,
                            ReplResponse::Error(_) => "Error".into(),
                        };

                        let response = EvalResponse { id, session, value, ns: "ns".into(), status: vec!["done".into()] };
                        bt_bencode::to_vec(&response).unwrap()
                    }
                    else {
                        vec![]
                    }
                },
                NReplMessage::Describe => {
                    let capabilities = DescribeResponse {
                        ops: vec![
                            "clone".into(), 
                            "eval".into(), 
                            "describe".into(), 
                            "ls-sessions".into(),
                            "close".into(),
                        ],
                        aux: vec![],
                    };
                    bt_bencode::to_vec(&capabilities).unwrap()
                },
                NReplMessage::LsSessions => {
                    let response = LsSessionsResponse { sessions: sessions.clone() };
                    bt_bencode::to_vec(&response).unwrap()
                },
                NReplMessage::Close { session } => {
                    if let Ok(i) = sessions.binary_search(&session) {
                        sessions.remove(i);
                    }
                    vec![]
                }
                _ => vec![],
            };

            let _ = out_stream.write_all(&encoded_data);
        }
    }
}
