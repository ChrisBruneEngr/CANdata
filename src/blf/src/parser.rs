//! BLF parser implementation.
//! BLF parser module for parsing log objects from BLF files.

use crate::objects::*;
use crate::{BlfParseError, BlfParseResult, LogContainer, ObjectType};
use crate::BlfResultContext;

use rayon::prelude::*;
use std::io::{Cursor, Read};

// Log object enum for all supported BLF objects
#[derive(Debug, Clone, PartialEq)]
pub enum LogObject {
    CanMessage(CanMessage),
    CanMessage2(CanMessage2),
    CanErrorFrame(CanErrorFrame),
    CanFdMessage(CanFdMessage),
    CanFdMessage64(CanFdMessage64),
    CanOverloadFrame(CanOverloadFrame),
    CanDriverStatistic(CanDriverStatistic),
    CanDriverError(CanDriverError),
    LinMessage(LinMessage),
    LinMessage2(LinMessage2),
    LinCrcError(LinCrcError),
    LinDlcInfo(LinDlcInfo),
    LinReceiveError(LinReceiveError),
    LinSendError(LinSendError),
    LinSlaveTimeout(LinSlaveTimeout),
    LinSchedulerModeChange(LinSchedulerModeChange),
    LinSyncError(LinSyncError),
    LinBaudrateEvent(LinBaudrateEvent),
    LinSleepModeEvent(LinSleepModeEvent),
    LinWakeupEvent(LinWakeupEvent),
    FlexRayData(FlexRayData),
    FlexRaySync(FlexRaySync),
    FlexRayV6Message(FlexRayV6Message),
    FlexRayV6StartCycleEvent(FlexRayV6StartCycleEvent),
    FlexRayStatusEvent(FlexRayStatusEvent),
    FlexRayVFrError(FlexRayVFrError),
    FlexRayVFrStatus(FlexRayVFrStatus),
    FlexRayVFrStartCycle(FlexRayVFrStartCycle),
    FlexRayVFrReceiveMsg(FlexRayVFrReceiveMsg),
    FlexRayVFrReceiveMsgEx(FlexRayVFrReceiveMsgEx),
    EthernetFrame(EthernetFrame),
    // Environment variables
    // EnvInteger(EnvInteger),
    // EnvDouble(EnvDouble),
    // EnvString(EnvString),
    AppTrigger(AppTrigger),
    EventComment(EventComment),
    GlobalMarker(GlobalMarker),
    MostSpy(MostSpy),
    MostCtrl(MostCtrl),
    MostPkt2(MostPkt2),
    MostLightLock(MostLightLock),
    MostStatistic(MostStatistic),
    MostHwMode(MostHwMode),
    MostReg(MostReg),
    MostGenReg(MostGenReg),
    MostNetState(MostNetState),
    MostDataLost(MostDataLost),
    MostTrigger(MostTrigger),
    // Placeholder for unhandled objects
    Unhandled {
        object_type: u32,
        timestamp: u64,
        data: Vec<u8>,
    },
}

impl LogObject {
    /// Returns the timestamp of the log object
    pub fn timestamp(&self) -> u64 {
        match self {
            LogObject::CanMessage(msg) => msg.header.object_time_stamp,
            LogObject::CanMessage2(msg) => msg.header.object_time_stamp,
            LogObject::CanErrorFrame(msg) => msg.header.object_time_stamp,
            LogObject::CanFdMessage(msg) => msg.header.object_time_stamp,
            LogObject::CanFdMessage64(msg) => msg.header.object_time_stamp,
            LogObject::CanOverloadFrame(msg) => msg.header.object_time_stamp,
            LogObject::CanDriverStatistic(msg) => msg.header.object_time_stamp,
            LogObject::CanDriverError(msg) => msg.header.object_time_stamp,
            LogObject::LinMessage(msg) => msg.header.object_time_stamp,
            LogObject::LinMessage2(msg) => msg.header.object_time_stamp,
            LogObject::LinCrcError(msg) => msg.header.object_time_stamp,
            LogObject::LinDlcInfo(msg) => msg.header.object_time_stamp,
            LogObject::LinReceiveError(msg) => msg.header.object_time_stamp,
            LogObject::LinSendError(msg) => msg.header.object_time_stamp,
            LogObject::LinSlaveTimeout(msg) => msg.header.object_time_stamp,
            LogObject::LinSchedulerModeChange(msg) => msg.header.object_time_stamp,
            LogObject::LinSyncError(msg) => msg.header.object_time_stamp,
            LogObject::LinBaudrateEvent(msg) => msg.header.object_time_stamp,
            LogObject::LinSleepModeEvent(msg) => msg.header.object_time_stamp,
            LogObject::LinWakeupEvent(msg) => msg.header.object_time_stamp,
            LogObject::FlexRayData(msg) => msg.timestamp,
            LogObject::FlexRaySync(msg) => msg.timestamp,
            LogObject::FlexRayV6Message(msg) => msg.timestamp,
            LogObject::FlexRayV6StartCycleEvent(msg) => msg.timestamp,
            LogObject::FlexRayStatusEvent(msg) => msg.timestamp,
            LogObject::FlexRayVFrError(msg) => msg.timestamp,
            LogObject::FlexRayVFrStatus(msg) => msg.timestamp,
            LogObject::FlexRayVFrStartCycle(msg) => msg.timestamp,
            LogObject::FlexRayVFrReceiveMsg(msg) => msg.timestamp,
            LogObject::FlexRayVFrReceiveMsgEx(msg) => msg.timestamp,
            LogObject::EthernetFrame(msg) => msg.timestamp,
            LogObject::AppTrigger(msg) => msg.timestamp,
            LogObject::EventComment(msg) => msg.timestamp,
            LogObject::GlobalMarker(msg) => msg.timestamp,
            LogObject::MostSpy(msg) => msg.timestamp,
            LogObject::MostCtrl(msg) => msg.timestamp,
            LogObject::MostPkt2(msg) => msg.timestamp,
            LogObject::MostLightLock(msg) => msg.timestamp,
            LogObject::MostStatistic(msg) => msg.timestamp,
            LogObject::MostHwMode(msg) => msg.timestamp,
            LogObject::MostReg(msg) => msg.timestamp,
            LogObject::MostGenReg(msg) => msg.timestamp,
            LogObject::MostNetState(msg) => msg.timestamp,
            LogObject::MostDataLost(msg) => msg.timestamp,
            LogObject::MostTrigger(msg) => msg.timestamp,
            LogObject::Unhandled { timestamp, .. } => *timestamp,
        }
    }

