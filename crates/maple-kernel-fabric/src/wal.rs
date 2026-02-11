use std::io::{Read, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::FabricError;
use crate::event::KernelEvent;
use crate::types::{IntegrityReport, WorldlineId};

/// WAL magic bytes: "MWLW" (Maple WorldLine WAL)
const WAL_MAGIC: [u8; 4] = [b'M', b'W', b'L', b'W'];
/// Current WAL format version
const WAL_VERSION: u16 = 1;
/// Segment header size: magic(4) + version(2) + reserved(2) = 8
const SEGMENT_HEADER_SIZE: usize = 8;
/// Entry overhead: length(4) + sequence(8) + crc32(4) = 16
const ENTRY_OVERHEAD: usize = 16;

/// Storage backend trait — allows file-backed and in-memory WAL.
pub trait WalStorage: Send + Sync {
    /// Create a new segment (truncates if exists).
    fn create_segment(&self, segment_id: u64) -> Result<Box<dyn SegmentWriter>, FabricError>;
    /// Open an existing segment for appending.
    fn append_segment(&self, segment_id: u64) -> Result<Box<dyn SegmentWriter>, FabricError>;
    fn open_segment(&self, segment_id: u64) -> Result<Box<dyn SegmentReader>, FabricError>;
    fn list_segments(&self) -> Result<Vec<u64>, FabricError>;
    fn remove_segment(&self, segment_id: u64) -> Result<(), FabricError>;
    fn rename_segment(&self, from: u64, to_path: &Path) -> Result<(), FabricError>;
}

pub trait SegmentWriter: Send + Sync {
    fn write_all(&mut self, data: &[u8]) -> Result<(), FabricError>;
    fn flush(&mut self) -> Result<(), FabricError>;
    fn sync(&mut self) -> Result<(), FabricError>;
    fn position(&self) -> u64;
}

pub trait SegmentReader: Send + Sync {
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), FabricError>;
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, FabricError>;
    fn position(&self) -> u64;
    fn seek_to(&mut self, pos: u64) -> Result<(), FabricError>;
    fn len(&self) -> Result<u64, FabricError>;
}

// ---- File-backed storage ----

pub struct FileStorage {
    data_dir: PathBuf,
}

impl FileStorage {
    pub fn new(data_dir: PathBuf) -> Result<Self, FabricError> {
        std::fs::create_dir_all(&data_dir)?;
        Ok(Self { data_dir })
    }

    fn segment_path(&self, segment_id: u64) -> PathBuf {
        self.data_dir.join(format!("wal-{:016x}.seg", segment_id))
    }
}

impl WalStorage for FileStorage {
    fn create_segment(&self, segment_id: u64) -> Result<Box<dyn SegmentWriter>, FabricError> {
        let path = self.segment_path(segment_id);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        Ok(Box::new(FileSegmentWriter { file, position: 0 }))
    }

    fn append_segment(&self, segment_id: u64) -> Result<Box<dyn SegmentWriter>, FabricError> {
        let path = self.segment_path(segment_id);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)?;
        let position = file.metadata()?.len();
        Ok(Box::new(FileSegmentWriter { file, position }))
    }

    fn open_segment(&self, segment_id: u64) -> Result<Box<dyn SegmentReader>, FabricError> {
        let path = self.segment_path(segment_id);
        let file = std::fs::File::open(&path)?;
        let len = file.metadata()?.len();
        Ok(Box::new(FileSegmentReader {
            file,
            position: 0,
            len,
        }))
    }

    fn list_segments(&self) -> Result<Vec<u64>, FabricError> {
        let mut segments = Vec::new();
        for entry in std::fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("wal-") && name.ends_with(".seg") {
                if let Some(hex) = name.strip_prefix("wal-").and_then(|s| s.strip_suffix(".seg")) {
                    if let Ok(id) = u64::from_str_radix(hex, 16) {
                        segments.push(id);
                    }
                }
            }
        }
        segments.sort();
        Ok(segments)
    }

    fn remove_segment(&self, segment_id: u64) -> Result<(), FabricError> {
        let path = self.segment_path(segment_id);
        std::fs::remove_file(&path)?;
        Ok(())
    }

    fn rename_segment(&self, segment_id: u64, to_path: &Path) -> Result<(), FabricError> {
        let from = self.segment_path(segment_id);
        std::fs::rename(&from, to_path)?;
        Ok(())
    }
}

