use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use hydracache_loadgen::allocation::measure_allocations;
use hydracache_redis_compat::{
    decode_resp2_command, encode_resp2_value, translate_redis_command, RedisCommand,
    RedisTranslatedCommand, RedisTranslationContext, RespValue,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const PROFILE_ID: &str = "w4-resp-stage-profile-074-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    Get,
    Set,
}

impl Operation {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value.to_ascii_lowercase().as_str() {
            "get" => Ok(Self::Get),
            "set" => Ok(Self::Set),
            _ => Err(format!("unsupported operation {value}").into()),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Set => "set",
        }
    }
}

#[derive(Debug, Clone)]
struct Options {
    source_commit: String,
    operation: Operation,
    iterations: u64,
    payload_bytes: usize,
    seed: u64,
    output: Option<PathBuf>,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut values = BTreeMap::new();
        let mut arguments = std::env::args().skip(1);
        while let Some(name) = arguments.next() {
            if !name.starts_with("--") {
                return Err(format!("unsupported argument {name}").into());
            }
            let value = arguments
                .next()
                .ok_or_else(|| format!("missing value after {name}"))?;
            values.insert(name, value);
        }
        let options = Self {
            source_commit: values
                .remove("--source-commit")
                .ok_or("--source-commit is required")?,
            operation: Operation::parse(&take(&mut values, "--operation", "get"))?,
            iterations: take(&mut values, "--iterations", "1000000").parse()?,
            payload_bytes: take(&mut values, "--payload-bytes", "256").parse()?,
            seed: take(&mut values, "--seed", "74").parse()?,
            output: values.remove("--output").map(PathBuf::from),
        };
        if !values.is_empty() {
            return Err(format!("unknown arguments: {:?}", values.keys()).into());
        }
        if options.iterations == 0 || options.payload_bytes == 0 {
            return Err("iterations and payload-bytes must be non-zero".into());
        }
        if options.source_commit.len() != 40
            || !options
                .source_commit
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("source-commit must be a full 40-character Git SHA".into());
        }
        Ok(options)
    }
}

fn take(values: &mut BTreeMap<String, String>, name: &str, default: &str) -> String {
    values.remove(name).unwrap_or_else(|| default.to_owned())
}

#[derive(Debug, Serialize)]
struct StageMeasurement {
    gross_allocated_bytes: u64,
    gross_allocated_bytes_per_operation: f64,
    cpu_nanoseconds_per_operation: f64,
    wall_nanoseconds_per_operation: f64,
    checksum: u64,
}

#[derive(Debug, Serialize)]
struct Receipt {
    schema_version: u32,
    release: &'static str,
    profile_id: &'static str,
    source_commit: String,
    binary_sha256: String,
    build_profile: &'static str,
    tier: &'static str,
    promotable: bool,
    operation: &'static str,
    iterations: u64,
    payload_bytes: usize,
    seed: u64,
    workload_sha256: String,
    exact_result_validation: bool,
    decode: StageMeasurement,
    translation_context: StageMeasurement,
    command_construction: StageMeasurement,
    command_construction_and_translation: StageMeasurement,
    translation_incremental_allocated_bytes_per_operation: f64,
    response_construction: StageMeasurement,
    response_construction_and_encode: StageMeasurement,
    encode_incremental_allocated_bytes_per_operation: f64,
    limitations: Vec<&'static str>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let key = format!("hc074-stage:{:016x}", options.seed).into_bytes();
    let payload = payload(options.payload_bytes, options.seed);
    let request = request(options.operation, &key, &payload);
    let context = RedisTranslationContext::new("default", "redis-resp-stage")?;
    validate_fixture(options.operation, &request, &key, &payload, &context)?;

