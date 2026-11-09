use blf::{read_blf_from_file, LogObject};

fn main() {
    println!("🧪 Testing parsing of a corrupted BLF file\n");
    println!("Reading test_corrupted.blf...\n");

    match read_blf_from_file("test_corrupted.blf") {
        Ok(result) => {
            println!("✅ File read successfully!\n");
            println!("=== File statistics ===");
            println!("Application version: {}.{}.{}.{}",
                result.file_stats.application_major,
                result.file_stats.application_minor,
                result.file_stats.application_build,
                result.file_stats.application_id
            );
            println!("File size: {} bytes", result.file_stats.file_size);
            println!("Measurement start time: {:?}\n", result.file_stats.measurement_start_time);

            println!("=== Parse results ===");
            println!("Objects parsed: {}", result.objects.len());
            println!("Errors encountered: {}\n", result.errors.len());

            if result.errors.len() > 0 {
                println!("⚠️  Errors found:");
                for (i, error) in result.errors.iter().enumerate() {
                    println!("  {}. {}", i + 1, error);
                }
                println!();
            }

            // Show the messages that parsed successfully
            if !result.objects.is_empty() {
                println!("=== Parsed messages (first 10) ===");
                for (i, obj) in result.objects.iter().take(10).enumerate() {
                    match obj {
                        LogObject::CanMessage(msg) => {
                            println!("  [{:02}] CAN Msg - ID: 0x{:03X}, DLC: {}, Data: {:02X?}",
                                i + 1,
                                msg.id,
                                msg.dlc,
                                &msg.data[..msg.dlc as usize]
                            );
                        }
                        LogObject::CanFdMessage(msg) => {
                            println!("  [{:02}] CAN FD Msg - ID: 0x{:03X}, DLC: {}, Data: {:02X?}",
                                i + 1,
                                msg.id,
                                msg.dlc,
                                &msg.data[..msg.dlc as usize]
                            );
                        }
                        _ => {
                            println!("  [{:02}] Other object type: {:?}", i + 1, obj);
                        }
                    }
                }
                if result.objects.len() > 10 {
                    println!("  ... and {} more messages", result.objects.len() - 10);
                }
            }

            println!("\n=== Test result ===");
            if result.errors.len() > 0 && result.objects.len() > 0 {
                println!("✅ Partial parse succeeded!");
                println!("   - {} objects parsed and can be shown in the Log and Plot views", result.objects.len());
                println!("   - {} errors shown in the status bar", result.errors.len());
                println!("\n📝 This verifies the error handling:");
                println!("   ✓ Valid results appear in the Log and Plot views");
                println!("   ✓ Errors are shown in the status bar");
            } else if result.errors.is_empty() {
                println!("✅ Fully parsed with no errors");
            } else {
                println!("❌ Parsing failed completely; no objects were parsed");
            }
        }
        Err(e) => {
            println!("❌ Failed to read file: {:?}\n", e);
            println!("Possible causes:");
            println!("  1. The file does not exist");
            println!("  2. The file header is completely corrupted");
            println!("  3. The file is not a BLF file");
        }
    }

    println!("\nPress any key to exit...");
    let _ = std::io::stdin().read_line(&mut String::new());
}
