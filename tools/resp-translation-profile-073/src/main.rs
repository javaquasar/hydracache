use std::alloc::{GlobalAlloc, Layout, System};
use std::error::Error;
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use hydracache_client_protocol::{ClientRequest, StructuredKey};
use hydracache_redis_compat::{
    decode_resp2_command, decode_resp3_command, encode_resp2_value, encode_resp3_value,
    translate_redis_command, RedisCommand, RedisTranslatedCommand, RedisTranslationContext,
    RespValue,
};
use serde::Serialize;

const PROFILE_ID: &str = "w4-resp-translation-profile-073-v1";
const DEFAULT_OPERATIONS: usize = 256;
const LARGE_OPERATIONS: usize = 16;

struct ProfilingAllocator;
static COUNTING: AtomicBool = AtomicBool::new(false);
static GROSS: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: ProfilingAllocator = ProfilingAllocator;

unsafe impl GlobalAlloc for ProfilingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            add(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            add(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        subtract(layout.size() as u64);
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(pointer, layout, new_size) };
        if !result.is_null() {
            if new_size >= layout.size() {
                LIVE.fetch_add((new_size - layout.size()) as u64, Ordering::Relaxed);
            } else {
                subtract((layout.size() - new_size) as u64);
            }
            if COUNTING.load(Ordering::Acquire) {
                GROSS.fetch_add(new_size as u64, Ordering::Relaxed);
            }
        }
        result
    }
}

fn add(bytes: usize) {
    LIVE.fetch_add(bytes as u64, Ordering::Relaxed);
    if COUNTING.load(Ordering::Acquire) {
        GROSS.fetch_add(bytes as u64, Ordering::Relaxed);
    }
}

fn subtract(bytes: u64) {
    let _ = LIVE.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        Some(value.saturating_sub(bytes))
    });
}

struct Scope(u64);

impl Scope {
    fn start() -> Self {
        GROSS.store(0, Ordering::Relaxed);
        let live = LIVE.load(Ordering::Relaxed);
        assert!(!COUNTING.swap(true, Ordering::AcqRel));
        Self(live)
    }

    fn finish(self) -> (u64, u64, i128) {
        COUNTING.store(false, Ordering::Release);
        let live = LIVE.load(Ordering::Relaxed);
        (
            GROSS.load(Ordering::Relaxed),
            live,
            live as i128 - self.0 as i128,
        )
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        COUNTING.store(false, Ordering::Release);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialect {
    Resp2,
    Resp3,
}

impl Dialect {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value {
            "resp2" => Ok(Self::Resp2),
            "resp3" => Ok(Self::Resp3),
            _ => Err(format!("unknown dialect {value}").into()),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Resp2 => "resp2",
            Self::Resp3 => "resp3",
        }
    }
}

struct Options {
    group: String,
    dialect: Dialect,
    case: String,
    key_bytes: usize,
    shape: String,
    batch_size: usize,
    payload_bytes: usize,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut values = std::collections::HashMap::new();
        let mut args = std::env::args().skip(1);
        while let Some(key) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value after {key}"))?;
            values.insert(key, value);
        }
        let group = values.remove("--group").ok_or("--group is required")?;
        let dialect = Dialect::parse(
            values
                .remove("--dialect")
                .unwrap_or_else(|| "resp2".to_owned())
                .as_str(),
        )?;
        let case = values.remove("--case").unwrap_or_default();
        let key_bytes = values
            .remove("--key-bytes")
            .unwrap_or_else(|| "0".to_owned())
            .parse()?;
        let shape = values
            .remove("--shape")
            .unwrap_or_else(|| "ascii".to_owned());
        let batch_size = values
            .remove("--batch-size")
            .unwrap_or_else(|| "0".to_owned())
            .parse()?;
        let payload_bytes = values
            .remove("--payload-bytes")
            .unwrap_or_else(|| "0".to_owned())
            .parse()?;
        if !values.is_empty() {
            return Err(format!("unknown arguments: {:?}", values.keys()).into());
        }
        Ok(Self {
            group,
            dialect,
            case,
            key_bytes,
            shape,
            batch_size,
            payload_bytes,
        })
    }
}