struct FileSegmentWriter {
    file: std::fs::File,
    position: u64,
}

impl SegmentWriter for FileSegmentWriter {
    fn write_all(&mut self, data: &[u8]) -> Result<(), FabricError> {
        self.file.write_all(data)?;
        self.position += data.len() as u64;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), FabricError> {
        self.file.flush()?;
        Ok(())
    }

    fn sync(&mut self) -> Result<(), FabricError> {
        self.file.sync_all()?;
        Ok(())
    }

    fn position(&self) -> u64 {
        self.position
    }
}

struct FileSegmentReader {
    file: std::fs::File,
    position: u64,
    len: u64,
}

impl SegmentReader for FileSegmentReader {
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), FabricError> {
        self.file.read_exact(buf)?;
        self.position += buf.len() as u64;
        Ok(())
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, FabricError> {
        let n = self.file.read_to_end(buf)?;
        self.position += n as u64;
        Ok(n)
    }

    fn position(&self) -> u64 {
        self.position
    }

    fn seek_to(&mut self, pos: u64) -> Result<(), FabricError> {
        use std::io::Seek;
        self.file.seek(std::io::SeekFrom::Start(pos))?;
        self.position = pos;
        Ok(())
    }

    fn len(&self) -> Result<u64, FabricError> {
        Ok(self.len)
    }
}

// ---- In-memory storage (for testing) ----

pub struct MemoryStorage {
    segments: std::sync::Mutex<std::collections::BTreeMap<u64, Vec<u8>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            segments: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl WalStorage for MemoryStorage {
    fn create_segment(&self, segment_id: u64) -> Result<Box<dyn SegmentWriter>, FabricError> {
        self.segments
            .lock()
            .unwrap()
            .insert(segment_id, Vec::new());
        Ok(Box::new(MemorySegmentWriter {
            segment_id,
            buffer: Vec::new(),
            segments: &self.segments as *const _ as usize,
        }))
    }

    fn append_segment(&self, segment_id: u64) -> Result<Box<dyn SegmentWriter>, FabricError> {
        // For memory storage, append just creates a writer that appends to existing data
        // (doesn't clear existing data like create_segment does)
        Ok(Box::new(MemorySegmentWriter {
            segment_id,
            buffer: Vec::new(),
            segments: &self.segments as *const _ as usize,
        }))
    }

    fn open_segment(&self, segment_id: u64) -> Result<Box<dyn SegmentReader>, FabricError> {
        let segments = self.segments.lock().unwrap();
        let data = segments
            .get(&segment_id)
            .cloned()
            .ok_or_else(|| FabricError::SegmentNotFound(segment_id))?;
        Ok(Box::new(MemorySegmentReader { data, position: 0 }))
    }

    fn list_segments(&self) -> Result<Vec<u64>, FabricError> {
        let segments = self.segments.lock().unwrap();
        Ok(segments.keys().copied().collect())
    }

    fn remove_segment(&self, segment_id: u64) -> Result<(), FabricError> {
        self.segments.lock().unwrap().remove(&segment_id);
        Ok(())
    }

    fn rename_segment(&self, _segment_id: u64, _to_path: &Path) -> Result<(), FabricError> {
        // No-op for memory storage
        Ok(())
    }
}

struct MemorySegmentWriter {
    segment_id: u64,
    buffer: Vec<u8>,
    // Raw pointer to the parent storage's segments mutex.
    // SAFETY: MemoryStorage owns the mutex and outlives the writer.
    segments: usize,
}

// SAFETY: MemorySegmentWriter is only used within the WAL's RwLock,
// and the MemoryStorage reference outlives it.
unsafe impl Send for MemorySegmentWriter {}
unsafe impl Sync for MemorySegmentWriter {}

impl SegmentWriter for MemorySegmentWriter {
    fn write_all(&mut self, data: &[u8]) -> Result<(), FabricError> {
        self.buffer.extend_from_slice(data);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), FabricError> {
        self.sync()
    }

    fn sync(&mut self) -> Result<(), FabricError> {
        // Flush buffer to the shared storage
        let segments_ptr =
            self.segments as *const std::sync::Mutex<std::collections::BTreeMap<u64, Vec<u8>>>;
        // SAFETY: The MemoryStorage that owns this Mutex outlives the writer.
        let segments = unsafe { &*segments_ptr };
        let mut guard = segments.lock().unwrap();
        let entry = guard.entry(self.segment_id).or_default();
        entry.extend_from_slice(&self.buffer);
        self.buffer.clear();
        Ok(())
    }

