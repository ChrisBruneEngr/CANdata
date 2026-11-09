use blf::{
    CanMessage, FileStatistics, LogContainer, ObjectHeader, ObjectHeaderBase, ObjectType,
    SystemTime,
};
use std::fs::File;
use std::io::Write;

/// Helper to write a SystemTime struct to a writer.
fn write_system_time(time: &SystemTime, writer: &mut impl Write) {
    use byteorder::{LittleEndian, WriteBytesExt};
    writer.write_u16::<LittleEndian>(time.year).unwrap();
    writer.write_u16::<LittleEndian>(time.month).unwrap();
    writer.write_u16::<LittleEndian>(time.day_of_week).unwrap();
    writer.write_u16::<LittleEndian>(time.day).unwrap();
    writer.write_u16::<LittleEndian>(time.hour).unwrap();
    writer.write_u16::<LittleEndian>(time.minute).unwrap();
    writer.write_u16::<LittleEndian>(time.second).unwrap();
    writer.write_u16::<LittleEndian>(time.milliseconds).unwrap();
}

/// Helper to serialize a FileStatistics struct into bytes.
fn serialize_file_statistics(stats: &FileStatistics) -> Vec<u8> {
    use byteorder::{LittleEndian, WriteBytesExt};
    let mut writer = Vec::new();
    writer.write_u32::<LittleEndian>(0x47474F4C).unwrap(); // Signature "LOGG"
    writer
        .write_u32::<LittleEndian>(stats.statistics_size)
        .unwrap();
    writer.write_u32::<LittleEndian>(stats.api_number).unwrap();
    writer.write_u8(stats.application_id).unwrap();
    writer.write_u8(stats.compression_level).unwrap();
    writer.write_u8(stats.application_major).unwrap();
    writer.write_u8(stats.application_minor).unwrap();
    writer.write_u64::<LittleEndian>(stats.file_size).unwrap();
    writer
        .write_u64::<LittleEndian>(stats.uncompressed_file_size)
        .unwrap();
    writer
        .write_u32::<LittleEndian>(stats.object_count)
        .unwrap();
    writer
        .write_u32::<LittleEndian>(stats.application_build)
        .unwrap();
    write_system_time(&stats.measurement_start_time, &mut writer);
    write_system_time(&stats.last_object_time, &mut writer);

    // Ensure the final size matches statistics_size by padding if necessary.
    // Current size should be 72 bytes, need to pad to statistics_size (144 or 208)
    if writer.len() < stats.statistics_size as usize {
        let padding_needed = stats.statistics_size as usize - writer.len();
        writer.write_all(&vec![0; padding_needed]).unwrap();
    } else if writer.len() > stats.statistics_size as usize {
        // Truncate to expected size
        writer.truncate(stats.statistics_size as usize);
    }

    writer
}

/// Helper to serialize an ObjectHeader struct into bytes.
fn serialize_object_header(header: &ObjectHeader, writer: &mut impl Write) {
    use byteorder::{LittleEndian, WriteBytesExt};
    writer
        .write_u32::<LittleEndian>(header.base.signature)
        .unwrap();
    writer
        .write_u16::<LittleEndian>(header.base.header_size)
        .unwrap();
    writer
        .write_u16::<LittleEndian>(header.base.header_version)
        .unwrap();
    writer
        .write_u32::<LittleEndian>(header.base.object_size)
        .unwrap();
    writer
        .write_u32::<LittleEndian>(header.base.object_type as u32)
        .unwrap();
    writer
        .write_u32::<LittleEndian>(header.object_flags)
        .unwrap();

    if header.base.header_version == 1 {
        writer
            .write_u16::<LittleEndian>(header.client_index)
            .unwrap();
        writer
            .write_u16::<LittleEndian>(header.object_version)
            .unwrap();
        writer
            .write_u64::<LittleEndian>(header.object_time_stamp)
            .unwrap();
    } else if header.header_version == 2 {
        writer
            .write_u8(header.time_stamp_status.unwrap_or(0))
            .unwrap();
        writer.write_u8(0).unwrap(); // reserved
        writer
            .write_u16::<LittleEndian>(header.object_version)
            .unwrap();
        writer
            .write_u64::<LittleEndian>(header.object_time_stamp)
            .unwrap();
        writer
            .write_u64::<LittleEndian>(header.original_time_stamp.unwrap_or(0))
            .unwrap();
    }
}

