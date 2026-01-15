use std::collections::HashMap;
use std::pin::Pin;
use std::sync::LazyLock;
use std::task::{Context, Poll};

use futures_util::future::{pending, FutureExt, RemoteHandle};
use futures_util::{Future, Stream};
use serde::{Deserialize, Serialize};
use tokio::runtime::Builder;
use tokio::sync::oneshot;
use tokio::time::{Duration, Instant};
use zlink::connection::{Connection, Socket};
use zlink::service::{MethodReply, Service};
use zlink::{Call, Server};

use crate::interfaces::systemd_repart::{CandidateDevice, RunOutput};
use crate::interfaces::{EmptyMode, ProgressPhase, RepartError};

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
    #[allow(dead_code)]
    handle: RemoteHandle<Result<(), zlink::Error>>,
    connection: zlink::unix::Connection,
}

impl RepartDemo {
    pub async fn run() -> Self {
        let (tx, rx) = oneshot::channel();
        let rt = Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .expect("Unable to create tokio runtime");

        std::thread::spawn(move || {
            rt.block_on(async move {
                let (stream1, stream2) = tokio::net::UnixStream::pair().unwrap();
                let listener = AnonListener::new(stream1.into());

                let service = RepartService {};
                let server = Server::new(listener, service);

                let (fut, handle) = server.run().remote_handle();
                let connection = zlink::unix::Connection::new(stream2.into());

                tx.send(Self { handle, connection }).unwrap();

                fut.await;
            })
        });

        rx.await.unwrap()
    }

    pub fn connection(&mut self) -> &mut zlink::unix::Connection {
        &mut self.connection
    }
}

struct RepartService {}

impl Service for RepartService {
    type MethodCall<'de> = RepartServiceMethod<'de>;
    type ReplyParams<'ser> = RepartServiceReply;
    type ReplyStreamParams = RepartServiceReply;
    type ReplyStream = DemoStream;
    type ReplyError<'ser> = RepartError;

    async fn handle<'service, Sock: Socket>(
        &'service mut self,
        call: &'service Call<Self::MethodCall<'_>>,
        _conn: &mut Connection<Sock>,
    ) -> MethodReply<Self::ReplyParams<'service>, Self::ReplyStream, Self::ReplyError<'service>>
    {
        match call.method() {
            RepartServiceMethod::ListCandidateDevices { .. } => {
                MethodReply::Multi(DemoStream::for_list_candidate_devices())
            }
            RepartServiceMethod::Run {
                node,
                empy,
                dry_run,
                ..
            } if call.more() && !dry_run => MethodReply::Multi(DemoStream::for_run()),
            RepartServiceMethod::Run { node, dry_run, .. } if *dry_run => {
                let Some(node) = node else {
                    return MethodReply::Single(Some(RepartServiceReply::RunOutput(RunOutput {
                        minimal_size_bytes: Some(MIN_DISK_SIZE),
                        ..Default::default()
                    })));
                };
                let device = DEVICES.get(*node).expect("mocked device to exist");

                match device.1.clone() {
                    DryRunResponse::RunOutput(output) => {
                        MethodReply::Single(Some(RepartServiceReply::RunOutput(output)))
                    }
                    DryRunResponse::Error(error) => MethodReply::Error(error),
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
    delay: Pin<Box<tokio::time::Sleep>>,
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
            delay: Box::pin(tokio::time::sleep(Duration::ZERO)),
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
                self.delay.as_mut().reset(Instant::now() + Duration::from_millis(delay));

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
    type Item = zlink::Reply<RepartServiceReply>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.producer.as_mut().poll_next(cx)
    }
}