    fn position(&self) -> u64 {
        // We need to check current segment size + buffer
        let segments_ptr =
            self.segments as *const std::sync::Mutex<std::collections::BTreeMap<u64, Vec<u8>>>;
        let segments = unsafe { &*segments_ptr };
        let guard = segments.lock().unwrap();
        let stored = guard.get(&self.segment_id).map(|v| v.len()).unwrap_or(0);
        (stored + self.buffer.len()) as u64
    }
}

struct MemorySegmentReader {
    data: Vec<u8>,
    position: usize,
}

impl SegmentReader for MemorySegmentReader {
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), FabricError> {
        if self.position + buf.len() > self.data.len() {
            return Err(FabricError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "read past end of memory segment",
            )));
        }
        buf.copy_from_slice(&self.data[self.position..self.position + buf.len()]);
        self.position += buf.len();
        Ok(())
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, FabricError> {
        let remaining = &self.data[self.position..];
        buf.extend_from_slice(remaining);
        let n = remaining.len();
        self.position = self.data.len();
        Ok(n)
    }

    fn position(&self) -> u64 {
        self.position as u64
    }

    fn seek_to(&mut self, pos: u64) -> Result<(), FabricError> {
        self.position = pos as usize;
        Ok(())
    }

    fn len(&self) -> Result<u64, FabricError> {
        Ok(self.data.len() as u64)
    }
}

// ---- Segment metadata ----

#[derive(Clone, Debug)]
pub struct SegmentMeta {
    pub id: u64,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub size_bytes: u64,
    pub entry_count: u64,
}

// ---- WAL Configuration ----

/// WAL configuration.
pub struct WalConfig {
    /// Maximum segment size before rotation (default: 64MB)
    pub max_segment_size: u64,
    /// Sync mode
    pub sync_mode: SyncMode,
    /// Batch interval for batched sync (default: 10ms)
    pub batch_interval: Duration,
    /// Maximum events per batch (default: 1000)
    pub max_batch_size: usize,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            max_segment_size: 64 * 1024 * 1024, // 64MB
            sync_mode: SyncMode::Immediate,
            batch_interval: Duration::from_millis(10),
            max_batch_size: 1000,
        }
    }
}

/// Sync mode for the WAL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncMode {
    /// fsync after every write — safest, slower
    Immediate,
    /// fsync in batches — faster, small risk window
    Batched,
    /// No explicit fsync — fastest, relies on OS
    OsManaged,
}

// ---- Write-Ahead Log ----

/// Write-Ahead Log: append-only, crash-recoverable event journal.
///
/// Segment format: `[magic:4][version:2][reserved:2][entries...]`
/// Entry format:   `[length:4][sequence:8][event_bytes:N][crc32:4]`
pub struct WriteAheadLog {
    config: WalConfig,
    storage: Box<dyn WalStorage>,
    current_segment_id: RwLock<u64>,
    current_writer: RwLock<Option<Box<dyn SegmentWriter>>>,
    segments: RwLock<Vec<SegmentMeta>>,
    sequence_counter: AtomicU64,
    current_segment_size: AtomicU64,
}