    let decode = measure_stage(options.iterations, || {
        let (command, consumed) =
            decode_resp2_command(&request)?.ok_or("complete stage request decoded as partial")?;
        black_box(command);
        Ok(consumed as u64)
    })
    .await?;
    let translation_context = measure_stage(options.iterations, || {
        let context = RedisTranslationContext::new("default", "redis-resp-stage")?;
        black_box(context);
        Ok(1)
    })
    .await?;
    let command_construction = measure_stage(options.iterations, || {
        black_box(command(options.operation, &key, &payload));
        Ok(1)
    })
    .await?;
    let command_construction_and_translation = measure_stage(options.iterations, || {
        let translated =
            translate_redis_command(command(options.operation, &key, &payload), &context)?;
        black_box(translated);
        Ok(1)
    })
    .await?;
    let response_construction = measure_stage(options.iterations, || {
        black_box(response(options.operation, &payload));
        Ok(1)
    })
    .await?;
    let response_construction_and_encode = measure_stage(options.iterations, || {
        let encoded = encode_resp2_value(response(options.operation, &payload))?;
        let length = encoded.len() as u64;
        black_box(encoded);
        Ok(length)
    })
    .await?;

    let receipt = Receipt {
        schema_version: 1,
        release: "0.74",
        profile_id: PROFILE_ID,
        source_commit: options.source_commit.clone(),
        binary_sha256: binary_digest()?,
        build_profile: "release",
        tier: "local-attribution",
        promotable: false,
        operation: options.operation.name(),
        iterations: options.iterations,
        payload_bytes: options.payload_bytes,
        seed: options.seed,
        workload_sha256: workload_digest(&options),
        exact_result_validation: true,
        translation_incremental_allocated_bytes_per_operation: command_construction_and_translation
            .gross_allocated_bytes_per_operation
            - command_construction.gross_allocated_bytes_per_operation,
        encode_incremental_allocated_bytes_per_operation: response_construction_and_encode
            .gross_allocated_bytes_per_operation
            - response_construction.gross_allocated_bytes_per_operation,
        decode,
        translation_context,
        command_construction,
        command_construction_and_translation,
        response_construction,
        response_construction_and_encode,
        limitations: vec![
            "stage_cpu_includes_counting_allocator_overhead",
            "incremental_allocation_is_subtraction_of_deterministic_stage_totals",
            "does_not_include_dispatch_store_or_socket_io",
        ],
    };
    let json = serde_json::to_string_pretty(&receipt)?;
    if let Some(output) = &options.output {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output, &json)?;
    }
    println!("{json}");
    Ok(())
}

async fn measure_stage<F>(
    iterations: u64,
    mut action: F,
) -> Result<StageMeasurement, Box<dyn Error>>
where
    F: FnMut() -> Result<u64, Box<dyn Error>>,
{
    let (allocation_checksum, allocation) = measure_allocations(iterations, async {
        let mut checksum = 0_u64;
        for _ in 0..iterations {
            checksum = checksum.wrapping_add(action()?);
        }
        Ok::<u64, Box<dyn Error>>(checksum)
    })
    .await;
    let allocation_checksum = allocation_checksum?;

    let cpu_before = process_cpu_seconds()?;
    let started = Instant::now();
    let mut timed_checksum = 0_u64;
    for _ in 0..iterations {
        timed_checksum = timed_checksum.wrapping_add(action()?);
    }
    let elapsed = started.elapsed();
    let cpu_seconds = (process_cpu_seconds()? - cpu_before).max(0.0);
    if allocation_checksum != timed_checksum {
        return Err("stage checksum changed between allocation and timing passes".into());
    }

    Ok(StageMeasurement {
        gross_allocated_bytes: allocation.gross_allocated_bytes,
        gross_allocated_bytes_per_operation: allocation.gross_allocated_bytes_per_operation,
        cpu_nanoseconds_per_operation: cpu_seconds * 1_000_000_000.0 / iterations as f64,
        wall_nanoseconds_per_operation: elapsed.as_secs_f64() * 1_000_000_000.0 / iterations as f64,
        checksum: timed_checksum,
    })
}

fn command(operation: Operation, key: &[u8], payload: &[u8]) -> RedisCommand {
    match operation {
        Operation::Get => RedisCommand::Get { key: key.to_vec() },
        Operation::Set => RedisCommand::Set {
            key: key.to_vec(),
            value: payload.to_vec(),
            options: Vec::new(),
        },
    }
}