/// Helper to serialize an ObjectHeaderBase struct into bytes (for LogContainer).
fn serialize_object_header_base(header: &ObjectHeaderBase, writer: &mut impl Write) {
    use byteorder::{LittleEndian, WriteBytesExt};
    writer.write_u32::<LittleEndian>(header.signature).unwrap();
    writer
        .write_u16::<LittleEndian>(header.header_size)
        .unwrap();
    writer
        .write_u16::<LittleEndian>(header.header_version)
        .unwrap();
    writer
        .write_u32::<LittleEndian>(header.object_size)
        .unwrap();
    writer
        .write_u32::<LittleEndian>(header.object_type as u32)
        .unwrap();
}

/// Helper to serialize a CanMessage object into bytes (including header).
fn serialize_can_message(msg: &CanMessage) -> Vec<u8> {
    use byteorder::{LittleEndian, WriteBytesExt};
    let mut writer = Vec::new();
    serialize_object_header(&msg.header, &mut writer);
    writer.write_u16::<LittleEndian>(msg.channel).unwrap();
    writer.write_u8(msg.flags).unwrap();
    writer.write_u8(msg.dlc).unwrap();
    writer.write_u32::<LittleEndian>(msg.id).unwrap();
    writer.write_all(&msg.data).unwrap();

    // Manual padding calculation to match object_size if needed
    let current_len = writer.len();
    if current_len < msg.header.object_size as usize {
        let padding = msg.header.object_size as usize - current_len;
        writer.write_all(&vec![0; padding]).unwrap();
    }

    writer
}

/// Helper to serialize a LogContainer into bytes (including header).
fn serialize_log_container(container: &LogContainer) -> Vec<u8> {
    use byteorder::{LittleEndian, WriteBytesExt};
    let mut writer = Vec::new();
    serialize_object_header_base(&container.header, &mut writer);
    writer
        .write_u16::<LittleEndian>(container.compression_method)
        .unwrap();
    writer.write_u16::<LittleEndian>(0).unwrap(); // _reserved1
    writer.write_u32::<LittleEndian>(0).unwrap(); // _reserved2
    writer
        .write_u32::<LittleEndian>(container.uncompressed_data.len() as u32)
        .unwrap();
    writer.write_u32::<LittleEndian>(0).unwrap(); // _reserved3
    writer.write_all(&container.uncompressed_data).unwrap();

    // Ensure the final size matches object_size by padding if necessary.
    let current_size = writer.len();
    let expected_size = container.header.object_size as usize;
    if current_size < expected_size {
        let padding_needed = expected_size - current_size;
        writer.write_all(&vec![0; padding_needed]).unwrap();
    } else if current_size > expected_size {
        // Truncate to expected size
        writer.truncate(expected_size);
    }

    writer
}

