use std::collections::HashMap;
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

use super::systemd_repart::{CandidateDevice, EmptyMode, ProgressPhase, RepartError, RunOutput};

const GIGA_BYTES: i64 = 1024 * 1024 * 1024;
const MIN_DISK_SIZE: i64 = 20 * GIGA_BYTES;

static DEVICES: LazyLock<HashMap<String, (CandidateDevice, DryRunResponse)>> =
    LazyLock::new(|| {
        let devices = [
            (
                CandidateDevice {
                    node: "/dev/sda".to_string(),
                    model: Some("Sandmug SSD".to_string()),
                    size_bytes: Some(1024 * GIGA_BYTES),
                    ..CandidateDevice::default()
                },
                DryRunResponse::Error(RepartError::ConflictingDiskLabelPresent),
            ),
            (
                CandidateDevice {
                    node: "/dev/sdb".to_string(),
                    model: Some("OceanDoor".to_string()),
                    vendor: Some("Sharkfin HDD".to_string()),
                    size_bytes: Some(1024 * GIGA_BYTES),
                    ..CandidateDevice::default()
                },
                DryRunResponse::RunOutput(RunOutput {
                    current_size_bytes: Some(GIGA_BYTES),
                    minimal_size_bytes: Some(MIN_DISK_SIZE),
                    ..Default::default()
                }),
            ),
            (
                CandidateDevice {
                    node: "/dev/sdc".to_string(),
                    model: Some("USB stick".to_string()),
                    size_bytes: Some(GIGA_BYTES),
                    ..CandidateDevice::default()
                },
                DryRunResponse::Error(RepartError::DiskTooSmall {
                    current_size_bytes: Some(0),
                    minimal_size_bytes: Some(MIN_DISK_SIZE),
                }),
            ),
            (
                CandidateDevice {
                    node: "/dev/sdd".to_string(),
                    model: Some("Some Random Disk".to_string()),
                    size_bytes: Some(1024 * GIGA_BYTES),
                    ..CandidateDevice::default()
                },
                DryRunResponse::RunOutput(RunOutput {
                    current_size_bytes: Some(0),
                    minimal_size_bytes: Some(MIN_DISK_SIZE),
                    ..Default::default()
                }),
            ),
            (
                CandidateDevice {
                    node: "/dev/sde".to_string(),
                    model: Some("Expensive Company Disk".to_string()),
                    size_bytes: Some(1024 * GIGA_BYTES),
                    ..CandidateDevice::default()
                },
                DryRunResponse::Error(RepartError::InsufficientFreeSpace {
                    current_size_bytes: Some(1000 * GIGA_BYTES),
                    minimal_size_bytes: Some(MIN_DISK_SIZE),
                    need_free_bytes: None,
                }),
            ),
            (
                CandidateDevice {
                    node: "/dev/sdf".to_string(),
                    model: Some("The Disk".to_string()),
                    vendor: Some("Awesome Company".to_string()),
                    size_bytes: Some(1024 * GIGA_BYTES),
                    ..CandidateDevice::default()
                },
                DryRunResponse::Error(RepartError::InsufficientFreeSpace {
                    current_size_bytes: Some(1024 * GIGA_BYTES),
                    minimal_size_bytes: Some(MIN_DISK_SIZE),
                    need_free_bytes: None,
                }),
            ),
        ];

        devices
            .into_iter()
            .map(|(candidate_device, run_output)| {
                (
                    candidate_device.node.clone(),
                    (candidate_device, run_output),
                )
            })
            .collect()
    });

#[derive(Debug, Clone)]
enum DryRunResponse {
    RunOutput(RunOutput),
    Error(RepartError),
}

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
pub struct RepartDemo {
    connection: zlink::unix::Connection,
    handle: glib::JoinHandle<()>,
}