impl WriteAheadLog {
    /// Open or create a WAL with the given storage backend.
    pub async fn open(config: WalConfig, storage: Box<dyn WalStorage>) -> Result<Self, FabricError> {
        let existing_segments = storage.list_segments()?;

        let mut segments_meta = Vec::new();
        let mut max_sequence: u64 = 0;

        // Scan existing segments to rebuild metadata
        for &seg_id in &existing_segments {
            let mut reader = storage.open_segment(seg_id)?;
            let seg_len = reader.len()?;
            if seg_len < SEGMENT_HEADER_SIZE as u64 {
                warn!(segment_id = seg_id, "Segment too small, skipping");
                continue;
            }

            // Validate header
            let mut header = [0u8; SEGMENT_HEADER_SIZE];
            reader.read_exact(&mut header)?;
            if header[..4] != WAL_MAGIC {
                warn!(segment_id = seg_id, "Invalid magic, skipping");
                continue;
            }

            // Scan entries for sequence numbers
            let mut first_seq = u64::MAX;
            let mut last_seq = 0u64;
            let mut entry_count = 0u64;

            while (reader.position() as usize) < seg_len as usize {
                // Try to read entry header: length(4) + sequence(8)
                let mut entry_header = [0u8; 12];
                if reader.read_exact(&mut entry_header).is_err() {
                    break;
                }
                let length = u32::from_le_bytes(entry_header[..4].try_into().unwrap()) as usize;
                let sequence = u64::from_le_bytes(entry_header[4..12].try_into().unwrap());

                if length == 0 || reader.position() + length as u64 + 4 > seg_len {
                    break;
                }

                // Skip event bytes + crc32
                let skip_pos = reader.position() + length as u64 + 4;
                reader.seek_to(skip_pos)?;

                first_seq = first_seq.min(sequence);
                last_seq = last_seq.max(sequence);
                max_sequence = max_sequence.max(sequence);
                entry_count += 1;
            }

            if entry_count > 0 {
                segments_meta.push(SegmentMeta {
                    id: seg_id,
                    first_sequence: first_seq,
                    last_sequence: last_seq,
                    size_bytes: seg_len,
                    entry_count,
                });
            }
        }

        // Create or reuse the current segment
        let current_seg_id = segments_meta
            .last()
            .map(|s| s.id)
            .unwrap_or(0);

        let (writer, seg_size) = if segments_meta.is_empty() || segments_meta.last().unwrap().size_bytes >= config.max_segment_size {
            // Need a new segment
            let new_id = current_seg_id + 1;
            let mut writer = storage.create_segment(new_id)?;
            write_segment_header(&mut *writer)?;

            if segments_meta.is_empty() {
                segments_meta.push(SegmentMeta {
                    id: new_id,
                    first_sequence: 0,
                    last_sequence: 0,
                    size_bytes: SEGMENT_HEADER_SIZE as u64,
                    entry_count: 0,
                });
            } else {
                segments_meta.push(SegmentMeta {
                    id: new_id,
                    first_sequence: max_sequence + 1,
                    last_sequence: max_sequence,
                    size_bytes: SEGMENT_HEADER_SIZE as u64,
                    entry_count: 0,
                });
            }

            (writer, SEGMENT_HEADER_SIZE as u64)
        } else {
            // Append to the last segment (open in append mode, not truncate)
            let last = segments_meta.last().unwrap();
            let writer = storage.append_segment(last.id)?;
            let seg_size = last.size_bytes;
            (writer, seg_size)
        };

        info!(
            segments = segments_meta.len(),
            max_sequence = max_sequence,
            "WAL opened"
        );

        let final_seg_id = segments_meta.last().map(|s| s.id).unwrap_or(1);

        Ok(Self {
            config,
            storage,
            current_segment_id: RwLock::new(final_seg_id),
            current_writer: RwLock::new(Some(writer)),
            segments: RwLock::new(segments_meta),
            sequence_counter: AtomicU64::new(max_sequence),
            current_segment_size: AtomicU64::new(seg_size),
        })
    }

    /// Open with file-backed storage.
    pub async fn open_file(config: WalConfig, data_dir: PathBuf) -> Result<Self, FabricError> {
        let storage = Box::new(FileStorage::new(data_dir)?);
        Self::open(config, storage).await
    }

    /// Open with in-memory storage (for testing).
    pub async fn open_memory(config: WalConfig) -> Result<Self, FabricError> {
        let storage = Box::new(MemoryStorage::new());
        Self::open(config, storage).await
    }

    /// Append an event. Returns the sequence number.
    pub async fn append(&self, event: &KernelEvent) -> Result<u64, FabricError> {
        let event_bytes =
            serde_json::to_vec(event).map_err(|e| FabricError::Serialization(e.to_string()))?;
        let sequence = self.sequence_counter.fetch_add(1, Ordering::SeqCst) + 1;

        let crc = crc32fast::hash(&event_bytes);

        // Build entry: [length:4][sequence:8][event_bytes:N][crc32:4]
        let length = event_bytes.len() as u32;
        let mut entry = Vec::with_capacity(ENTRY_OVERHEAD + event_bytes.len());
        entry.extend_from_slice(&length.to_le_bytes());
        entry.extend_from_slice(&sequence.to_le_bytes());
        entry.extend_from_slice(&event_bytes);
        entry.extend_from_slice(&crc.to_le_bytes());

        {
            let mut writer_guard = self.current_writer.write().await;
            let writer = writer_guard
                .as_mut()
                .ok_or(FabricError::Closed)?;

            writer.write_all(&entry)?;

            match self.config.sync_mode {
                SyncMode::Immediate => writer.sync()?,
                SyncMode::Batched => writer.flush()?,
                SyncMode::OsManaged => {}
            }
        }

        let new_size = self
            .current_segment_size
            .fetch_add(entry.len() as u64, Ordering::SeqCst)
            + entry.len() as u64;

        // Update segment metadata for the segment we just wrote to (BEFORE rotation)
        {
            let mut segments = self.segments.write().await;
            if let Some(last) = segments.last_mut() {
                last.last_sequence = sequence;
                last.size_bytes = new_size;
                last.entry_count += 1;
                if last.first_sequence == 0 || last.first_sequence > sequence {
                    last.first_sequence = sequence;
                }
            }
        }

        // Check if segment rotation is needed (after metadata is updated)
        if new_size >= self.config.max_segment_size {
            self.rotate_segment().await?;
        }

        debug!(sequence = sequence, stage = ?event.stage, "Event appended to WAL");
        Ok(sequence)
    }