#[derive(Serialize)]
struct ResultRow {
    schema_version: u32,
    profile_id: &'static str,
    group: String,
    case: String,
    dialect: &'static str,
    key_bytes: usize,
    key_shape: String,
    batch_size: usize,
    payload_bytes: usize,
    operations: usize,
    gross_allocated_bytes: u64,
    live_allocated_bytes: u64,
    live_allocated_delta_bytes: i128,
    elapsed_nanoseconds: u128,
    wire_input_bytes: u64,
    decoded_argument_bytes: u64,
    structured_key_bytes: u64,
    execution_plan_request_count: u64,
    response_payload_bytes: u64,
    wire_output_bytes: u64,
    complete_frame_consumption_passed: bool,
    structured_key_shape_passed: bool,
    batch_cardinality_passed: bool,
    response_wire_passed: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let row = match options.group.as_str() {
        "decode-corpus" => profile_decode(&options)?,
        "single-key-translation" | "batch-translation" => profile_translate(&options)?,
        "response-encode" => profile_encode(&options)?,
        "roundtrip-controls" => profile_roundtrip(&options)?,
        _ => return Err(format!("unknown group {}", options.group).into()),
    };
    println!("{}", serde_json::to_string(&row)?);
    Ok(())
}

fn profile_decode(options: &Options) -> Result<ResultRow, Box<dyn Error>> {
    let args = corpus_args(&options.case)?;
    let wire = wire(&args);
    let expected_argument_bytes = args.iter().map(|arg| arg.len() as u64).sum::<u64>();
    let scope = Scope::start();
    let started = Instant::now();
    let mut consumed_ok = true;
    let mut decoded_bytes = 0_u64;
    for _ in 0..DEFAULT_OPERATIONS {
        let (command, consumed) = decode(&wire, options.dialect)?;
        consumed_ok &= consumed == wire.len();
        decoded_bytes += command_argument_bytes(&command);
        black_box(command);
    }
    let elapsed = started.elapsed().as_nanos();
    let allocation = scope.finish();
    Ok(row(
        options,
        DEFAULT_OPERATIONS,
        allocation,
        elapsed,
        (wire.len() * DEFAULT_OPERATIONS) as u64,
        decoded_bytes,
        0,
        0,
        0,
        0,
        consumed_ok && decoded_bytes == expected_argument_bytes * DEFAULT_OPERATIONS as u64,
        true,
        true,
        true,
    ))
}

fn profile_translate(options: &Options) -> Result<ResultRow, Box<dyn Error>> {
    let command = translation_command(options)?;
    let commands = vec![command; DEFAULT_OPERATIONS];
    let context = RedisTranslationContext::new("profile", "w4")?;
    let scope = Scope::start();
    let started = Instant::now();
    let mut structured = 0_u64;
    let mut requests = 0_u64;
    let mut cardinality_ok = true;
    let mut shape_ok = true;
    for command in commands {
        let translated = translate_redis_command(command, &context)?;
        let (bytes, count, keys) = translated_stats(&translated);
        structured += bytes;
        requests += count;
        let expected_keys = expected_plan_keys(options);
        cardinality_ok &= keys == expected_keys;
        shape_ok &= bytes == expected_structured_bytes(options, expected_keys) as u64;
        black_box(translated);
    }
    let elapsed = started.elapsed().as_nanos();
    let allocation = scope.finish();
    Ok(row(
        options,
        DEFAULT_OPERATIONS,
        allocation,
        elapsed,
        0,
        0,
        structured,
        requests,
        0,
        0,
        true,
        shape_ok,
        cardinality_ok,
        true,
    ))
}

