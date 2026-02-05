//! QMDL Replay Device for testing without hardware.
//!
//! This module provides a way to replay captured QMDL files through the
//! daemon's analysis pipeline, enabling device-free testing on any platform.

use futures::TryStream;
use log::info;
use rayhunter::diag::MessagesContainer;
use rayhunter::qmdl::QmdlReader;
use tokio::fs::File;

/// A device that replays QMDL data from a file instead of reading from /dev/diag.
/// This allows testing the full analysis pipeline without Qualcomm hardware.
pub struct QmdlReplayDevice {
    reader: QmdlReader<File>,
    #[allow(dead_code)]
    speed: f32, // Reserved for future realtime pacing support
}

impl QmdlReplayDevice {
    /// Create a new QmdlReplayDevice from a QMDL file path.
    ///
    /// # Arguments
    /// * `path` - Path to the QMDL file to replay
    /// * `speed` - Replay speed multiplier (0 = fast as possible, currently unused)
    pub async fn new(path: &str, speed: f32) -> std::io::Result<Self> {
        info!("Opening QMDL file for replay: {}", path);
        let file = File::open(path).await?;
        let file_size = file.metadata().await?.len();
        info!("QMDL file size: {} bytes", file_size);
        Ok(Self {
            reader: QmdlReader::new(file, Some(file_size as usize)),
            speed,
        })
    }

    /// Returns a stream of MessagesContainer, same interface as DiagDevice.
    pub fn as_stream(
        &mut self,
    ) -> impl TryStream<Ok = MessagesContainer, Error = std::io::Error> + '_ {
        self.reader.as_stream()
    }
}