    /// Append a batch of events atomically.
    pub async fn append_batch(&self, events: &[KernelEvent]) -> Result<Vec<u64>, FabricError> {
        let mut sequences = Vec::with_capacity(events.len());
        for event in events {
            let seq = self.append(event).await?;
            sequences.push(seq);
        }
        Ok(sequences)
    }

    /// Read events from a sequence number.
    pub async fn read_from(
        &self,
        from_sequence: u64,
        limit: usize,
    ) -> Result<Vec<(u64, KernelEvent)>, FabricError> {
        let segments = self.segments.read().await;
        let mut results = Vec::new();

        for seg_meta in segments.iter() {
            if seg_meta.last_sequence < from_sequence {
                continue;
            }

            let entries = self.read_segment_entries(seg_meta.id).await?;
            for (seq, event) in entries {
                if seq >= from_sequence {
                    results.push((seq, event));
                    if results.len() >= limit {
                        return Ok(results);
                    }
                }
            }
        }

        Ok(results)
    }

    /// Read events for a specific worldline.
    pub async fn read_worldline(
        &self,
        worldline_id: &WorldlineId,
        from: u64,
        limit: usize,
    ) -> Result<Vec<(u64, KernelEvent)>, FabricError> {
        let all = self.read_from(from, usize::MAX).await?;
        let filtered: Vec<_> = all
            .into_iter()
            .filter(|(_, e)| e.worldline_id == *worldline_id)
            .take(limit)
            .collect();
        Ok(filtered)
    }

    /// Get the latest sequence number.
    pub fn latest_sequence(&self) -> u64 {
        self.sequence_counter.load(Ordering::SeqCst)
    }

    /// Create a checkpoint. Returns the sequence number at checkpoint.
    pub async fn checkpoint(&self) -> Result<u64, FabricError> {
        // Flush current writer
        {
            let mut writer_guard = self.current_writer.write().await;
            if let Some(ref mut writer) = *writer_guard {
                writer.sync()?;
            }
        }

        let seq = self.latest_sequence();
        info!(sequence = seq, "Checkpoint created");
        Ok(seq)
    }

    /// Archive segments before a given sequence number.
    pub async fn archive_before(
        &self,
        sequence: u64,
        archive_path: &Path,
    ) -> Result<(), FabricError> {
        std::fs::create_dir_all(archive_path)?;
        let mut segments = self.segments.write().await;
        let mut to_remove = Vec::new();

        for seg in segments.iter() {
            if seg.last_sequence < sequence {
                let dest = archive_path.join(format!("wal-{:016x}.seg", seg.id));
                self.storage.rename_segment(seg.id, &dest)?;
                to_remove.push(seg.id);
            }
        }

        segments.retain(|s| !to_remove.contains(&s.id));
        info!(
            archived = to_remove.len(),
            before_sequence = sequence,
            "Segments archived"
        );
        Ok(())
    }

    /// Replay events from WAL (for crash recovery).
    pub async fn replay<F>(&self, from: u64, mut handler: F) -> Result<u64, FabricError>
    where
        F: FnMut(u64, KernelEvent) -> Result<(), FabricError>,
    {
        let events = self.read_from(from, usize::MAX).await?;
        let mut count = 0u64;
        for (seq, event) in events {
            handler(seq, event)?;
            count += 1;
        }
        info!(replayed = count, from = from, "WAL replay complete");
        Ok(count)
    }