    /// Overwrite the timestamp stored in this log object. Used by
    /// multi-file merge to rewrite each message's relative timestamp
    /// to a global absolute timestamp (in nanoseconds since Unix epoch)
    /// so that messages from different files sort and display uniformly.
    pub fn set_timestamp(&mut self, ts: u64) {
        match self {
            LogObject::CanMessage(msg) => msg.header.object_time_stamp = ts,
            LogObject::CanMessage2(msg) => msg.header.object_time_stamp = ts,
            LogObject::CanErrorFrame(msg) => msg.header.object_time_stamp = ts,
            LogObject::CanFdMessage(msg) => msg.header.object_time_stamp = ts,
            LogObject::CanFdMessage64(msg) => msg.header.object_time_stamp = ts,
            LogObject::CanOverloadFrame(msg) => msg.header.object_time_stamp = ts,
            LogObject::CanDriverStatistic(msg) => msg.header.object_time_stamp = ts,
            LogObject::CanDriverError(msg) => msg.header.object_time_stamp = ts,
            LogObject::LinMessage(msg) => msg.header.object_time_stamp = ts,
            LogObject::LinMessage2(msg) => msg.header.object_time_stamp = ts,
            LogObject::LinCrcError(msg) => msg.header.object_time_stamp = ts,
            LogObject::LinDlcInfo(msg) => msg.header.object_time_stamp = ts,
            LogObject::LinReceiveError(msg) => msg.header.object_time_stamp = ts,
            LogObject::LinSendError(msg) => msg.header.object_time_stamp = ts,
            LogObject::LinSlaveTimeout(msg) => msg.header.object_time_stamp = ts,
            LogObject::LinSchedulerModeChange(msg) => msg.header.object_time_stamp = ts,
            LogObject::LinSyncError(msg) => msg.header.object_time_stamp = ts,
            LogObject::LinBaudrateEvent(msg) => msg.header.object_time_stamp = ts,
            LogObject::LinSleepModeEvent(msg) => msg.header.object_time_stamp = ts,
            LogObject::LinWakeupEvent(msg) => msg.header.object_time_stamp = ts,
            LogObject::FlexRayData(msg) => msg.timestamp = ts,
            LogObject::FlexRaySync(msg) => msg.timestamp = ts,
            LogObject::FlexRayV6Message(msg) => msg.timestamp = ts,
            LogObject::FlexRayV6StartCycleEvent(msg) => msg.timestamp = ts,
            LogObject::FlexRayStatusEvent(msg) => msg.timestamp = ts,
            LogObject::FlexRayVFrError(msg) => msg.timestamp = ts,
            LogObject::FlexRayVFrStatus(msg) => msg.timestamp = ts,
            LogObject::FlexRayVFrStartCycle(msg) => msg.timestamp = ts,
            LogObject::FlexRayVFrReceiveMsg(msg) => msg.timestamp = ts,
            LogObject::FlexRayVFrReceiveMsgEx(msg) => msg.timestamp = ts,
            LogObject::EthernetFrame(msg) => msg.timestamp = ts,
            LogObject::AppTrigger(msg) => msg.timestamp = ts,
            LogObject::EventComment(msg) => msg.timestamp = ts,
            LogObject::GlobalMarker(msg) => msg.timestamp = ts,
            LogObject::MostSpy(msg) => msg.timestamp = ts,
            LogObject::MostCtrl(msg) => msg.timestamp = ts,
            LogObject::MostPkt2(msg) => msg.timestamp = ts,
            LogObject::MostLightLock(msg) => msg.timestamp = ts,
            LogObject::MostStatistic(msg) => msg.timestamp = ts,
            LogObject::MostHwMode(msg) => msg.timestamp = ts,
            LogObject::MostReg(msg) => msg.timestamp = ts,
            LogObject::MostGenReg(msg) => msg.timestamp = ts,
            LogObject::MostNetState(msg) => msg.timestamp = ts,
            LogObject::MostDataLost(msg) => msg.timestamp = ts,
            LogObject::MostTrigger(msg) => msg.timestamp = ts,
            LogObject::Unhandled { timestamp, .. } => *timestamp = ts,
        }
    }