fn response(operation: Operation, payload: &[u8]) -> RespValue {
    match operation {
        Operation::Get => RespValue::BulkString(payload.to_vec()),
        Operation::Set => RespValue::SimpleString("OK"),
    }
}

fn request(operation: Operation, key: &[u8], payload: &[u8]) -> Vec<u8> {
    let arguments = match operation {
        Operation::Get => vec![b"GET".as_slice(), key],
        Operation::Set => vec![b"SET".as_slice(), key, payload],
    };
    let mut output = format!("*{}\r\n", arguments.len()).into_bytes();
    for argument in arguments {
        output.extend_from_slice(format!("${}\r\n", argument.len()).as_bytes());
        output.extend_from_slice(argument);
        output.extend_from_slice(b"\r\n");
    }
    output
}

fn validate_fixture(
    operation: Operation,
    request: &[u8],
    key: &[u8],
    payload: &[u8],
    context: &RedisTranslationContext,
) -> Result<(), Box<dyn Error>> {
    let (decoded, consumed) =
        decode_resp2_command(request)?.ok_or("stage request is incomplete")?;
    if consumed != request.len() || decoded != command(operation, key, payload) {
        return Err("stage request does not decode to the expected command".into());
    }
    let translated = translate_redis_command(decoded, context)?;
    if !matches!(translated, RedisTranslatedCommand::Execute(ref plan) if plan.initial_requests().len() == 1)
    {
        return Err("stage command did not translate to one initial request".into());
    }
    let encoded = encode_resp2_value(response(operation, payload))?;
    let expected = match operation {
        Operation::Get => {
            let mut expected = format!("${}\r\n", payload.len()).into_bytes();
            expected.extend_from_slice(payload);
            expected.extend_from_slice(b"\r\n");
            expected
        }
        Operation::Set => b"+OK\r\n".to_vec(),
    };
    if encoded != expected {
        return Err("stage response did not encode to the expected bytes".into());
    }
    Ok(())
}

fn payload(length: usize, seed: u64) -> Vec<u8> {
    let marker = seed.to_le_bytes();
    (0..length)
        .map(|index| marker[index % marker.len()])
        .collect()
}

fn workload_digest(options: &Options) -> String {
    let input = format!(
        "{PROFILE_ID}|{}|{}|{}|{}",
        options.operation.name(),
        options.iterations,
        options.payload_bytes,
        options.seed
    );
    format!("sha256:{:x}", Sha256::digest(input.as_bytes()))
}

fn binary_digest() -> Result<String, Box<dyn Error>> {
    let executable = std::env::current_exe()?;
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(fs::read(executable)?)
    ))
}

#[cfg(windows)]
fn process_cpu_seconds() -> Result<f64, Box<dyn Error>> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: all FILETIME pointers refer to writable values for the current process.
    if unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    let ticks =
        |value: FILETIME| ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64;
    Ok((ticks(kernel) + ticks(user)) as f64 / 10_000_000.0)
}

#[cfg(unix)]
fn process_cpu_seconds() -> Result<f64, Box<dyn Error>> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: getrusage initializes the supplied structure on success.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: the successful call above initialized `usage`.
    let usage = unsafe { usage.assume_init() };
    let seconds = |time: libc::timeval| time.tv_sec as f64 + time.tv_usec as f64 / 1_000_000.0;
    Ok(seconds(usage.ru_utime) + seconds(usage.ru_stime))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workload_digest_changes_with_semantic_input() {
        let original = Options {
            source_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            operation: Operation::Get,
            iterations: 100,
            payload_bytes: 256,
            seed: 74,
            output: None,
        };
        let mut changed = original.clone();
        changed.seed += 1;
        assert_ne!(workload_digest(&original), workload_digest(&changed));
    }

    #[test]
    fn stage_requests_roundtrip_exactly() {
        let key = b"binary\0key";
        let value = b"binary\r\nvalue";
        for operation in [Operation::Get, Operation::Set] {
            let context = RedisTranslationContext::default();
            validate_fixture(
                operation,
                &request(operation, key, value),
                key,
                value,
                &context,
            )
            .unwrap();
        }
    }
}