impl Drop for RepartDemo {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl RepartDemo {
    pub async fn run() -> Self {
        let (stream1, stream2) = Async::<UnixStream>::pair().unwrap();
        let listener = AnonListener::new(stream1.into());

        let service = RepartService {};
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

struct RepartService {}

impl Service<zlink::unix::Stream> for RepartService {
    type MethodCall<'de> = RepartServiceMethod<'de>;
    type ReplyParams<'ser> = RepartServiceReply;
    type ReplyStreamParams = RepartServiceReply;
    type ReplyStream = DemoStream;
    type ReplyError<'ser> = RepartError;
    type ReplyStreamError = RepartError;

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
            RepartServiceMethod::ListCandidateDevices { .. } => (
                MethodReply::Multi(DemoStream::for_list_candidate_devices()),
                fds_out,
            ),
            RepartServiceMethod::Run {
                node,
                empy,
                dry_run,
                ..
            } if method.more() && !dry_run => (MethodReply::Multi(DemoStream::for_run()), fds_out),
            RepartServiceMethod::Run { node, dry_run, .. } if *dry_run => {
                let Some(node) = node else {
                    return (
                        MethodReply::Single(Some(RepartServiceReply::RunOutput(RunOutput {
                            minimal_size_bytes: Some(MIN_DISK_SIZE),
                            ..Default::default()
                        }))),
                        fds_out,
                    );
                };
                let device = DEVICES.get(*node).expect("mocked device to exist");

                match device.1.clone() {
                    DryRunResponse::RunOutput(output) => (
                        MethodReply::Single(Some(RepartServiceReply::RunOutput(output))),
                        fds_out,
                    ),
                    DryRunResponse::Error(error) => (MethodReply::Error(error), fds_out),
                }
            }
            RepartServiceMethod::Run { .. } => {
                unreachable!("Setup never calls `run` without [more] and `dry_run = false`")
            }
        }
    }
}

// Method calls the service handles
#[derive(Debug, Deserialize)]
#[serde(tag = "method", content = "parameters")]
#[allow(unused)]
enum RepartServiceMethod<'a> {
    #[serde(rename = "io.systemd.Repart.ListCandidateDevices")]
    ListCandidateDevices {
        ignore_root: Option<bool>,
        ignore_empty: Option<bool>,
    },
    #[serde(rename = "io.systemd.Repart.Run")]
    Run {
        #[serde(default, borrow)]
        node: Option<&'a str>,
        #[serde(default)]
        empy: EmptyMode,
        #[serde(default, rename = "dryRun")]
        dry_run: bool,
        #[serde(default, borrow)]
        seed: Option<&'a str>,
        #[serde(default, borrow)]
        definitions: Vec<&'a str>,
        #[serde(default, rename = "deferPartitionsEmpty")]
        defer_partitions_empty: Option<bool>,
        #[serde(default, rename = "deferPartitionsFactoryReset")]
        defer_partitions_factory_reset: Option<bool>,
    },
}

// Reply types
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
enum RepartServiceReply {
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
            (ProgressPhase::LoadingDefinitions, 200),
            (ProgressPhase::LoadingTable, 200),
            (ProgressPhase::OpeningCopyBlockSources, 200),
            (ProgressPhase::AcquiringPartitionLabels, 200),
            (ProgressPhase::Minimizing, 200),
            (ProgressPhase::Placing, 200),
            (ProgressPhase::WipingDisk, 200),
            (ProgressPhase::WipingPartition, 200),
            (ProgressPhase::CopyingPartition, 100),
            (ProgressPhase::FormattingPartition, 200),
            (ProgressPhase::AdjustingPartition, 200),
            (ProgressPhase::WritingTable, 200),
            (ProgressPhase::RereadingTable, 200),
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
    type Item = zlink::Reply<RepartServiceReply>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.current.is_empty() {
            return Poll::Ready(None);
        };

        match self.delay.as_mut().poll(cx) {
            Poll::Ready(_) => {
                let (phase, delay) = self.current.pop().unwrap();
                self.delay.as_mut().set_after(Duration::from_millis(delay));

                // Systemd-repart only gives progress percentage for CopyPartition
                let progress = if phase == ProgressPhase::CopyingPartition {
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
                let reply = zlink::Reply::new(Some(RepartServiceReply::RunOutput(result)))
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
        let devices = DEVICES.values().map(|(device, _)| device.clone()).collect();

        Self { devices }
    }
}

impl Stream for ListCandidateDeviceProducer {
    type Item = zlink::Reply<RepartServiceReply>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(device) = self.devices.pop() else {
            return Poll::Ready(None);
        };

        let continues = Some(!self.devices.is_empty());
        let reply = zlink::Reply::new(Some(RepartServiceReply::CandidateDevice(device)))
            .set_continues(continues);
        Poll::Ready(Some(reply))
    }
}

struct DemoStream {
    producer: Pin<Box<dyn Stream<Item = zlink::Reply<RepartServiceReply>>>>,
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
        Result<zlink::Reply<RepartServiceReply>, RepartError>,
        Vec<OwnedFd>,
    );

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.producer
            .as_mut()
            .poll_next(cx)
            .map(|value| value.map(|inner| (Ok(inner), Vec::new())))
    }
}
