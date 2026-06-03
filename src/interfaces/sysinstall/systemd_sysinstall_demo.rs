use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::pin::Pin;
use std::sync::LazyLock;
use std::task::{Context, Poll};
use std::time::Duration;

use async_io::{Async, Timer};
use serde::{Deserialize, Serialize};
use zlink::connection::Connection;
use zlink::futures_util::future::{pending, Future};
use zlink::futures_util::stream::Stream;
use zlink::service::{MethodReply, Service};
use zlink::{Call, Server};

use super::systemd_sysinstall::{
    CandidateDevice, DeviceFit, ProgressPhase, RunOutput, SysInstallError,
};

const GIGA_BYTES: i64 = 1024 * 1024 * 1024;
const MIN_DISK_SIZE: i64 = 20 * GIGA_BYTES;

static DEVICES: LazyLock<Vec<CandidateDevice>> = LazyLock::new(|| {
    vec![
        CandidateDevice {
            node: Some("/dev/sda".to_string()),
            model: Some("Sandmug SSD".to_string()),
            size_bytes: Some(1024 * GIGA_BYTES),
            fit: Some(DeviceFit::ConflictingDiskLabelPresent),
            ..CandidateDevice::default()
        },
        CandidateDevice {
            node: Some("/dev/sdb".to_string()),
            model: Some("OceanDoor".to_string()),
            vendor: Some("Sharkfin HDD".to_string()),
            size_bytes: Some(1024 * GIGA_BYTES),
            current_size_bytes: Some(GIGA_BYTES),
            minimal_size_bytes: Some(MIN_DISK_SIZE),
            ..CandidateDevice::default()
        },
        CandidateDevice {
            node: Some("/dev/sdc".to_string()),
            model: Some("USB stick".to_string()),
            size_bytes: Some(GIGA_BYTES),
            current_size_bytes: Some(0),
            minimal_size_bytes: Some(MIN_DISK_SIZE),
            fit: Some(DeviceFit::DiskTooSmall),
            ..CandidateDevice::default()
        },
        CandidateDevice {
            node: Some("/dev/sdd".to_string()),
            model: Some("Some Random Disk".to_string()),
            size_bytes: Some(1024 * GIGA_BYTES),
            current_size_bytes: Some(0),
            minimal_size_bytes: Some(MIN_DISK_SIZE),
            ..CandidateDevice::default()
        },
        CandidateDevice {
            node: Some("/dev/sde".to_string()),
            model: Some("Expensive Company Disk".to_string()),
            size_bytes: Some(1024 * GIGA_BYTES),
            current_size_bytes: Some(1000 * GIGA_BYTES),
            minimal_size_bytes: Some(MIN_DISK_SIZE),
            ..CandidateDevice::default()
        },
        CandidateDevice {
            node: Some("/dev/sdf".to_string()),
            model: Some("The Disk".to_string()),
            vendor: Some("Awesome Company".to_string()),
            size_bytes: Some(1024 * GIGA_BYTES),
            current_size_bytes: Some(1024 * GIGA_BYTES),
            minimal_size_bytes: Some(MIN_DISK_SIZE),
            fit: Some(DeviceFit::InsufficientFreeSpace),
            ..CandidateDevice::default()
        },
    ]
});

#[derive(Debug)]
struct AnonListener {
    connection: Option<zlink::Connection<zlink::unix::Stream>>,
}

impl AnonListener {
    fn new(stream: zlink::unix::Stream) -> Self {
        let connection = Some(Connection::new(stream));

        Self { connection }
    }
}

impl zlink::Listener for AnonListener {
    type Socket = zlink::unix::Stream;

    async fn accept(&mut self) -> zlink::Result<zlink::Connection<Self::Socket>> {
        if let Some(connection) = self.connection.take() {
            Ok(connection)
        } else {
            // We only accept one connection
            pending().await
        }
    }
}

#[derive(Debug)]
pub struct SysInstallDemo {
    connection: zlink::unix::Connection,
    handle: glib::JoinHandle<()>,
}

impl Drop for SysInstallDemo {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl SysInstallDemo {
    pub async fn run() -> Self {
        let (stream1, stream2) = Async::<UnixStream>::pair().unwrap();
        let listener = AnonListener::new(stream1.into());

        let service = SysInstallService {};
        let server = Server::new(listener, service);

        let handle = glib::spawn_future_local(async move {
            server.run().await.unwrap();
        });
        let connection = zlink::unix::Connection::new(stream2.into());

        Self { handle, connection }
    }

    pub fn connection(&mut self) -> &mut zlink::unix::Connection {
        &mut self.connection
    }
}

struct SysInstallService {}

impl Service<zlink::unix::Stream> for SysInstallService {
    type MethodCall<'de> = SysInstallServiceMethod<'de>;
    type ReplyParams<'ser> = SysInstallServiceReply;
    type ReplyStreamParams = SysInstallServiceReply;
    type ReplyStream = DemoStream;
    type ReplyError<'ser> = SysInstallError;
    type ReplyStreamError = SysInstallError;

