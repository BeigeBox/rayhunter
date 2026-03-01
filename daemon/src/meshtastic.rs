use std::time::Duration;

use log::{error, info, warn};
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_serial::SerialPortBuilderExt;
use tokio_util::task::TaskTracker;

use crate::notifications::{Notification, NotificationType};

const SERIAL_BAUD: u32 = 115_200;
const FRAME_MAGIC: [u8; 2] = [0x94, 0xc3];
const MAX_PAYLOAD: usize = 512;

// Meshtastic protobuf types, hand-annotated from meshtastic/mesh.proto.
// Only the fields needed to send and receive text messages are included.

#[derive(Clone, PartialEq, Message)]
pub struct ToRadio {
    #[prost(message, optional, tag = "1")]
    pub packet: Option<MeshPacket>,
    #[prost(uint32, tag = "3")]
    pub want_config_id: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct FromRadio {
    #[prost(uint32, tag = "1")]
    pub id: u32,
    #[prost(message, optional, tag = "2")]
    pub packet: Option<MeshPacket>,
    #[prost(message, optional, tag = "3")]
    pub my_info: Option<MyNodeInfo>,
    #[prost(message, optional, tag = "5")]
    pub config: Option<Config>,
    #[prost(uint32, tag = "7")]
    pub config_complete_id: u32,
    #[prost(message, optional, tag = "10")]
    pub channel: Option<Channel>,
    #[prost(message, optional, tag = "13")]
    pub metadata: Option<DeviceMetadata>,
}

#[derive(Clone, PartialEq, Message)]
pub struct DeviceMetadata {
    #[prost(string, tag = "1")]
    pub firmware_version: String,
    #[prost(bool, tag = "9")]
    pub has_pkc: bool,
}

#[derive(Clone, PartialEq, Message)]
pub struct Config {
    #[prost(message, optional, tag = "6")]
    pub lora: Option<LoRaConfig>,
}

#[derive(Clone, PartialEq, Message)]
pub struct LoRaConfig {
    #[prost(bool, tag = "1")]
    pub use_preset: bool,
    #[prost(enumeration = "ModemPreset", tag = "2")]
    pub modem_preset: i32,
    #[prost(enumeration = "RegionCode", tag = "7")]
    pub region: i32,
    #[prost(uint32, tag = "8")]
    pub hop_limit: u32,
    #[prost(bool, tag = "9")]
    pub tx_enabled: bool,
    #[prost(int32, tag = "10")]
    pub tx_power: i32,
    #[prost(uint32, tag = "11")]
    pub channel_num: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum ModemPreset {
    LongFast = 0,
    LongSlow = 1,
    VeryLongSlow = 2,
    MediumSlow = 3,
    MediumFast = 4,
    ShortSlow = 5,
    ShortFast = 6,
    LongModerate = 7,
    ShortTurbo = 8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum RegionCode {
    Unset = 0,
    Us = 1,
    Eu433 = 2,
    Eu868 = 3,
    Cn = 4,
    Jp = 5,
    Anz = 6,
    Kr = 7,
    Tw = 8,
    Ru = 9,
    In = 10,
    Nz865 = 11,
    Th = 12,
    Lora24 = 13,
    Ua433 = 14,
    Ua868 = 15,
    My433 = 16,
    My919 = 17,
    Sg923 = 18,
}

#[derive(Clone, PartialEq, Message)]
pub struct Channel {
    #[prost(uint32, tag = "1")]
    pub index: u32,
    #[prost(message, optional, tag = "2")]
    pub settings: Option<ChannelSettings>,
    #[prost(enumeration = "ChannelRole", tag = "3")]
    pub role: i32,
}

#[derive(Clone, PartialEq, Message)]
pub struct ChannelSettings {
    #[prost(string, tag = "3")]
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum ChannelRole {
    Disabled = 0,
    Primary = 1,
    Secondary = 2,
}

#[derive(Clone, PartialEq, Message)]
pub struct MyNodeInfo {
    #[prost(uint32, tag = "1")]
    pub my_node_num: u32,
    #[prost(string, tag = "2")]
    pub firmware_version: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct MeshPacket {
    #[prost(fixed32, tag = "1")]
    pub from: u32,
    #[prost(fixed32, tag = "2")]
    pub to: u32,
    #[prost(uint32, tag = "3")]
    pub channel: u32,
    #[prost(message, optional, tag = "4")]
    pub decoded: Option<Data>,
    #[prost(fixed32, tag = "6")]
    pub id: u32,
    #[prost(uint32, tag = "9")]
    pub hop_limit: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct Data {
    #[prost(enumeration = "PortNum", tag = "1")]
    pub portnum: i32,
    #[prost(bytes = "vec", tag = "2")]
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum PortNum {
    UnknownApp = 0,
    TextMessageApp = 1,
}

fn build_text_packet(text: &str) -> Vec<u8> {
    let data = Data {
        portnum: PortNum::TextMessageApp as i32,
        payload: text.as_bytes().to_vec(),
    };
    let radio = ToRadio {
        packet: Some(MeshPacket {
            to: 0xFFFF_FFFF,
            channel: 0,
            decoded: Some(data),
            hop_limit: 3,
            ..MeshPacket::default()
        }),
        ..ToRadio::default()
    };
    radio.encode_to_vec()
}

fn build_want_config() -> Vec<u8> {
    let radio = ToRadio {
        want_config_id: 0xDEAD,
        ..ToRadio::default()
    };
    radio.encode_to_vec()
}

fn frame_packet(payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u16;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&FRAME_MAGIC);
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

fn parse_from_radio(data: &[u8]) -> String {
    match FromRadio::decode(data) {
        Ok(msg) => {
            if msg.config_complete_id != 0 {
                return format!("config_complete (id=0x{:X})", msg.config_complete_id);
            }
            if let Some(info) = &msg.my_info {
                return format!(
                    "my_info (node=0x{:X}, fw={})",
                    info.my_node_num, info.firmware_version
                );
            }
            if let Some(config) = &msg.config
                && let Some(lora) = &config.lora
            {
                let preset = ModemPreset::try_from(lora.modem_preset)
                    .map(|p| format!("{p:?}"))
                    .unwrap_or_else(|_| format!("unknown({})", lora.modem_preset));
                let region = RegionCode::try_from(lora.region)
                    .map(|r| format!("{r:?}"))
                    .unwrap_or_else(|_| format!("unknown({})", lora.region));
                return format!(
                    "config_lora (preset={preset}, region={region}, hop_limit={}, tx_enabled={}, tx_power={}, channel_num={})",
                    lora.hop_limit, lora.tx_enabled, lora.tx_power, lora.channel_num
                );
            }
            if let Some(channel) = &msg.channel {
                let role = ChannelRole::try_from(channel.role)
                    .map(|r| format!("{r:?}"))
                    .unwrap_or_else(|_| format!("unknown({})", channel.role));
                let name = channel.settings.as_ref().map_or("", |s| &s.name);
                return format!(
                    "channel (index={}, role={role}, name={name:?})",
                    channel.index
                );
            }
            if let Some(meta) = &msg.metadata {
                return format!(
                    "metadata (fw={}, has_pkc={})",
                    meta.firmware_version, meta.has_pkc
                );
            }
            if let Some(pkt) = &msg.packet {
                let text = pkt
                    .decoded
                    .as_ref()
                    .filter(|d| d.portnum == PortNum::TextMessageApp as i32)
                    .map(|d| String::from_utf8_lossy(&d.payload).to_string());
                if let Some(text) = text {
                    return format!(
                        "packet (from=0x{:X}, to=0x{:X}, text={:?})",
                        pkt.from, pkt.to, text
                    );
                }
                return format!(
                    "packet (from=0x{:X}, to=0x{:X}, port={})",
                    pkt.from,
                    pkt.to,
                    pkt.decoded.as_ref().map_or(0, |d| d.portnum)
                );
            }
            if msg.id != 0 {
                return format!("from_radio (id={})", msg.id);
            }
            format!("from_radio ({} bytes, unrecognized)", data.len())
        }
        Err(e) => {
            let hex: String = data
                .iter()
                .take(16)
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "from_radio ({} bytes, parse error: {e}, hex: {hex})",
                data.len()
            )
        }
    }
}

pub struct MeshtasticService {
    serial_port: String,
    tx: mpsc::Sender<Notification>,
    rx: mpsc::Receiver<Notification>,
}

impl MeshtasticService {
    pub fn new(serial_port: String) -> Self {
        let (tx, rx) = mpsc::channel(10);
        Self {
            serial_port,
            tx,
            rx,
        }
    }

    pub fn new_handler(&self) -> mpsc::Sender<Notification> {
        self.tx.clone()
    }
}

pub fn run_meshtastic_worker(
    task_tracker: &TaskTracker,
    mut service: MeshtasticService,
    enabled_notifications: Vec<NotificationType>,
) {
    task_tracker.spawn(async move {
        info!("Meshtastic worker starting on {}", service.serial_port);

        // Retry opening the serial port — the USB device may not be
        // available yet if the hub is plugged in after boot.
        let port = loop {
            match tokio_serial::new(&service.serial_port, SERIAL_BAUD).open_native_async() {
                Ok(p) => break p,
                Err(e) => {
                    warn!(
                        "Meshtastic serial port {} not available: {e}, retrying in 10s",
                        service.serial_port
                    );
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        };

        info!("Meshtastic serial port opened");
        let (mut reader, mut writer) = tokio::io::split(port);

        // Request config to establish connection with the radio
        let config_frame = frame_packet(&build_want_config());
        if let Err(e) = writer.write_all(&config_frame).await {
            error!("Meshtastic config request failed: {e}");
            return;
        }

        // Read responses from the radio in a background task
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let mut accum = Vec::new();

            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => {
                        info!("Meshtastic serial port closed");
                        return;
                    }
                    Ok(n) => {
                        accum.extend_from_slice(&buf[..n]);

                        // Parse all complete frames from the buffer
                        while accum.len() >= 4 {
                            if accum[0] != FRAME_MAGIC[0] || accum[1] != FRAME_MAGIC[1] {
                                // Skip until we find magic bytes
                                if let Some(pos) = accum.windows(2).position(|w| w == FRAME_MAGIC) {
                                    accum.drain(..pos);
                                } else {
                                    accum.clear();
                                }
                                continue;
                            }
                            let payload_len = u16::from_be_bytes([accum[2], accum[3]]) as usize;
                            if payload_len > MAX_PAYLOAD {
                                accum.drain(..2);
                                continue;
                            }
                            if accum.len() < 4 + payload_len {
                                break; // need more data
                            }
                            let payload = accum[4..4 + payload_len].to_vec();
                            accum.drain(..4 + payload_len);

                            let desc = parse_from_radio(&payload);
                            info!("Meshtastic rx: {desc}");
                        }
                    }
                    Err(e) => {
                        error!("Meshtastic serial read error: {e}");
                        return;
                    }
                }
            }
        });

        // Wait for config exchange to complete, then announce on the mesh
        tokio::time::sleep(Duration::from_secs(2)).await;

        let announce = build_text_packet("Rayhunter online");
        let frame = frame_packet(&announce);
        if let Err(e) = writer.write_all(&frame).await {
            error!("Meshtastic announce failed: {e}");
        } else {
            info!("Meshtastic tx: announce sent");
        }

        loop {
            let notification = match service.rx.recv().await {
                Some(n) => n,
                None => return,
            };

            if !enabled_notifications.contains(&notification.notification_type) {
                continue;
            }

            let text = if notification.message.len() > 200 {
                &notification.message[..200]
            } else {
                &notification.message
            };
            let payload = build_text_packet(text);
            if payload.len() > MAX_PAYLOAD {
                warn!(
                    "Meshtastic payload too large ({} bytes), skipping",
                    payload.len()
                );
                continue;
            }

            let frame = frame_packet(&payload);
            if let Err(e) = writer.write_all(&frame).await {
                error!("Meshtastic serial write failed: {e}");
                tokio::time::sleep(Duration::from_secs(5)).await;
            } else {
                info!("Meshtastic tx: {}", text);
            }
        }
    });
}
