//! TESTCore `Client/ZeroMqClient.cs` 이식: ZeroMQ REQ 소켓으로 서비스에 `pause`·`resume`을 요청한다.
//!
//! 원본과 같게 맞춘 동작:
//!
//! - `tcp://{ip}:{port}`에 연결해 `{serviceType}.pause`(또는 `.resume`) 프레임 하나를 보내고 응답 프레임을
//!   문자열로 받는다. 응답이 `OK!`이면 `0`, 아니면 `-1`을 돌려준다.
//! - 로그: 성공은 `Paused|Resumed {serviceType} on {ip}:{port}`(정보), 실패는
//!   `Error pausing|resuming {serviceType} on {ip}:{port} => {response}`(오류).
//! - 서버에 연결할 수 없거나 응답이 없으면 연결·응답을 기다리며 끝나지 않는다(NetMQ와 같다.
//!   `zeromq` 크레이트도 연결을 계속 다시 시도한다). 호출자가 취소(퓨처 drop)해야 한다.
//!
//! NetMQ 서버와 통신하려면 REQ 소켓이 핸드셰이크에 `Identity`(임의 식별자)를 실어야 한다. `zeromq` 크레이트는 기본으로
//! 보내지 않아 NetMQ REP가 죽으므로 항상 지정한다(`tests/parity/zeromq.rs`의 `interop_*`로 확인).
//!
//! 다른 점: NetMQ가 던지는 예외(주소 형식 오류 등) 대신 [`ZeroMqError`]를 돌려주며 메시지가 다르다.
//! 응답이 여러 프레임이면 `ReceiveFrameString`처럼 첫 프레임만 쓴다.

use zeromq::util::PeerIdentity;
use zeromq::{ReqSocket, Socket, SocketOptions, SocketRecv, SocketSend};

/// ZeroMQ 소켓 오류(주소 형식, 연결 실패, 전송 실패 등). 원본에서는 NetMQ 예외로 나타난다.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ZeroMqError(#[from] zeromq::ZmqError);

impl ZeroMqError {
    /// 원본에서 던지던 예외 계열의 형식 이름.
    pub fn dotnet_type(&self) -> &'static str {
        "NetMQ.NetMQException"
    }
}

/// 지정한 서비스에 ZeroMQ 요청을 보내 일시정지시킨다. 성공 시 0, 실패 시 -1.
pub async fn pause(service_type: &str, ip: &str, port: i32) -> Result<i32, ZeroMqError> {
    send_command(service_type, ip, port, "pause", "pausing", "Paused").await
}

/// 지정한 서비스에 ZeroMQ 요청을 보내 일시정지를 해제한다. 성공 시 0, 실패 시 -1.
pub async fn resume(service_type: &str, ip: &str, port: i32) -> Result<i32, ZeroMqError> {
    send_command(service_type, ip, port, "resume", "resuming", "Resumed").await
}

async fn send_command(
    service_type: &str,
    ip: &str,
    port: i32,
    command: &str,
    error_verb: &str,
    done_verb: &str,
) -> Result<i32, ZeroMqError> {
    // NetMQ의 REP 서버는 연결 핸드셰이크의 `Identity` 속성이 없으면 `NullReferenceException`으로 죽는다.
    // libzmq·NetMQ 클라이언트는 항상 보내므로 `zeromq` 크레이트도 식별자를 보내도록 지정한다.
    let mut options = SocketOptions::default();
    options.peer_identity(PeerIdentity::new());
    let mut client = ReqSocket::with_options(options);
    client.connect(&format!("tcp://{ip}:{port}")).await?;
    client
        .send(format!("{service_type}.{command}").into())
        .await?;
    let reply = client.recv().await?;
    let response = reply
        .get(0)
        .map(|frame| String::from_utf8_lossy(frame).into_owned())
        .unwrap_or_default();
    if response != "OK!" {
        tracing::error!("Error {error_verb} {service_type} on {ip}:{port} => {response}");
        return Ok(-1);
    }
    tracing::info!("{done_verb} {service_type} on {ip}:{port}");
    Ok(0)
}