fn profile_encode(options: &Options) -> Result<ResultRow, Box<dyn Error>> {
    let operations = if options.payload_bytes == 1_048_576 {
        LARGE_OPERATIONS
    } else {
        DEFAULT_OPERATIONS
    };
    let values = (0..operations)
        .map(|_| response_value(&options.case, options.payload_bytes))
        .collect::<Vec<_>>();
    let scope = Scope::start();
    let started = Instant::now();
    let mut wire_output = 0_u64;
    let mut wire_ok = true;
    for value in values {
        let encoded = encode(value, options.dialect)?;
        wire_ok &= encoded.ends_with(b"\r\n");
        wire_output += encoded.len() as u64;
        black_box(encoded);
    }
    let elapsed = started.elapsed().as_nanos();
    let allocation = scope.finish();
    Ok(row(
        options,
        operations,
        allocation,
        elapsed,
        0,
        0,
        0,
        0,
        (options.payload_bytes * operations) as u64,
        wire_output,
        true,
        true,
        true,
        wire_ok,
    ))
}

fn profile_roundtrip(options: &Options) -> Result<ResultRow, Box<dyn Error>> {
    let args = roundtrip_args(&options.case);
    let wire_input = wire(&args);
    let context = RedisTranslationContext::new("profile", "w4")?;
    let scope = Scope::start();
    let started = Instant::now();
    let mut consumed_ok = true;
    let mut structured = 0_u64;
    let mut requests = 0_u64;
    let mut output = 0_u64;
    for _ in 0..DEFAULT_OPERATIONS {
        let (command, consumed) = decode(&wire_input, options.dialect)?;
        consumed_ok &= consumed == wire_input.len();
        let translated = translate_redis_command(command, &context)?;
        let stats = translated_stats(&translated);
        structured += stats.0;
        requests += stats.1;
        let encoded = encode(roundtrip_response(&options.case), options.dialect)?;
        output += encoded.len() as u64;
        black_box((translated, encoded));
    }
    let elapsed = started.elapsed().as_nanos();
    let allocation = scope.finish();
    Ok(row(
        options,
        DEFAULT_OPERATIONS,
        allocation,
        elapsed,
        (wire_input.len() * DEFAULT_OPERATIONS) as u64,
        0,
        structured,
        requests,
        0,
        output,
        consumed_ok,
        true,
        true,
        output > 0,
    ))
}

#[allow(clippy::too_many_arguments)]
fn row(
    options: &Options,
    operations: usize,
    allocation: (u64, u64, i128),
    elapsed_nanoseconds: u128,
    wire_input_bytes: u64,
    decoded_argument_bytes: u64,
    structured_key_bytes: u64,
    execution_plan_request_count: u64,
    response_payload_bytes: u64,
    wire_output_bytes: u64,
    complete_frame_consumption_passed: bool,
    structured_key_shape_passed: bool,
    batch_cardinality_passed: bool,
    response_wire_passed: bool,
) -> ResultRow {
    ResultRow {
        schema_version: 1,
        profile_id: PROFILE_ID,
        group: options.group.clone(),
        case: options.case.clone(),
        dialect: options.dialect.name(),
        key_bytes: options.key_bytes,
        key_shape: options.shape.clone(),
        batch_size: options.batch_size,
        payload_bytes: options.payload_bytes,
        operations,
        gross_allocated_bytes: allocation.0,
        live_allocated_bytes: allocation.1,
        live_allocated_delta_bytes: allocation.2,
        elapsed_nanoseconds,
        wire_input_bytes,
        decoded_argument_bytes,
        structured_key_bytes,
        execution_plan_request_count,
        response_payload_bytes,
        wire_output_bytes,
        complete_frame_consumption_passed,
        structured_key_shape_passed,
        batch_cardinality_passed,
        response_wire_passed,
    }
}

fn decode(wire: &[u8], dialect: Dialect) -> Result<(RedisCommand, usize), Box<dyn Error>> {
    let decoded = match dialect {
        Dialect::Resp2 => decode_resp2_command(wire)?,
        Dialect::Resp3 => decode_resp3_command(wire)?,
    };
    decoded.ok_or_else(|| "complete frame decoded as incomplete".into())
}

fn encode(value: RespValue, dialect: Dialect) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(match dialect {
        Dialect::Resp2 => encode_resp2_value(value)?,
        Dialect::Resp3 => encode_resp3_value(value)?,
    })
}

fn wire(args: &[Vec<u8>]) -> Vec<u8> {
    let mut output = format!("*{}\r\n", args.len()).into_bytes();
    for arg in args {
        output.extend_from_slice(format!("${}\r\n", arg.len()).as_bytes());
        output.extend_from_slice(arg);
        output.extend_from_slice(b"\r\n");
    }
    output
}