    /// Verify WAL integrity (check all CRC32 and event hashes).
    pub async fn verify_integrity(&self) -> Result<IntegrityReport, FabricError> {
        let segments = self.segments.read().await;
        let mut report = IntegrityReport {
            total_events: 0,
            verified_events: 0,
            corrupted_events: 0,
            corrupted_offsets: Vec::new(),
            segments_checked: 0,
        };

        for seg_meta in segments.iter() {
            report.segments_checked += 1;
            let entries = self.read_segment_entries_raw(seg_meta.id).await?;

            for (offset, _seq, event_bytes, stored_crc) in entries {
                report.total_events += 1;

                // Verify CRC32
                let computed_crc = crc32fast::hash(&event_bytes);
                if computed_crc != stored_crc {
                    report.corrupted_events += 1;
                    report.corrupted_offsets.push(offset);
                    continue;
                }

                // Verify event integrity hash
                match serde_json::from_slice::<KernelEvent>(&event_bytes) {
                    Ok(event) if event.verify_integrity() => {
                        report.verified_events += 1;
                    }
                    Ok(_) => {
                        report.corrupted_events += 1;
                        report.corrupted_offsets.push(offset);
                    }
                    Err(_) => {
                        report.corrupted_events += 1;
                        report.corrupted_offsets.push(offset);
                    }
                }
            }
        }

        Ok(report)
    }

    // ---- Internal helpers ----

    async fn rotate_segment(&self) -> Result<(), FabricError> {
        let mut writer_guard = self.current_writer.write().await;

        // Flush and sync the current writer
        if let Some(ref mut writer) = *writer_guard {
            writer.sync()?;
        }

        let mut seg_id = self.current_segment_id.write().await;
        let new_id = *seg_id + 1;

        let mut writer = self.storage.create_segment(new_id)?;
        write_segment_header(&mut *writer)?;

        *writer_guard = Some(writer);
        *seg_id = new_id;
        self.current_segment_size
            .store(SEGMENT_HEADER_SIZE as u64, Ordering::SeqCst);

        // Add new segment metadata
        let mut segments = self.segments.write().await;
        let next_seq = self.sequence_counter.load(Ordering::SeqCst) + 1;
        segments.push(SegmentMeta {
            id: new_id,
            first_sequence: next_seq,
            last_sequence: next_seq.saturating_sub(1),
            size_bytes: SEGMENT_HEADER_SIZE as u64,
            entry_count: 0,
        });

        info!(segment_id = new_id, "WAL segment rotated");
        Ok(())
    }

    async fn read_segment_entries(
        &self,
        segment_id: u64,
    ) -> Result<Vec<(u64, KernelEvent)>, FabricError> {
        let raw = self.read_segment_entries_raw(segment_id).await?;
        let mut results = Vec::new();
        for (_offset, seq, event_bytes, stored_crc) in raw {
            let computed_crc = crc32fast::hash(&event_bytes);
            if computed_crc != stored_crc {
                warn!(
                    segment_id = segment_id,
                    sequence = seq,
                    "CRC mismatch, skipping corrupted entry"
                );
                continue;
            }
            match serde_json::from_slice::<KernelEvent>(&event_bytes) {
                Ok(event) => results.push((seq, event)),
                Err(e) => {
                    warn!(
                        segment_id = segment_id,
                        sequence = seq,
                        error = %e,
                        "Failed to deserialize event, skipping"
                    );
                }
            }
        }
        Ok(results)
    }

    /// Read raw entries: (offset, sequence, event_bytes, crc32)
    async fn read_segment_entries_raw(
        &self,
        segment_id: u64,
    ) -> Result<Vec<(u64, u64, Vec<u8>, u32)>, FabricError> {
        // Flush current writer first so in-memory data is readable
        {
            let mut writer_guard = self.current_writer.write().await;
            if let Some(ref mut writer) = *writer_guard {
                writer.flush()?;
            }
        }

        let mut reader = self.storage.open_segment(segment_id)?;
        let seg_len = reader.len()?;

        if seg_len < SEGMENT_HEADER_SIZE as u64 {
            return Ok(Vec::new());
        }

        // Skip header
        let mut header = [0u8; SEGMENT_HEADER_SIZE];
        reader.read_exact(&mut header)?;

        let mut results = Vec::new();

        while reader.position() + ENTRY_OVERHEAD as u64 <= seg_len {
            let entry_offset = reader.position();

            // Read length(4) + sequence(8)
            let mut entry_header = [0u8; 12];
            if reader.read_exact(&mut entry_header).is_err() {
                break;
            }

            let length = u32::from_le_bytes(entry_header[..4].try_into().unwrap()) as usize;
            let sequence = u64::from_le_bytes(entry_header[4..12].try_into().unwrap());

            if length == 0 || reader.position() + length as u64 + 4 > seg_len {
                break;
            }

            // Read event bytes
            let mut event_bytes = vec![0u8; length];
            if reader.read_exact(&mut event_bytes).is_err() {
                break;
            }

            // Read CRC32
            let mut crc_bytes = [0u8; 4];
            if reader.read_exact(&mut crc_bytes).is_err() {
                break;
            }
            let stored_crc = u32::from_le_bytes(crc_bytes);

            results.push((entry_offset, sequence, event_bytes, stored_crc));
        }

        Ok(results)
    }
}