    /// Returns the `object_flags` of the log object (BLF spec: 0x1=TimeTenMics,
    /// 0x2=TimeOneNans). For object types that don't have an ObjectHeader
    /// (FlexRay, Ethernet, Most, AppTrigger, EventComment, GlobalMarker, Unhandled),
    /// returns 0x2 (TimeOneNans) as a safe default — these types already store
    /// an absolute-nanosecond timestamp in their `timestamp` field (per
    /// `timestamp()` impl above).
    pub fn flags(&self) -> u32 {
        match self {
            LogObject::CanMessage(msg) => msg.header.object_flags,
            LogObject::CanMessage2(msg) => msg.header.object_flags,
            LogObject::CanErrorFrame(msg) => msg.header.object_flags,
            LogObject::CanFdMessage(msg) => msg.header.object_flags,
            LogObject::CanFdMessage64(msg) => msg.header.object_flags,
            LogObject::CanOverloadFrame(msg) => msg.header.object_flags,
            LogObject::CanDriverStatistic(msg) => msg.header.object_flags,
            LogObject::CanDriverError(msg) => msg.header.object_flags,
            LogObject::LinMessage(msg) => msg.header.object_flags,
            LogObject::LinMessage2(msg) => msg.header.object_flags,
            LogObject::LinCrcError(msg) => msg.header.object_flags,
            LogObject::LinDlcInfo(msg) => msg.header.object_flags,
            LogObject::LinReceiveError(msg) => msg.header.object_flags,
            LogObject::LinSendError(msg) => msg.header.object_flags,
            LogObject::LinSlaveTimeout(msg) => msg.header.object_flags,
            LogObject::LinSchedulerModeChange(msg) => msg.header.object_flags,
            LogObject::LinSyncError(msg) => msg.header.object_flags,
            LogObject::LinBaudrateEvent(msg) => msg.header.object_flags,
            LogObject::LinSleepModeEvent(msg) => msg.header.object_flags,
            LogObject::LinWakeupEvent(msg) => msg.header.object_flags,
            _ => 0x2, // FlexRay/Ethernet/Most/etc. store abs_ns already
        }
    }

    /// Returns the timestamp converted to **nanoseconds** based on
    /// `object_flags` (per BLF spec):
    /// - `TimeTenMics` (0x1): raw ticks × 10µs = ticks × 10000 ns
    /// - `TimeOneNans` (0x2) or default: raw ticks = ns
    pub fn timestamp_nanos(&self) -> u64 {
        let raw = self.timestamp();
        let flags = self.flags();
        if flags & 0x1 != 0 {
            // TimeTenMics: 10µs per tick → × 10000 ns
            raw.saturating_mul(10_000)
        } else {
            // TimeOneNans (default): 1ns per tick
            raw
        }
    }

    /// Returns the channel ID of the log object (if applicable)
    pub fn channel(&self) -> Option<u16> {
        match self {
            LogObject::CanMessage(msg) => Some(msg.channel),
            LogObject::CanMessage2(msg) => Some(msg.channel),
            LogObject::CanFdMessage(msg) => Some(msg.channel),
            LogObject::CanFdMessage64(msg) => Some(msg.channel as u16),
            LogObject::LinMessage(msg) => Some(msg.channel),
            LogObject::LinMessage2(_msg) => None, // LinMessage2 doesn't have a direct channel field
            _ => None,
        }
    }
}

/// BLF parser for handling log objects
#[derive(Debug, Default)]
pub struct BlfParser {
    /// Enable debug logging
    pub debug: bool,
}

impl BlfParser {
    /// Creates a new BlfParser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new BlfParser with debug logging enabled.
    pub fn with_debug() -> Self {
        Self { debug: true }
    }

    /// Parses the data slice and returns a vector of log objects and a vector of errors.
    ///
    /// The top level of a BLF file (after the `FileStatistics` header, which the
    /// caller handles) is a series of `LogContainer` objects. Parsing runs in
    /// three phases:
    ///
    /// 1. A sequential scan collects the container headers. Each container is
    ///    followed by `object_size % 4` padding bytes (Vector's rule, see
    ///    [`next_object_pos`]). A stretch of bytes that isn't a container is
    ///    skipped up to the next `LOBJ` signature and reported as one error.
    /// 2. The containers are decompressed in parallel.
    /// 3. The decompressed bytes are joined into one stream and the inner
    ///    objects are parsed from it. The joining matters: Vector writers fill
    ///    each container to a fixed size, so an inner object can start in one
    ///    container and end in the next. To keep this phase parallel, the
    ///    stream is cut into chunks at object boundaries first.
    pub fn parse(&self, data: &[u8]) -> BlfParseResult<(Vec<LogObject>, Vec<BlfParseError>, u64)> {
        let mut all_objects = Vec::new();
        let mut all_errors = Vec::new();
        let data_len = data.len();

        if self.debug {
            println!("Starting to parse {} bytes of BLF data", data_len);
        }

        // Phase 1: find the containers.
        let mut pending_containers: Vec<(usize, ObjectHeaderBase)> = Vec::new();
        let mut pos = 0usize;

        // A container is at least a 16-byte header plus 16 bytes of metadata.
        while pos + 32 <= data_len {
            let mut cursor = Cursor::new(data);
            cursor.set_position(pos as u64);

            let header = match ObjectHeaderBase::read(&mut cursor).context("BlfParser.ObjectHeaderBase") {
                Ok(h) if h.object_size >= h.header_size as u32 && h.object_size >= 16 => h,
                result => {
                    let resume = find_next_signature(data, pos + 1).unwrap_or(data_len);
                    let err = match result {
                        Ok(h) => BlfParseError::UnexpectedData.context(format!(
                            "BlfParser.ObjectHeaderBase.object_size ({} is smaller than the header)",
                            h.object_size
                        )),
                        Err(e) => e,
                    };
                    if self.debug {
                        println!("Bad object header at offset {}: {}", pos, err);
                    }
                    all_errors.push(err.context(format!(
                        "BlfParser@0x{:X} (skipped {} bytes to the next LOBJ)",
                        pos,
                        resume - pos
                    )));
                    pos = resume;
                    continue;
                }
            };

            if header.object_type == ObjectType::LogContainer {
                pending_containers.push((pos, header.clone()));
            } else if self.debug {
                println!(
                    "Non-container object at top level: {:?}, skipping",
                    header.object_type
                );
            }
            pos = next_object_pos(data, pos, header.object_size);
        }

        let consumed = pos.min(data_len) as u64;

        // Phase 2: decompress every container in parallel. `collect()` keeps
        // file order, which the joined stream depends on.
        let decompressed: Vec<BlfParseResult<Vec<u8>>> = pending_containers
            .into_par_iter()
            .map(|(start_pos, header)| {
                // LogContainer::read expects the cursor just past the header.
                let payload_start = start_pos + header.header_size as usize;
                let payload = data.get(payload_start..).unwrap_or(&[]);
                LogContainer::read(&mut Cursor::new(payload), header)
                    .map(|container| container.uncompressed_data)
                    .context(format!("BlfParser.LogContainer@0x{:X}", start_pos))
            })
            .collect();

        // Join the decompressed data. A container that fails to decompress
        // breaks the stream, so the data after it starts a new segment rather
        // than being glued onto bytes it doesn't follow.
        let mut segments: Vec<Vec<u8>> = vec![Vec::new()];
        for result in decompressed {
            match result {
                Ok(mut bytes) => segments.last_mut().unwrap().append(&mut bytes),
                Err(e) => {
                    if self.debug {
                        println!("Error decompressing container: {}", e);
                    }
                    all_errors.push(e);
                    if !segments.last().unwrap().is_empty() {
                        segments.push(Vec::new());
                    }
                }
            }
        }

        // Phase 3: parse the inner objects, chunk by chunk in parallel.
        for segment in segments.iter().filter(|s| !s.is_empty()) {
            let chunks = split_at_object_boundaries(segment, INNER_CHUNK_TARGET);
            let results: Vec<BlfParseResult<Vec<LogObject>>> = chunks
                .into_par_iter()
                .map(|range| {
                    let mut cursor = Cursor::new(&segment[range]);
                    self.parse_inner_objects(&mut cursor)
                        .context("BlfParser.parse_inner_objects")
                })
                .collect();
            for result in results {
                match result {
                    Ok(objects) => all_objects.extend(objects),
                    Err(e) => {
                        if self.debug {
                            println!("Error parsing inner objects: {:?}", e);
                        }
                        all_errors.push(e);
                    }
                }
            }
        }

        if self.debug {
            println!(
                "Parsing complete, found {} objects total, {} errors",
                all_objects.len(),
                all_errors.len()
            );
        }

        Ok((all_objects, all_errors, consumed))
    }