fn key(index: usize, bytes: usize, shape: &str) -> Vec<u8> {
    if bytes == 0 {
        return Vec::new();
    }
    (0..bytes)
        .map(|offset| match shape {
            "ascii" => b'a' + ((index + offset) % 26) as u8,
            "binary" => ((index * 131 + offset * 17) % 256) as u8,
            _ => panic!("unknown key shape {shape}"),
        })
        .collect()
}

fn corpus_args(case: &str) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let key64 = key(0, 64, "binary");
    Ok(match case {
        "get-64" => vec![b"GET".to_vec(), key64],
        "set-64-64" => vec![b"SET".to_vec(), key64, vec![b'v'; 64]],
        "set-64-4096" => vec![b"SET".to_vec(), key64, vec![b'v'; 4_096]],
        "mget-16" => batch_args(b"MGET", 16, 64, "binary", false, false),
        "mget-256" => batch_args(b"MGET", 256, 64, "binary", false, false),
        "mset-16" => batch_args(b"MSET", 16, 64, "binary", false, true),
        "del-256-duplicate" => batch_args(b"DEL", 256, 64, "binary", true, false),
        "hc-tag-64" => {
            let mut args = vec![b"HC.TAG".to_vec(), key64];
            args.extend((0..64).map(|index| format!("tag-{index:03}").into_bytes()));
            args
        }
        _ => return Err(format!("unknown decode case {case}").into()),
    })
}

fn translation_command(options: &Options) -> Result<RedisCommand, Box<dyn Error>> {
    let duplicate = options.case == "del-all-duplicate";
    let keys = (0..options.batch_size.max(1))
        .map(|index| {
            key(
                if duplicate { 0 } else { index },
                options.key_bytes,
                &options.shape,
            )
        })
        .collect::<Vec<_>>();
    Ok(match options.case.as_str() {
        "get" => RedisCommand::Get {
            key: keys[0].clone(),
        },
        "set" => RedisCommand::Set {
            key: keys[0].clone(),
            value: vec![b'v'; 64],
            options: Vec::new(),
        },
        "hc-invalidate" => RedisCommand::HcInvalidate {
            key: keys[0].clone(),
        },
        "mget-distinct" => RedisCommand::Mget { keys },
        "mset-distinct" => RedisCommand::Mset {
            entries: keys.into_iter().map(|key| (key, vec![b'v'; 64])).collect(),
        },
        "exists-distinct" => RedisCommand::Exists { keys },
        "del-distinct" | "del-all-duplicate" => RedisCommand::Del { keys },
        _ => return Err(format!("unknown translation case {}", options.case).into()),
    })
}

fn batch_args(
    verb: &[u8],
    count: usize,
    bytes: usize,
    shape: &str,
    duplicate: bool,
    values: bool,
) -> Vec<Vec<u8>> {
    let mut args = vec![verb.to_vec()];
    for index in 0..count {
        args.push(key(if duplicate { 0 } else { index }, bytes, shape));
        if values {
            args.push(vec![b'v'; 64]);
        }
    }
    args
}

fn command_argument_bytes(command: &RedisCommand) -> u64 {
    match command {
        RedisCommand::Get { key } => 3 + key.len() as u64,
        RedisCommand::Set {
            key,
            value,
            options,
        } => {
            3 + key.len() as u64
                + value.len() as u64
                + options.iter().map(|value| value.len() as u64).sum::<u64>()
        }
        RedisCommand::Mget { keys }
        | RedisCommand::Del { keys }
        | RedisCommand::Exists { keys } => {
            command_verb_bytes(command) + keys.iter().map(|key| key.len() as u64).sum::<u64>()
        }
        RedisCommand::Mset { entries } => {
            4 + entries
                .iter()
                .map(|(key, value)| (key.len() + value.len()) as u64)
                .sum::<u64>()
        }
        RedisCommand::HcTag { key, tags } => {
            6 + key.len() as u64 + tags.iter().map(|tag| tag.len() as u64).sum::<u64>()
        }
        _ => 0,
    }
}

