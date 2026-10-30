//! Isolated context-lifetime control, not a repaired MTU or resource claim.
use super::*;

#[test]
fn retains_context_for_its_writer() {
    let mut writer = CompressedStreamWriter::with_block_threshold(64);
    assert!(writer.compression_context.is_none());
    writer.append("first job ".repeat(30).as_str());
    assert!(
        writer.compression_context.is_some(),
        "source control has no retained context"
    );
    let first = std::ptr::from_mut(writer.compression_context.as_mut().unwrap().context_mut());
    writer.append("second job ".repeat(30).as_str());
    let second = std::ptr::from_mut(writer.compression_context.as_mut().unwrap().context_mut());
    assert_eq!(first, second);
    let raw = writer.finish();
    let decoded: Vec<String> = deserialize_stream(&mut raw.as_slice()).unwrap();
    assert_eq!(decoded, ["first job ".repeat(30), "second job ".repeat(30)]);
}

#[test]
fn buffer_error_then_next_job_has_exact_standalone_bytes() {
    let mut context = None;
    assert!(compress_block_with_context(&mut context, &[0x71; 1024], &mut [0u8; 1]).is_err());
    let mut output = [0u8; 256];
    let mut reference = [0u8; 256];
    let n = compress_block_with_context(&mut context, b"second job", &mut output).unwrap();
    let m = zstd::bulk::compress_to_buffer(b"second job", &mut reference, 0).unwrap();
    assert_eq!(&output[..n], &reference[..m]);
    assert_eq!(zstd::decode_all(&output[..n]).unwrap(), b"second job");
}

#[test]
fn independent_writers_keep_their_own_frames() {
    let workers: Vec<_> = (0..4)
        .map(|i| {
            std::thread::spawn(move || {
                let value = format!("owner-{i}-").repeat(100);
                let mut writer = CompressedStreamWriter::with_block_threshold(100);
                writer.append(value.as_str());
                let bytes = writer.finish();
                let decoded: Vec<String> = deserialize_stream(&mut bytes.as_slice()).unwrap();
                assert_eq!(decoded, [value]);
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
}