fn main() {
    let filename = "test_corrupted.blf";
    println!("🔧 Generating BLF file with a corrupted tail: {}", filename);

    let start_time = SystemTime {
        year: 2025,
        month: 1,
        day_of_week: 1,
        day: 15,
        hour: 14,
        minute: 30,
        second: 0,
        milliseconds: 0,
    };

    // 1. Create valid file statistics
    let stats = FileStatistics {
        statistics_size: 144, // standard BLF header size
        api_number: 0,
        application_id: 1,
        compression_level: 0,
        application_major: 1,
        application_minor: 0,
        file_size: 0, // updated later
        uncompressed_file_size: 0,
        object_count: 0,
        application_build: 0,
        measurement_start_time: start_time.clone(),
        last_object_time: start_time.clone(),
    };

    // 2. Create 20 valid CAN messages
    let mut messages_bytes = Vec::new();
    let object_count = 20;

    for i in 0..object_count {
        let header = ObjectHeader {
            base: blf::ObjectHeaderBase {
                signature: 0x4A424F4C, // LOBJ
                header_size: 32,
                header_version: 1,
                object_size: 48,
                object_type: ObjectType::CanMessage,
            },
            object_flags: 1,
            client_index: 0,
            object_version: 0,
            object_time_stamp: (i as u64 + 1) * 1000000,
            original_time_stamp: None,
            time_stamp_status: None,
            reserved: 0,
        };

        let msg = CanMessage {
            header,
            channel: 1,
            flags: 0,
            dlc: 8,
            id: 0x100 + i as u32,
            data: [
                i as u8,
                i as u8 + 1,
                i as u8 + 2,
                i as u8 + 3,
                i as u8 + 4,
                i as u8 + 5,
                i as u8 + 6,
                i as u8 + 7,
            ],
        };

        let msg_bytes = serialize_can_message(&msg);
        messages_bytes.extend_from_slice(&msg_bytes);
    }

    // 3. Wrap them in a LogContainer
    let container_header_size = 32;
    let container_extra_size = 16;
    let container_data_size = messages_bytes.len();
    let container_total_size = container_header_size + container_extra_size + container_data_size;
    let padding = (4 - (container_total_size % 4)) % 4;
    let final_container_size = container_total_size + padding;

    let container_header = ObjectHeader {
        base: blf::ObjectHeaderBase {
            signature: 0x4A424F4C,
            header_size: 32,
            header_version: 1,
            object_size: final_container_size as u32,
            object_type: ObjectType::LogContainer,
        },
        object_flags: 0,
        client_index: 0,
        object_version: 0,
        object_time_stamp: 0,
        original_time_stamp: None,
        time_stamp_status: None,
        reserved: 0,
    };

    let container = LogContainer {
        header: container_header.base,
        compression_method: 0,
        uncompressed_data: messages_bytes,
    };

    let container_bytes = serialize_log_container(&container);

    // 4. Create the corrupted tail data
    // Add several different kinds of corruption to exercise error handling
    let mut corrupted_tail = Vec::new();

    // Corruption type 1: incomplete object header (only part of the magic number)
    println!("  - Adding corruption type 1: incomplete object header");
    use byteorder::{LittleEndian, WriteBytesExt};
    corrupted_tail.write_u32::<LittleEndian>(0x4A424F).unwrap(); // only part of LOBJ

    // Corruption type 2: wrong magic number
    println!("  - Adding corruption type 2: wrong magic number");
    corrupted_tail
        .write_u32::<LittleEndian>(0xDEADBEEF)
        .unwrap(); // invalid signature
    corrupted_tail.write_u16::<LittleEndian>(32).unwrap();
    corrupted_tail.write_u16::<LittleEndian>(1).unwrap();
    corrupted_tail.write_u32::<LittleEndian>(48).unwrap();

    // Corruption type 3: declared size far larger than the data available (causes UnexpectedEof)
    println!("  - Adding corruption type 3: object size too large");
    corrupted_tail
        .write_u32::<LittleEndian>(0x4A424F4C)
        .unwrap(); // LOBJ
    corrupted_tail.write_u16::<LittleEndian>(32).unwrap();
    corrupted_tail.write_u16::<LittleEndian>(1).unwrap();
    corrupted_tail.write_u32::<LittleEndian>(999999).unwrap(); // declares a very large size
    corrupted_tail
        .write_u32::<LittleEndian>(ObjectType::CanMessage as u32)
        .unwrap();
    // not enough data follows

    // Corruption type 4: random garbage data
    println!("  - Adding corruption type 4: random garbage data");
    corrupted_tail.extend_from_slice(&[0xFF, 0xAA, 0x55, 0x00, 0x12, 0x34, 0x56, 0x78]);

    // 5. Write the file
    let mut file = File::create(filename).expect("Failed to create file");

    // Update the statistics
    let mut final_stats = stats.clone();
    final_stats.file_size =
        stats.statistics_size as u64 + container_bytes.len() as u64 + corrupted_tail.len() as u64;
    final_stats.uncompressed_file_size = final_stats.file_size;
    final_stats.object_count = object_count;

    let final_stats_bytes = serialize_file_statistics(&final_stats);

    // Write each part
    file.write_all(&final_stats_bytes)
        .expect("Failed to write statistics");
    file.write_all(&container_bytes).expect("Failed to write container");
    file.write_all(&corrupted_tail).expect("Failed to write corrupted data");

    println!("\n✅ Generated {}", filename);
    println!("   The file contains:");
    println!("   - {} valid CAN messages", object_count);
    println!("   - 4 different kinds of corrupted data");
    println!("   - Expected result: {} messages parsed, 4 errors reported", object_count);
    println!("\n📝 How to use:");
    println!("   1. Start CANdata");
    println!("   2. Choose File → Open BLF...");
    println!("   3. Select {}", filename);
    println!("   4. Check the status bar and the Log view: the parsed messages and the errors should both appear\n");
}