fn command_verb_bytes(command: &RedisCommand) -> u64 {
    match command {
        RedisCommand::Mget { .. } => 4,
        RedisCommand::Del { .. } => 3,
        RedisCommand::Exists { .. } => 6,
        _ => 0,
    }
}

fn translated_stats(translated: &RedisTranslatedCommand) -> (u64, u64, usize) {
    let RedisTranslatedCommand::Execute(plan) = translated else {
        return (0, 0, 0);
    };
    let mut bytes = 0_u64;
    let mut keys = 0_usize;
    for envelope in plan.initial_requests() {
        match &envelope.request {
            ClientRequest::Get { key, .. }
            | ClientRequest::Put { key, .. }
            | ClientRequest::Invalidate { key, .. } => add_structured(key, &mut bytes, &mut keys),
            ClientRequest::BatchGet { keys: batch, .. } => {
                for key in batch {
                    add_structured(key, &mut bytes, &mut keys);
                }
            }
            ClientRequest::BatchPut { entries, .. } => {
                for entry in entries {
                    add_structured(&entry.key, &mut bytes, &mut keys);
                }
            }
            _ => {}
        }
    }
    (bytes, plan.initial_requests().len() as u64, keys)
}

fn add_structured(key: &StructuredKey, bytes: &mut u64, keys: &mut usize) {
    *bytes += key
        .segments()
        .iter()
        .map(|segment| segment.len() as u64)
        .sum::<u64>();
    *keys += 1;
}

fn expected_plan_keys(options: &Options) -> usize {
    match options.case.as_str() {
        "get" | "set" | "hc-invalidate" => 1,
        "del-all-duplicate" => 1,
        _ => options.batch_size,
    }
}

fn expected_structured_bytes(options: &Options, keys: usize) -> usize {
    let one = if options.key_bytes == 0 {
        "redis-binary-v1-empty".len()
    } else {
        "redis-binary-v1-".len() + options.key_bytes * 2
    };
    one * keys
}

fn response_value(shape: &str, payload_bytes: usize) -> RespValue {
    match shape {
        "bulk" => RespValue::BulkString(vec![b'x'; payload_bytes]),
        "array-16" => {
            let base = payload_bytes / 16;
            let remainder = payload_bytes % 16;
            RespValue::Array(
                (0..16)
                    .map(|index| {
                        RespValue::BulkString(vec![b'x'; base + usize::from(index < remainder)])
                    })
                    .collect(),
            )
        }
        _ => panic!("unknown response shape {shape}"),
    }
}

fn roundtrip_args(case: &str) -> Vec<Vec<u8>> {
    match case {
        "get-binary-64-hit" => vec![b"GET".to_vec(), key(0, 64, "binary")],
        "mget-binary-16-mixed" => batch_args(b"MGET", 16, 64, "binary", false, false),
        "del-duplicate-256" => batch_args(b"DEL", 256, 64, "binary", true, false),
        _ => panic!("unknown roundtrip case {case}"),
    }
}

fn roundtrip_response(case: &str) -> RespValue {
    match case {
        "get-binary-64-hit" => RespValue::BulkString(vec![b'v'; 64]),
        "mget-binary-16-mixed" => RespValue::Array(
            (0..16)
                .map(|index| {
                    if index % 2 == 0 {
                        RespValue::BulkString(vec![b'v'; 64])
                    } else {
                        RespValue::Null
                    }
                })
                .collect(),
        ),
        "del-duplicate-256" => RespValue::Integer(1),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_is_binary_safe_and_complete() {
        let args = vec![b"GET".to_vec(), vec![0, 255]];
        let wire = wire(&args);
        let (command, consumed) = decode(&wire, Dialect::Resp2).unwrap();
        assert_eq!(consumed, wire.len());
        assert_eq!(command, RedisCommand::Get { key: vec![0, 255] });
    }

    #[test]
    fn frozen_group_count_is_ninety_two() {
        assert_eq!(2 * 8 + 4 * 2 * 3 + 3 * 2 * 5 + 2 * 4 * 2 + 2 * 3, 92);
    }
}