    fn parse_can_object(
        &self,
        cursor: &mut Cursor<&[u8]>,
        header: &ObjectHeader,
        object_data_size: usize,
    ) -> BlfParseResult<Option<LogObject>> {
        match header.object_type {
            ObjectType::CanMessage => Ok(Some(LogObject::CanMessage(CanMessage::read(
                cursor, header,
            )?))),
            ObjectType::CanMessage2 => Ok(Some(LogObject::CanMessage2(CanMessage2::read(
                cursor,
                header,
                object_data_size,
            )?))),
            ObjectType::CanError => Ok(Some(LogObject::CanErrorFrame(CanErrorFrame::read(
                cursor, header,
            )?))),
            ObjectType::CanFdMessage => Ok(Some(LogObject::CanFdMessage(CanFdMessage::read(
                cursor, header,
            )?))),
            ObjectType::CanFdMessage64 => Ok(Some(LogObject::CanFdMessage64(
                CanFdMessage64::read(cursor, header)?,
            ))),
            ObjectType::CanOverload => Ok(Some(LogObject::CanOverloadFrame(
                CanOverloadFrame::read(cursor, header)?,
            ))),
            ObjectType::CanStatistic => Ok(Some(LogObject::CanDriverStatistic(
                CanDriverStatistic::read(cursor, header)?,
            ))),
            ObjectType::CanDriverError => Ok(Some(LogObject::CanDriverError(
                CanDriverError::read(cursor, header)?,
            ))),
            _ => self.parse_lin_object(cursor, header, object_data_size),
        }
    }