fn write_segment_header(writer: &mut dyn SegmentWriter) -> Result<(), FabricError> {
    let mut header = [0u8; SEGMENT_HEADER_SIZE];
    header[..4].copy_from_slice(&WAL_MAGIC);
    header[4..6].copy_from_slice(&WAL_VERSION.to_le_bytes());
    // bytes 6..8 reserved
    writer.write_all(&header)?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventPayload, KernelEvent, ResonanceStage};
    use crate::hlc::HlcTimestamp;
    use crate::types::{EventId, NodeId, WorldlineId};
    use maple_mwl_types::IdentityMaterial;

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn test_event(seq_hint: u32) -> KernelEvent {
        KernelEvent::new(
            EventId::new(),
            HlcTimestamp {
                physical: 1000 + seq_hint as u64,
                logical: 0,
                node_id: NodeId(1),
            },
            test_worldline(),
            ResonanceStage::Meaning,
            EventPayload::MeaningFormed {
                interpretation_count: seq_hint,
                confidence: 0.5,
                ambiguity_preserved: true,
            },
            vec![],
        )
    }

    #[tokio::test]
    async fn append_and_read_roundtrip() {
        let wal = WriteAheadLog::open_memory(WalConfig::default()).await.unwrap();

        let event = test_event(1);
        let seq = wal.append(&event).await.unwrap();
        assert_eq!(seq, 1);

        let events = wal.read_from(1, 10).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, 1);
        assert_eq!(events[0].1.id, event.id);
        assert!(events[0].1.verify_integrity());
    }

    #[tokio::test]
    async fn append_multiple_and_read() {
        let wal = WriteAheadLog::open_memory(WalConfig::default()).await.unwrap();

        for i in 0..10 {
            wal.append(&test_event(i)).await.unwrap();
        }

        assert_eq!(wal.latest_sequence(), 10);

        let events = wal.read_from(1, 100).await.unwrap();
        assert_eq!(events.len(), 10);

        // Verify ordering
        for i in 0..10 {
            assert_eq!(events[i].0, (i + 1) as u64);
        }
    }

    #[tokio::test]
    async fn read_from_middle() {
        let wal = WriteAheadLog::open_memory(WalConfig::default()).await.unwrap();

        for i in 0..10 {
            wal.append(&test_event(i)).await.unwrap();
        }

        let events = wal.read_from(5, 100).await.unwrap();
        assert_eq!(events.len(), 6); // sequences 5,6,7,8,9,10
        assert_eq!(events[0].0, 5);
    }

    #[tokio::test]
    async fn read_with_limit() {
        let wal = WriteAheadLog::open_memory(WalConfig::default()).await.unwrap();

        for i in 0..10 {
            wal.append(&test_event(i)).await.unwrap();
        }

        let events = wal.read_from(1, 3).await.unwrap();
        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn segment_rotation() {
        let config = WalConfig {
            max_segment_size: 200, // Very small to trigger rotation
            ..WalConfig::default()
        };
        let wal = WriteAheadLog::open_memory(config).await.unwrap();

        for i in 0..20 {
            wal.append(&test_event(i)).await.unwrap();
        }

        let segments = wal.segments.read().await;
        assert!(
            segments.len() > 1,
            "Should have rotated to multiple segments, got {}",
            segments.len()
        );

        drop(segments);

        // All events should still be readable
        let events = wal.read_from(1, 100).await.unwrap();
        assert_eq!(events.len(), 20);
    }

    #[tokio::test]
    async fn integrity_verification() {
        let wal = WriteAheadLog::open_memory(WalConfig::default()).await.unwrap();

        for i in 0..5 {
            wal.append(&test_event(i)).await.unwrap();
        }

        let report = wal.verify_integrity().await.unwrap();
        assert!(report.is_clean());
        assert_eq!(report.total_events, 5);
        assert_eq!(report.verified_events, 5);
    }

    #[tokio::test]
    async fn read_worldline_filter() {
        let wal = WriteAheadLog::open_memory(WalConfig::default()).await.unwrap();

        let wid1 = WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]));
        let wid2 = WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]));

        let e1 = KernelEvent::new(
            EventId::new(),
            HlcTimestamp { physical: 1000, logical: 0, node_id: NodeId(1) },
            wid1.clone(),
            ResonanceStage::Meaning,
            EventPayload::MeaningFormed { interpretation_count: 1, confidence: 0.5, ambiguity_preserved: true },
            vec![],
        );
        let e2 = KernelEvent::new(
            EventId::new(),
            HlcTimestamp { physical: 1001, logical: 0, node_id: NodeId(1) },
            wid2.clone(),
            ResonanceStage::Meaning,
            EventPayload::MeaningFormed { interpretation_count: 2, confidence: 0.5, ambiguity_preserved: true },
            vec![],
        );

        wal.append(&e1).await.unwrap();
        wal.append(&e2).await.unwrap();

        let wid1_events = wal.read_worldline(&wid1, 1, 100).await.unwrap();
        assert_eq!(wid1_events.len(), 1);
        assert_eq!(wid1_events[0].1.worldline_id, wid1);

        let wid2_events = wal.read_worldline(&wid2, 1, 100).await.unwrap();
        assert_eq!(wid2_events.len(), 1);
        assert_eq!(wid2_events[0].1.worldline_id, wid2);
    }

    #[tokio::test]
    async fn checkpoint() {
        let wal = WriteAheadLog::open_memory(WalConfig::default()).await.unwrap();

        for i in 0..5 {
            wal.append(&test_event(i)).await.unwrap();
        }

        let seq = wal.checkpoint().await.unwrap();
        assert_eq!(seq, 5);
    }

    #[tokio::test]
    async fn replay() {
        let wal = WriteAheadLog::open_memory(WalConfig::default()).await.unwrap();

        for i in 0..5 {
            wal.append(&test_event(i)).await.unwrap();
        }

        let mut replayed = Vec::new();
        let count = wal
            .replay(1, |seq, event| {
                replayed.push((seq, event));
                Ok(())
            })
            .await
            .unwrap();

        assert_eq!(count, 5);
        assert_eq!(replayed.len(), 5);
    }

    #[tokio::test]
    async fn batch_append() {
        let wal = WriteAheadLog::open_memory(WalConfig::default()).await.unwrap();

        let events: Vec<_> = (0..5).map(|i| test_event(i)).collect();
        let seqs = wal.append_batch(&events).await.unwrap();

        assert_eq!(seqs.len(), 5);
        assert_eq!(seqs, vec![1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    async fn file_backed_wal() {
        let dir = tempfile::tempdir().unwrap();
        let config = WalConfig::default();
        let wal = WriteAheadLog::open_file(config, dir.path().to_path_buf())
            .await
            .unwrap();

        for i in 0..10 {
            wal.append(&test_event(i)).await.unwrap();
        }

        let events = wal.read_from(1, 100).await.unwrap();
        assert_eq!(events.len(), 10);

        let report = wal.verify_integrity().await.unwrap();
        assert!(report.is_clean());
    }

    #[tokio::test]
    async fn file_backed_crash_recovery() {
        let dir = tempfile::tempdir().unwrap();

        // Write some events
        {
            let config = WalConfig::default();
            let wal = WriteAheadLog::open_file(config, dir.path().to_path_buf())
                .await
                .unwrap();
            for i in 0..5 {
                wal.append(&test_event(i)).await.unwrap();
            }
            wal.checkpoint().await.unwrap();
            // WAL is dropped here (simulating crash)
        }

        // Recover
        {
            let config = WalConfig::default();
            let wal = WriteAheadLog::open_file(config, dir.path().to_path_buf())
                .await
                .unwrap();

            let mut replayed = Vec::new();
            let count = wal
                .replay(1, |seq, event| {
                    replayed.push((seq, event));
                    Ok(())
                })
                .await
                .unwrap();

            assert_eq!(count, 5);
            for (_, event) in &replayed {
                assert!(event.verify_integrity());
            }
        }
    }
}