    async fn handle<'ser>(
        &'ser mut self,
        method: &'ser Call<Self::MethodCall<'_>>,
        _conn: &mut Connection<zlink::unix::Stream>,
        _fds: Vec<OwnedFd>,
    ) -> (
        MethodReply<Self::ReplyParams<'ser>, Self::ReplyStream, Self::ReplyError<'ser>>,
        Vec<OwnedFd>,
    ) {
        let fds_out = Vec::new();
        match method.method() {
            SysInstallServiceMethod::ListCandidateDevices { .. } => (
                MethodReply::Multi(DemoStream::for_list_candidate_devices()),
                fds_out,
            ),
            SysInstallServiceMethod::Run { node, .. } if method.more() => {
                (MethodReply::Multi(DemoStream::for_run()), fds_out)
            }
            SysInstallServiceMethod::Run { .. } => {
                unreachable!("Setup never calls `run` without [more] and `dry_run = false`")
            }
        }
    }
}

// Method calls the service handles
#[derive(Debug, Deserialize)]
#[serde(tag = "method", content = "parameters")]
#[allow(unused)]
enum SysInstallServiceMethod<'a> {
    #[serde(rename = "io.systemd.SysInstall.ListCandidateDevices")]
    ListCandidateDevices {
        #[serde(default, borrow)]
        definitions: Vec<&'a str>,
    },
    #[serde(rename = "io.systemd.SysInstall.Run")]
    Run {
        #[serde(default, borrow)]
        node: &'a str,
        #[serde(default)]
        erase: bool,
        #[serde(default, borrow)]
        definitions: Vec<&'a str>,
        #[serde(default)]
        variables: bool,
        #[serde(default)]
        copy_locale: Option<bool>,
        #[serde(default)]
        copy_keymap: Option<bool>,
        #[serde(default)]
        copy_timezone: Option<bool>,
    },
}

// Reply types
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
enum SysInstallServiceReply {
    RunOutput(RunOutput),
    CandidateDevice(CandidateDevice),
}

struct RunProducer {
    current: Vec<(ProgressPhase, u64)>,
    progress: i64,
    delay: Pin<Box<Timer>>,
}

impl RunProducer {
    fn new() -> Self {
        let mut current = vec![
            (ProgressPhase::ValidateBlockDevice, 200),
            (ProgressPhase::LoadCredentials, 200),
            (ProgressPhase::EncryptCredentials, 200),
            (ProgressPhase::InstallPartitions, 100),
            (ProgressPhase::MountPartitions, 200),
            (ProgressPhase::InstallKernel, 200),
            (ProgressPhase::InstallBootloader, 200),
            (ProgressPhase::UnmountPartitions, 200),
        ];

        current.reverse();
        Self {
            current,
            progress: -1,
            delay: Box::pin(Timer::after(Duration::ZERO)),
        }
    }
}

impl Stream for RunProducer {
    type Item = zlink::Reply<SysInstallServiceReply>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.current.is_empty() {
            return Poll::Ready(None);
        };

        match self.delay.as_mut().poll(cx) {
            Poll::Ready(_) => {
                let (phase, delay) = self.current.pop().unwrap();
                self.delay.as_mut().set_after(Duration::from_millis(delay));

                // Systemd-sysinstall only gives progress percentage for InstallPartitions
                let progress = if phase == ProgressPhase::InstallPartitions {
                    if self.progress != 100 {
                        self.current.push((phase.clone(), delay));
                    }
                    self.progress += 1;
                    Some(self.progress)
                } else {
                    None
                };

                let result = RunOutput {
                    phase: Some(phase),
                    progress,
                    ..Default::default()
                };
                let continues = Some(!self.current.is_empty());
                let reply = zlink::Reply::new(Some(SysInstallServiceReply::RunOutput(result)))
                    .set_continues(continues);
                Poll::Ready(Some(reply))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

struct ListCandidateDeviceProducer {
    devices: Vec<CandidateDevice>,
}

impl ListCandidateDeviceProducer {
    fn new() -> Self {
        let devices = DEVICES.clone();

        Self { devices }
    }
}

impl Stream for ListCandidateDeviceProducer {
    type Item = zlink::Reply<SysInstallServiceReply>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(device) = self.devices.pop() else {
            return Poll::Ready(None);
        };

        let continues = Some(!self.devices.is_empty());
        let reply = zlink::Reply::new(Some(SysInstallServiceReply::CandidateDevice(device)))
            .set_continues(continues);
        Poll::Ready(Some(reply))
    }
}

struct DemoStream {
    producer: Pin<Box<dyn Stream<Item = zlink::Reply<SysInstallServiceReply>>>>,
}

impl DemoStream {
    fn for_run() -> Self {
        Self {
            producer: Box::pin(RunProducer::new()),
        }
    }

    fn for_list_candidate_devices() -> Self {
        Self {
            producer: Box::pin(ListCandidateDeviceProducer::new()),
        }
    }
}

impl Stream for DemoStream {
    type Item = (
        Result<zlink::Reply<SysInstallServiceReply>, SysInstallError>,
        Vec<OwnedFd>,
    );

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.producer
            .as_mut()
            .poll_next(cx)
            .map(|value| value.map(|inner| (Ok(inner), Vec::new())))
    }
}