    fn parse_lin_object(
        &self,
        cursor: &mut Cursor<&[u8]>,
        header: &ObjectHeader,
        object_data_size: usize,
    ) -> BlfParseResult<Option<LogObject>> {
        match header.object_type {
            ObjectType::LinMessage => Ok(Some(LogObject::LinMessage(LinMessage::read(
                cursor, header,
            )?))),
            ObjectType::LinCrcError => Ok(Some(LogObject::LinCrcError(LinCrcError::read(
                cursor, header,
            )?))),
            ObjectType::LinDlcInfo => Ok(Some(LogObject::LinDlcInfo(LinDlcInfo::read(
                cursor, header,
            )?))),
            ObjectType::LinReceiveError => Ok(Some(LogObject::LinReceiveError(
                LinReceiveError::read(cursor, header)?,
            ))),
            ObjectType::LinSendError => Ok(Some(LogObject::LinSendError(LinSendError::read(
                cursor, header,
            )?))),
            ObjectType::LinSlaveTimeout => Ok(Some(LogObject::LinSlaveTimeout(
                LinSlaveTimeout::read(cursor, header)?,
            ))),
            ObjectType::LinSchedulerModeChange => Ok(Some(LogObject::LinSchedulerModeChange(
                LinSchedulerModeChange::read(cursor, header)?,
            ))),
            ObjectType::LinSyncError => Ok(Some(LogObject::LinSyncError(LinSyncError::read(
                cursor, header,
            )?))),
            ObjectType::LinBaudrate => Ok(Some(LogObject::LinBaudrateEvent(
                LinBaudrateEvent::read(cursor, header)?,
            ))),
            ObjectType::LinSleep => Ok(Some(LogObject::LinSleepModeEvent(
                LinSleepModeEvent::read(cursor, header)?,
            ))),
            ObjectType::LinWakeup => Ok(Some(LogObject::LinWakeupEvent(LinWakeupEvent::read(
                cursor, header,
            )?))),
            ObjectType::LinMessage2 => Ok(Some(LogObject::LinMessage2(LinMessage2::read(
                cursor,
                header,
                object_data_size,
            )?))),
            _ => self.parse_flexray_object(cursor, &header.clone(), object_data_size),
        }
    }
    fn parse_flexray_object(
        &self,
        cursor: &mut Cursor<&[u8]>,
        header: &ObjectHeader,
        object_data_size: usize,
    ) -> BlfParseResult<Option<LogObject>> {
        match header.object_type {
            ObjectType::FlexRayData => Ok(Some(LogObject::FlexRayData(FlexRayData::read(
                cursor, header,
            )?))),
            ObjectType::FlexRaySync => Ok(Some(LogObject::FlexRaySync(FlexRaySync::read(
                cursor, header,
            )?))),
            ObjectType::FlexRayMessage => Ok(Some(LogObject::FlexRayV6Message(
                FlexRayV6Message::read(cursor, header)?,
            ))),
            ObjectType::FlexRayV6StartCycleEvent => Ok(Some(LogObject::FlexRayV6StartCycleEvent(
                FlexRayV6StartCycleEvent::read(cursor, header)?,
            ))),
            ObjectType::FlexRayStatusEvent => Ok(Some(LogObject::FlexRayStatusEvent(
                FlexRayStatusEvent::read(cursor, header)?,
            ))),
            ObjectType::FlexRayVFrError => Ok(Some(LogObject::FlexRayVFrError(
                FlexRayVFrError::read(cursor, header)?,
            ))),
            ObjectType::FlexRayVFrStatus => Ok(Some(LogObject::FlexRayVFrStatus(
                FlexRayVFrStatus::read(cursor, header)?,
            ))),
            ObjectType::FlexRayVFrStartCycle => Ok(Some(LogObject::FlexRayVFrStartCycle(
                FlexRayVFrStartCycle::read(cursor, header)?,
            ))),
            ObjectType::FlexRayVFrReceiveMsg => Ok(Some(LogObject::FlexRayVFrReceiveMsg(
                FlexRayVFrReceiveMsg::read(cursor, header)?,
            ))),
            ObjectType::FlexRayVFrReceiveMsgEx => Ok(Some(LogObject::FlexRayVFrReceiveMsgEx(
                FlexRayVFrReceiveMsgEx::read(cursor, header)?,
            ))),
            _ => self.parse_other_object(cursor, &header.clone(), object_data_size),
        }
    }
    fn parse_other_object(
        &self,
        cursor: &mut Cursor<&[u8]>,
        header: &ObjectHeader,
        object_data_size: usize,
    ) -> BlfParseResult<Option<LogObject>> {
        match header.object_type {
            ObjectType::EthernetFrame => Ok(Some(LogObject::EthernetFrame(EthernetFrame::read(
                cursor, header,
            )?))),
            ObjectType::AppTrigger => Ok(Some(LogObject::AppTrigger(AppTrigger::read(
                cursor, header,
            )?))),
            ObjectType::EventComment => Ok(Some(LogObject::EventComment(EventComment::read(
                cursor, header,
            )?))),
            ObjectType::GlobalMarker => Ok(Some(LogObject::GlobalMarker(GlobalMarker::read(
                cursor, header,
            )?))),
            // Temporarily comment out missing types
            // ObjectType::SystemVariable => Ok(Some(LogObject::SystemVariable(SystemVariable::read(cursor, header)?))),
            // ObjectType::EnvInteger => Ok(Some(LogObject::EnvInteger(EnvInteger::read(cursor, header)?))),
            // ObjectType::EnvDouble => Ok(Some(LogObject::EnvDouble(EnvDouble::read(cursor, header)?))),
            _ => self.parse_most_object(cursor, header.clone(), object_data_size),
        }
    }
    fn parse_most_object(
        &self,
        cursor: &mut Cursor<&[u8]>,
        header: ObjectHeader,
        object_data_size: usize,
    ) -> BlfParseResult<Option<LogObject>> {
        match header.object_type {
            // Temporarily comment out EnvString since it's not yet implemented
            // ObjectType::EnvString => Ok(Some(LogObject::EnvString(EnvString::read(cursor, &header, object_data_size)?))),
            ObjectType::MostSpy => Ok(Some(LogObject::MostSpy(MostSpy::read(cursor, &header)?))),
            ObjectType::MostCtrl => Ok(Some(LogObject::MostCtrl(MostCtrl::read(cursor, &header)?))),
            ObjectType::MostPkt2 => Ok(Some(LogObject::MostPkt2(MostPkt2::read(cursor, &header)?))),
            ObjectType::MostLightLock => Ok(Some(LogObject::MostLightLock(MostLightLock::read(
                cursor, &header,
            )?))),
            ObjectType::MostStatistic => Ok(Some(LogObject::MostStatistic(MostStatistic::read(
                cursor, &header,
            )?))),
            ObjectType::MostHwMode => Ok(Some(LogObject::MostHwMode(MostHwMode::read(
                cursor, &header,
            )?))),
            ObjectType::MostReg => Ok(Some(LogObject::MostReg(MostReg::read(cursor, &header)?))),
            ObjectType::MostGenReg => Ok(Some(LogObject::MostGenReg(MostGenReg::read(
                cursor, &header,
            )?))),
            ObjectType::MostNetState => Ok(Some(LogObject::MostNetState(MostNetState::read(
                cursor, &header,
            )?))),
            ObjectType::MostDataLost => Ok(Some(LogObject::MostDataLost(MostDataLost::read(
                cursor, &header,
            )?))),
            ObjectType::MostTrigger => Ok(Some(LogObject::MostTrigger(MostTrigger::read(
                cursor, &header,
            )?))),
            _ => {
                // Create an Unhandled object for unknown types
                let mut data = vec![0u8; object_data_size];
                cursor.read_exact(&mut data)?;
                Ok(Some(LogObject::Unhandled {
                    object_type: header.object_type as u32,
                    timestamp: header.object_time_stamp,
                    data,
                }))
            }
        }
    }

