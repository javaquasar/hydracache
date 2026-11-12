//! Bounded fixture qualification over the actual local service. No live import or disk format.
use hydracache_cluster_testkit::canonical_key_075::{ReferenceCanonicalKey, ReferenceKeyBounds};
use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_service_075::{
    ServiceCommand, ServiceEntry, ServiceLimits, ServiceTtl, UnifiedIMapService,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAX_FILE_BYTES: usize = 1024 * 1024;
pub const MODE: &str = "api_only_cold_repopulation";
type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub max_entries: usize,
    pub max_total_bytes: usize,
    pub max_chunk_entries: usize,
    pub max_chunk_bytes: usize,
    pub max_key_bytes: usize,
    pub max_value_bytes: usize,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub key_hex: String,
    pub value_hex: String,
    pub ttl_remaining: Option<u64>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub id: String,
    pub codec_id: String,
    pub tenant: String,
    pub namespace: String,
    pub generation: u64,
    pub source_snapshot: String,
    pub entries: Vec<Entry>,
    pub expected_count: usize,
    pub expected_digest: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodecVector {
    pub id: String,
    pub text: String,
    pub encoded_hex: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub release: String,
    pub fixture_version: u64,
    pub status: String,
    pub mode: String,
    pub production_import: bool,
    pub limits: Limits,
    pub application_codec_vectors: Vec<CodecVector>,
    pub fixtures: Vec<Fixture>,
}
#[derive(Debug, Clone)]
pub struct Chunk {
    pub index: usize,
    pub codec_id: String,
    pub tenant: String,
    pub namespace: String,
    pub generation: u64,
    pub source_snapshot: String,
    pub entries: Vec<Entry>,
    pub digest: String,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ChunkReceipt {
    pub index: usize,
    pub duplicate: bool,
    pub entries: usize,
    pub next_index: usize,
}

pub fn load_corpus(source: &[u8]) -> Result<Corpus> {
    if source.len() > MAX_FILE_BYTES {
        return Err("migration fixture file exceeds bound".into());
    }
    let corpus: Corpus = serde_json::from_slice(source)?;
    if corpus.release != "0.75"
        || corpus.fixture_version != 1
        || corpus.status != "provisional"
        || corpus.mode != MODE
        || corpus.production_import
    {
        return Err(
            "migration fixture must remain provisional dry-run API-only/cold repopulation".into(),
        );
    }
    validate_limits(&corpus.limits)?;
    if corpus.fixtures.is_empty()
        || corpus.fixtures.len() > 32
        || corpus.application_codec_vectors.len() > 32
    {
        return Err("migration corpus count bound".into());
    }
    let mut ids = BTreeSet::new();
    for vector in &corpus.application_codec_vectors {
        if vector.id.is_empty()
            || !ids.insert(&vector.id)
            || vector.text.len() > corpus.limits.max_value_bytes
            || hex(&vector.encoded_hex, corpus.limits.max_value_bytes)? != vector.text.as_bytes()
        {
            return Err("application codec vector mismatch".into());
        }
    }
    ids.clear();
    for fixture in &corpus.fixtures {
        if !ids.insert(&fixture.id) {
            return Err("duplicate migration fixture".into());
        }
        validate_fixture(fixture, &corpus.limits)?;
    }
    Ok(corpus)
}
fn validate_limits(l: &Limits) -> Result<()> {
    if [
        l.max_entries,
        l.max_total_bytes,
        l.max_chunk_entries,
        l.max_chunk_bytes,
        l.max_key_bytes,
        l.max_value_bytes,
    ]
    .contains(&0)
        || l.max_entries > 256
        || l.max_chunk_entries > 64
        || l.max_chunk_entries > l.max_entries
        || l.max_total_bytes > MAX_FILE_BYTES
        || l.max_chunk_bytes > l.max_total_bytes
        || l.max_key_bytes > 4096
        || l.max_value_bytes > 65536
    {
        return Err("migration bounds exceed local fixture limits".into());
    }
    Ok(())
}
fn validate_fixture(f: &Fixture, l: &Limits) -> Result<()> {
    validate_limits(l)?;
    if f.id.is_empty()
        || f.id.len() > 64
        || f.source_snapshot.is_empty()
        || f.source_snapshot.len() > 64
        || f.codec_id != "binary_identity_v1"
        || f.generation == 0
        || f.generation > i64::MAX as u64
        || f.entries.len() != f.expected_count
        || f.expected_count > l.max_entries
    {
        return Err("invalid migration fixture identity/codec/count".into());
    }
    ReferenceCanonicalKey::new(&f.tenant, &f.namespace, f.generation, vec![])
        .encode(ReferenceKeyBounds::default())
        .map_err(|error| format!("fixture namespace: {error:?}"))?;
    let mut keys = BTreeSet::new();
    let mut total = 0;
    for entry in &f.entries {
        let key = hex(&entry.key_hex, l.max_key_bytes)?;
        if !keys.insert(key) {
            return Err("duplicate migration key".into());
        }
        total += owner_bytes(f, entry, l)?;
    }
    if total > l.max_total_bytes || digest(f, &f.entries, None)? != f.expected_digest {
        return Err("migration source digest or budget mismatch".into());
    }
    Ok(())
}
pub fn chunk(fixture: &Fixture, index: usize, entries: Vec<Entry>) -> Result<Chunk> {
    if entries.len() > 64 {
        return Err("migration chunk hard count bound".into());
    }
    Ok(Chunk {
        index,
        codec_id: fixture.codec_id.clone(),
        tenant: fixture.tenant.clone(),
        namespace: fixture.namespace.clone(),
        generation: fixture.generation,
        source_snapshot: fixture.source_snapshot.clone(),
        digest: digest(fixture, &entries, Some(index))?,
        entries,
    })
}

/// A separate actual UnifiedIMapService is staging only. The passed live owner is never mutated.
pub struct DryRun {
    fixture: Fixture,
    limits: Limits,
    live: UnifiedIMapService,
    before: Vec<ServiceEntry>,
    staging: UnifiedIMapService,
    completed: BTreeMap<usize, String>,
    accepted: usize,
    closed: bool,
}
impl DryRun {
    pub fn new(fixture: Fixture, limits: Limits, live: UnifiedIMapService) -> Result<Self> {
        validate_fixture(&fixture, &limits)?;
        let bounds = CollectionBounds {
            max_key_bytes: 8192,
            max_value_bytes: limits.max_value_bytes,
            max_entries: limits.max_entries + 1,
            max_page_items: 256,
            max_scan_items: 256,
            max_response_bytes: MAX_FILE_BYTES,
            max_bulk_mutation_items: limits.max_entries + 1,
        };
        let service_limits = ServiceLimits {
            value_bytes: limits.max_total_bytes,
            ..ServiceLimits::default()
        };
        let staging = UnifiedIMapService::with_limits(bounds, fixture.generation, service_limits)?;
        Ok(Self {
            fixture,
            limits,
            before: live.diagnostic_snapshot(0)?,
            live,
            staging,
            completed: BTreeMap::new(),
            accepted: 0,
            closed: false,
        })
    }
    pub fn accept(&mut self, chunk: &Chunk) -> Result<ChunkReceipt> {
        if self.closed {
            return Err("migration dry-run is closed".into());
        }
        if chunk.codec_id != self.fixture.codec_id
            || chunk.tenant != self.fixture.tenant
            || chunk.namespace != self.fixture.namespace
            || chunk.generation != self.fixture.generation
            || chunk.source_snapshot != self.fixture.source_snapshot
        {
            return Err("migration chunk scope/codec/source mismatch".into());
        }
        if chunk.entries.len() > self.limits.max_chunk_entries
            || (chunk.entries.is_empty() && self.fixture.expected_count != 0)
            || digest(&self.fixture, &chunk.entries, Some(chunk.index))? != chunk.digest
        {
            return Err("migration corrupt or oversized chunk".into());
        }
        if let Some(previous) = self.completed.get(&chunk.index) {
            if previous != &chunk.digest {
                return Err("migration chunk identity conflict".into());
            }
            return Ok(ChunkReceipt {
                index: chunk.index,
                duplicate: true,
                entries: chunk.entries.len(),
                next_index: self.completed.len(),
            });
        }
        if chunk.index != self.completed.len() || self.completed.len() > self.limits.max_entries {
            return Err("migration chunk out of order".into());
        }
        let existing = self.staging.diagnostic_snapshot(0)?;
        let mut keys = existing
            .iter()
            .map(|entry| entry.key.clone())
            .collect::<BTreeSet<_>>();
        let mut bytes = 0;
        for entry in &chunk.entries {
            let physical = physical(&self.fixture, entry, &self.limits)?;
            if !keys.insert(physical) {
                return Err("duplicate migration key".into());
            }
            bytes += owner_bytes(&self.fixture, entry, &self.limits)?;
        }
        if bytes > self.limits.max_chunk_bytes
            || self.staging.resource_usage().values + bytes > self.limits.max_total_bytes
            || self.accepted + chunk.entries.len() > self.fixture.expected_count
        {
            return Err("migration chunk budget/count refusal".into());
        }
        for entry in &chunk.entries {
            let mut command =
                ServiceCommand::point("set", &physical(&self.fixture, entry, &self.limits)?);
            command.value = Some(hex(&entry.value_hex, self.limits.max_value_bytes)?);
            command.identity = Some(MutationIdentity::new(
                "migration-dry-run",
                (self.accepted + 1) as u64,
            ));
            command.ttl = entry
                .ttl_remaining
                .map_or(ServiceTtl::Eternal, ServiceTtl::ExpireAfter);
            self.staging.execute(command)?;
            self.accepted += 1;
        }
        self.completed.insert(chunk.index, chunk.digest.clone());
        Ok(ChunkReceipt {
            index: chunk.index,
            duplicate: false,
            entries: chunk.entries.len(),
            next_index: self.completed.len(),
        })
    }
    pub fn finish(&self) -> Result<Value> {
        if self.closed || self.accepted != self.fixture.expected_count {
            return Err("migration dry-run is closed or incomplete".into());
        }
        let actual = self.staging.diagnostic_snapshot(0)?;
        let mut entries = Vec::new();
        for entry in actual {
            let key = ReferenceCanonicalKey::decode(&entry.key, ReferenceKeyBounds::default())
                .map_err(|error| format!("canonical key decode: {error:?}"))?;
            if key.tenant != self.fixture.tenant
                || key.namespace != self.fixture.namespace
                || key.namespace_generation != self.fixture.generation
            {
                return Err("staging namespace mismatch".into());
            }
            entries.push(Entry {
                key_hex: encode_hex(&key.key),
                value_hex: encode_hex(&entry.value),
                ttl_remaining: entry.remaining_ttl,
            });
        }
        let actual_digest = digest(&self.fixture, &entries, None)?;
        if entries.len() != self.fixture.expected_count
            || actual_digest != self.fixture.expected_digest
        {
            return Err("actual staging cardinality/digest mismatch".into());
        }
        Ok(
            json!({"fixture":self.fixture.id,"mode":MODE,"dry_run":true,"production_import":false,"entries":entries.len(),"chunks":self.completed.len(),"owned_bytes":self.staging.resource_usage().values,"source_digest":self.fixture.expected_digest,"staging_digest":actual_digest,"live_unchanged":self.before == self.live.diagnostic_snapshot(0)?}),
        )
    }
    pub fn staging(&self) -> &UnifiedIMapService {
        &self.staging
    }
    pub fn close(&mut self) -> Result<()> {
        if !self.closed {
            let mut command = ServiceCommand::point("destroy", &[]);
            command.identity = Some(MutationIdentity::new("migration-close", 1));
            self.staging.execute(command)?;
            self.completed.clear();
            self.closed = true;
        }
        Ok(())
    }
}

fn physical(f: &Fixture, entry: &Entry, l: &Limits) -> Result<Vec<u8>> {
    ReferenceCanonicalKey::new(
        &f.tenant,
        &f.namespace,
        f.generation,
        hex(&entry.key_hex, l.max_key_bytes)?,
    )
    .encode(ReferenceKeyBounds::default())
    .map_err(|error| format!("canonical key: {error:?}").into())
}
fn owner_bytes(f: &Fixture, entry: &Entry, l: &Limits) -> Result<usize> {
    if entry
        .ttl_remaining
        .is_some_and(|ttl| ttl == 0 || ttl > i64::MAX as u64)
    {
        return Err("invalid migration remaining TTL".into());
    }
    Ok(24 + physical(f, entry, l)?.len() + hex(&entry.value_hex, l.max_value_bytes)?.len())
}
pub fn digest(f: &Fixture, entries: &[Entry], index: Option<usize>) -> Result<String> {
    if entries.len() > 256
        || [&f.codec_id, &f.tenant, &f.namespace, &f.source_snapshot]
            .iter()
            .any(|text| text.len() > 256)
    {
        return Err("migration digest scope/count bound".into());
    }
    let mut hash = Sha256::new();
    hash.update(b"HC075-MIGRATION-DRY-RUN\0");
    for text in [&f.codec_id, &f.tenant, &f.namespace, &f.source_snapshot] {
        hash.update((text.len() as u32).to_be_bytes());
        hash.update(text.as_bytes());
    }
    hash.update(f.generation.to_be_bytes());
    if let Some(index) = index {
        hash.update([1]);
        hash.update((index as u64).to_be_bytes());
    } else {
        hash.update([0]);
    }
    let mut ordered = entries.to_vec();
    if index.is_none() {
        ordered.sort_by(|a, b| a.key_hex.cmp(&b.key_hex));
    }
    hash.update((ordered.len() as u32).to_be_bytes());
    for entry in ordered {
        for text in [&entry.key_hex, &entry.value_hex] {
            let bytes = hex(text, MAX_FILE_BYTES)?;
            hash.update((bytes.len() as u32).to_be_bytes());
            hash.update(bytes);
        }
        hash.update(entry.ttl_remaining.unwrap_or(0).to_be_bytes());
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
fn hex(text: &str, maximum: usize) -> Result<Vec<u8>> {
    if !text.len().is_multiple_of(2)
        || text.len() / 2 > maximum
        || !text
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err("noncanonical or oversized migration hex".into());
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair)?, 16).map_err(Into::into))
        .collect()
}
fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
pub fn check_at_root(root: &Path, release: &str) -> Result<Value> {
    if release != "0.75" {
        return Err("unsupported migration dry-run release".into());
    }
    check_file(&root.join("docs/testing/imap/0.75/migration-fixtures.json"))
}
pub fn check_file(path: &Path) -> Result<Value> {
    let mut source = Vec::new();
    File::open(path)?
        .take((MAX_FILE_BYTES + 1) as u64)
        .read_to_end(&mut source)?;
    let corpus = load_corpus(&source)?;
    let mut receipts = Vec::new();
    for fixture in corpus.fixtures {
        let live = UnifiedIMapService::new(CollectionBounds::default(), fixture.generation)?;
        let mut dry_run = DryRun::new(fixture.clone(), corpus.limits.clone(), live)?;
        if fixture.entries.is_empty() {
            dry_run.accept(&chunk(&fixture, 0, vec![])?)?;
        } else {
            for (index, entries) in fixture
                .entries
                .chunks(corpus.limits.max_chunk_entries)
                .enumerate()
            {
                dry_run.accept(&chunk(&fixture, index, entries.to_vec())?)?;
            }
        }
        receipts.push(dry_run.finish()?);
        dry_run.close()?;
    }
    Ok(
        json!({"release":"0.75","status":"provisional","mode":MODE,"dry_run":true,"production_import":false,"fixtures":receipts}),
    )
}
pub fn run(args: Vec<String>) -> Result<()> {
    let mut release = None;
    let mut fixture = None;
    let mut output = None;
    let mut seen = BTreeSet::new();
    for pair in args.chunks(2) {
        if pair.len() != 2 || !seen.insert(&pair[0]) {
            return Err("migration dry-run arguments require unique name/value pairs".into());
        }
        match pair[0].as_str() {
            "--release" => release = Some(pair[1].clone()),
            "--fixtures" => fixture = Some(PathBuf::from(&pair[1])),
            "--output" => output = Some(PathBuf::from(&pair[1])),
            _ => return Err("unknown migration dry-run argument".into()),
        }
    }
    if release.as_deref() != Some("0.75") {
        return Err("migration dry-run requires --release 0.75".into());
    }
    let result = match fixture {
        Some(path) => check_file(&path)?,
        None => check_at_root(&crate::doc_check::find_repo_root()?, "0.75")?,
    };
    let encoded = serde_json::to_vec_pretty(&result)?;
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?
            .write_all(&encoded)?;
    } else {
        println!("{}", String::from_utf8(encoded)?);
    }
    Ok(())
}