    /// Parses the actual log objects contained within a (decompressed) LogContainer.
    fn parse_inner_objects(&self, cursor: &mut Cursor<&[u8]>) -> BlfParseResult<Vec<LogObject>> {
        let mut all_objects = Vec::new();
        let mut all_errors = Vec::new();
        let data_len = cursor.get_ref().len();

        while (cursor.position() as usize) < data_len {
            let start_pos = cursor.position();

            // Check if we have enough bytes to read the signature
            if (data_len as u64 - start_pos) < 4 {
                break;
            }

            // Try to read the header, but handle the case where there's no valid object left.
            // The error comes back wrapped in context, so match on the innermost cause.
            let header = match ObjectHeader::read(cursor).context("BlfParser.ObjectHeader") {
                Ok(header) => header,
                Err(e) => match root_cause(&e) {
                    BlfParseError::InvalidContainerMagic => {
                        // Not at an object: jump to the next signature.
                        let data = *cursor.get_ref();
                        let resume = find_next_signature(data, start_pos as usize + 1).unwrap_or(data_len);
                        cursor.set_position(resume as u64);
                        continue;
                    }
                    // Trailing bytes too short for a header: return what we have.
                    BlfParseError::UnexpectedEof => break,
                    _ => {
                        all_errors.push(e);
                        cursor.set_position(start_pos + 4);
                        continue;
                    }
                },
            };

            // LogContainers should not be nested. If they are, we skip them to avoid infinite recursion.
            if header.object_type != ObjectType::LogContainer {
                let object_body_size = (header.object_size as usize)
                    .saturating_sub(header.calculate_header_size() as usize);

                match self.parse_can_object(cursor, &header, object_body_size) {
                    Ok(Some(object)) => {
                        all_objects.push(object);
                    }
                    Ok(None) => {
                        // Object type not handled, skip
                    }
                    Err(BlfParseError::UnexpectedEof) => {
                        // End of file reached while parsing object body
                        all_errors.push(BlfParseError::UnexpectedEof);
                        break;
                    }
                    Err(e) => {
                        // Skip the bad object and carry on with the next one.
                        all_errors.push(e);
                    }
                }
            } else {
                // For LogContainer objects, we skip them but still need to advance the cursor
                if self.debug {
                    println!("Skipping LogContainer object");
                }
            }

            let data = *cursor.get_ref();
            let next_pos = next_object_pos(data, start_pos as usize, header.object_size);
            // Always move forward, even for a corrupt object_size of 0.
            cursor.set_position(next_pos.max(start_pos as usize + 1) as u64);
        }

        // If we have errors but also successfully parsed objects, return the objects
        // Otherwise, if we have no objects but have errors, return the first error
        if !all_objects.is_empty() {
            Ok(all_objects)
        } else if !all_errors.is_empty() {
            Err(all_errors.into_iter().next().unwrap())
        } else {
            Ok(all_objects)
        }
    }

    #[allow(dead_code)]
    fn parse_unhandled_object(&self) -> Option<LogObject> {
        // The `parse_objects` loop handles advancing the cursor to the next object.
        // For unhandled or deprecated object types, we simply do nothing and return `None`.
        //
        // This includes:
        // - ObjectType::MostPkt: Deprecated and should be skipped.
        // - Any other unknown ObjectType.
        None
    }
}

/// The 4-byte signature that starts every BLF object ("LOBJ", 0x4A424F4C little-endian).
const OBJECT_SIGNATURE: &[u8; 4] = b"LOBJ";

/// Rough size of each chunk of the joined inner-object stream handed to a
/// worker thread in phase 3 of [`BlfParser::parse`].
const INNER_CHUNK_TARGET: usize = 1 << 20;

fn has_signature_at(data: &[u8], pos: usize) -> bool {
    data.get(pos..pos + 4) == Some(&OBJECT_SIGNATURE[..])
}

/// Position of the next `LOBJ` signature at or after `from`, if any.
fn find_next_signature(data: &[u8], from: usize) -> Option<usize> {
    data.get(from..)?
        .windows(4)
        .position(|w| w == OBJECT_SIGNATURE)
        .map(|i| from + i)
}

/// Start of the object after the one at `start`, clamped to `data.len()`.
///
/// Vector's own reader skips `object_size % 4` padding bytes after an object
/// (`is.seekg(objectSize % 4, std::ios_base::cur)`), and real files are
/// written that way. That is NOT the same as rounding up to a multiple of 4:
/// a 21,521-byte container is followed by 1 padding byte, not 3. Getting this
/// wrong lands the parser 2 bytes off, where it never finds a `LOBJ` again.
///
/// Other readers special-case some object types (python-can skips the
/// padding for `CAN_FD_MESSAGE_64`), and this crate's own test writers round
/// up instead. So when the `% 4`
/// position doesn't hold a signature but one of the alternatives does, the
/// alternative wins.
fn next_object_pos(data: &[u8], start: usize, object_size: u32) -> usize {
    let size = object_size as usize;
    let candidates = [
        start + size + size % 4,  // Vector's rule
        start + size,             // no padding
        start + ((size + 3) & !3), // rounded up to a multiple of 4
    ];
    candidates
        .iter()
        .copied()
        .find(|&p| has_signature_at(data, p))
        .unwrap_or(candidates[0])
        .min(data.len())
}

/// Splits a stream of inner objects into ranges of roughly `target` bytes
/// that each start at an object boundary, so they can be parsed in parallel.
///
/// Only the object headers are read. If the walk hits bytes that aren't an
/// object, everything from there on goes into the last range, where
/// `parse_inner_objects` resynchronises on its own.
fn split_at_object_boundaries(data: &[u8], target: usize) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut chunk_start = 0;
    let mut pos = 0;
    while pos + 16 <= data.len() && has_signature_at(data, pos) {
        if pos - chunk_start >= target {
            ranges.push(chunk_start..pos);
            chunk_start = pos;
        }
        let size = u32::from_le_bytes(data[pos + 8..pos + 12].try_into().unwrap());
        if size < 16 {
            break;
        }
        pos = next_object_pos(data, pos, size);
    }
    ranges.push(chunk_start..data.len());
    ranges
}

/// The innermost error under any `Context` wrappers.
fn root_cause(err: &BlfParseError) -> &BlfParseError {
    match err {
        BlfParseError::Context { inner, .. } => root_cause(inner),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_parse_inner_objects_single_can_message() {
        let parser = BlfParser::new();
        let can_message = CanMessage {
            header: ObjectHeader {
                base: crate::objects::object_header::ObjectHeaderBase {
                    signature: 0x4A424F4C, // "LOBJ"
                    header_size: 32,
                    header_version: 1,
                    object_size: 48, // header + can_msg_fields + data
                    object_type: ObjectType::CanMessage,
                },
                object_flags: 0,
                client_index: 0,
                object_version: 0,
                object_time_stamp: 1000,
                original_time_stamp: None,
                time_stamp_status: None,
                reserved: 0,
            },
            channel: 1,
            flags: 0,
            dlc: 8,
            id: 0x123,
            data: [1, 2, 3, 4, 5, 6, 7, 8],
        };

        // Create a LogContainer to wrap the CAN message
        let mut can_msg_bytes = serialize_can_message(&can_message);
        println!("can_msg_bytes before padding: {:?}", can_msg_bytes);
        add_padding(&mut can_msg_bytes);
        println!("can_msg_bytes after padding: {:?}", can_msg_bytes);

        let mut cursor = Cursor::new(&can_msg_bytes[..]);
        let result = parser.parse_inner_objects(&mut cursor).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], LogObject::CanMessage(can_message));
    }

    #[test]
    fn test_parse_inner_objects_multiple_objects() {
        let parser = BlfParser::new();
        let can_message1 = CanMessage {
            header: ObjectHeader {
                base: crate::objects::object_header::ObjectHeaderBase {
                    signature: 0x4A424F4C, // "LOBJ"
                    header_size: 32,
                    header_version: 1,
                    object_size: 48,
                    object_type: ObjectType::CanMessage,
                },
                object_flags: 0,
                client_index: 0,
                object_version: 0,
                object_time_stamp: 1000,
                original_time_stamp: None,
                time_stamp_status: None,
                reserved: 0,
            },
            channel: 1,
            flags: 0,
            dlc: 8,
            id: 0x111,
            data: [0; 8],
        };
        let can_message2 = CanMessage {
            header: ObjectHeader {
                base: crate::objects::object_header::ObjectHeaderBase {
                    signature: 0x4A424F4C, // "LOBJ"
                    header_size: 32,
                    header_version: 1,
                    object_size: 48,
                    object_type: ObjectType::CanMessage,
                },
                object_flags: 0,
                client_index: 0,
                object_version: 0,
                object_time_stamp: 2000,
                original_time_stamp: None,
                time_stamp_status: None,
                reserved: 0,
            },
            channel: 2,
            flags: 0,
            dlc: 8,
            id: 0x222,
            data: [1; 8],
        };

        // Create a LogContainer to wrap the CAN messages
        let mut bytes1 = serialize_can_message(&can_message1);
        println!("bytes1 before padding: {:?}", bytes1);
        add_padding(&mut bytes1);
        println!("bytes1 after padding: {:?}", bytes1);
        let mut bytes2 = serialize_can_message(&can_message2);
        println!("bytes2 before padding: {:?}", bytes2);
        add_padding(&mut bytes2);
        println!("bytes2 after padding: {:?}", bytes2);

        let combined_bytes = [bytes1, bytes2].concat();
        println!("combined_bytes: {:?}", combined_bytes);

        let mut cursor = Cursor::new(&combined_bytes[..]);
        let result = parser.parse_inner_objects(&mut cursor).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], LogObject::CanMessage(can_message1));
        assert_eq!(result[1], LogObject::CanMessage(can_message2));
    }

    #[test]
    fn test_parse_inner_objects_keeps_unknown_object_as_unhandled() {
        use crate::ObjectType;
        use std::io::Write;

        let parser = BlfParser::new();

        // Create a LogContainer to wrap the unknown object
        let unknown_header = ObjectHeader {
            base: crate::objects::object_header::ObjectHeaderBase {
                signature: 0x4A424F4C, // "LOBJ"
                header_size: 32,
                header_version: 1,
                object_size: 38, // Does not need to be a multiple of 4
                object_type: ObjectType::Unknown,
            },
            object_flags: 0,
            client_index: 0,
            object_version: 0,
            object_time_stamp: 1000,
            original_time_stamp: None,
            time_stamp_status: None,
            reserved: 0,
        };

        let mut unknown_object_bytes = Vec::new();
        serialize_object_header(&unknown_header, &mut unknown_object_bytes);
        unknown_object_bytes
            .write_all(&vec![
                0;
                (unknown_header.object_size - unknown_header.header_size as u32)
                    as usize
            ])
            .unwrap();
        add_padding(&mut unknown_object_bytes);

        let mut cursor = Cursor::new(&unknown_object_bytes[..]);
        let result = parser.parse_inner_objects(&mut cursor).unwrap();

        // Unknown objects are kept as `Unhandled`, with their type, timestamp
        // and raw body, rather than being dropped.
        assert_eq!(result.len(), 1);
        match &result[0] {
            LogObject::Unhandled { object_type, timestamp, data } => {
                assert_eq!(*object_type, ObjectType::Unknown as u32);
                assert_eq!(*timestamp, 1000);
                assert_eq!(data.len(), 38 - 32);
            }
            other => panic!("expected Unhandled, got {:?}", other),
        }
    }
    fn test_can_message(id: u32, time_stamp: u64) -> CanMessage {
        CanMessage {
            header: ObjectHeader {
                base: crate::objects::object_header::ObjectHeaderBase {
                    signature: 0x4A424F4C,
                    header_size: 32,
                    header_version: 1,
                    object_size: 48,
                    object_type: ObjectType::CanMessage,
                },
                object_flags: 0,
                client_index: 0,
                object_version: 0,
                object_time_stamp: time_stamp,
                original_time_stamp: None,
                time_stamp_status: None,
                reserved: 0,
            },
            channel: 1,
            flags: 0,
            dlc: 8,
            id,
            data: [id as u8; 8],
        }
    }

    /// Builds a zlib LogContainer around `payload` the way Vector writes it:
    /// `object_size` is the exact size (often odd) and `object_size % 4`
    /// padding bytes follow.
    fn vector_style_container(payload: &[u8]) -> Vec<u8> {
        use byteorder::{LittleEndian, WriteBytesExt};
        use flate2::{write::ZlibEncoder, Compression};
        use std::io::Write;

        // Stored (uncompressed) zlib blocks: the size is payload + 11, so
        // the container sizes in these tests are predictable.
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::none());
        encoder.write_all(payload).unwrap();
        let compressed = encoder.finish().unwrap();

        let object_size = (16 + 16 + compressed.len()) as u32;
        let mut out = Vec::new();
        out.write_u32::<LittleEndian>(0x4A424F4C).unwrap();
        out.write_u16::<LittleEndian>(16).unwrap(); // header_size
        out.write_u16::<LittleEndian>(1).unwrap(); // header_version
        out.write_u32::<LittleEndian>(object_size).unwrap();
        out.write_u32::<LittleEndian>(ObjectType::LogContainer as u32).unwrap();
        out.write_u16::<LittleEndian>(2).unwrap(); // zlib
        out.write_u16::<LittleEndian>(0).unwrap();
        out.write_u32::<LittleEndian>(0).unwrap();
        out.write_u32::<LittleEndian>(payload.len() as u32).unwrap();
        out.write_u32::<LittleEndian>(0).unwrap();
        out.extend_from_slice(&compressed);
        out.extend(std::iter::repeat(0u8).take(object_size as usize % 4));
        out
    }

    #[test]
    fn test_parse_containers_with_odd_sizes_use_vector_padding() {
        let messages: Vec<CanMessage> = (0..3).map(|i| test_can_message(0x100 + i, 1000 * i as u64)).collect();
        let mut data = Vec::new();
        for msg in &messages {
            let container = vector_style_container(&serialize_can_message(msg));
            // 32 + (48 + 11) = 91 bytes, so 3 padding bytes. Rounding up to
            // a multiple of 4 would skip only 1 and miss the next container.
            assert_eq!((container.len() - 3) % 4, 3);
            data.extend_from_slice(&container);
        }

        let (objects, errors, consumed) = BlfParser::new().parse(&data).unwrap();

        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(consumed, data.len() as u64);
        let expected: Vec<LogObject> = messages.into_iter().map(LogObject::CanMessage).collect();
        assert_eq!(objects, expected);
    }

    #[test]
    fn test_parse_object_split_across_containers() {
        let first = test_can_message(0x111, 1000);
        let second = test_can_message(0x222, 2000);
        let third = test_can_message(0x333, 3000);
        let stream = [
            serialize_can_message(&first),
            serialize_can_message(&second),
            serialize_can_message(&third),
        ]
        .concat();

        // Cut the stream in the middle of `second`, as Vector writers do when
        // a container fills up.
        let cut = 48 + 20;
        let data = [
            vector_style_container(&stream[..cut]),
            vector_style_container(&stream[cut..]),
        ]
        .concat();

        let (objects, errors, _) = BlfParser::new().parse(&data).unwrap();

        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(
            objects,
            vec![
                LogObject::CanMessage(first),
                LogObject::CanMessage(second),
                LogObject::CanMessage(third),
            ]
        );
    }

    #[test]
    fn test_parse_reports_one_error_per_garbage_region() {
        let msg = test_can_message(0x123, 1000);
        let container = vector_style_container(&serialize_can_message(&msg));
        let data = [container.clone(), vec![0xAB; 4000], container].concat();

        let (objects, errors, _) = BlfParser::new().parse(&data).unwrap();

        assert_eq!(objects.len(), 2);
        assert_eq!(errors.len(), 1, "errors: {:?}", errors);
        assert!(errors[0].to_string().contains("skipped 4000 bytes"), "{}", errors[0]);
    }

    #[test]
    fn test_split_at_object_boundaries_cuts_only_at_objects() {
        let stream: Vec<u8> = (0..10).flat_map(|i| serialize_can_message(&test_can_message(i, 0))).collect();
        let ranges = split_at_object_boundaries(&stream, 100);

        assert_eq!(ranges.first().unwrap().start, 0);
        assert_eq!(ranges.last().unwrap().end, stream.len());
        for pair in ranges.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
        }
        for r in &ranges {
            assert_eq!(r.start % 48, 0);
        }
        assert!(ranges.len() > 1);
    }
}
