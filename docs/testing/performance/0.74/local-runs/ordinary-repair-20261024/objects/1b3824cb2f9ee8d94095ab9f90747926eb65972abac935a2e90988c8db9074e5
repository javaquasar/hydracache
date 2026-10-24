use super::*;
use std::io::{Seek, SeekFrom};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;

fn p16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn p32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn p64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn sample() -> Vec<u8> {
    let mut bytes = vec![0; 2048];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    p16(&mut bytes, 16, 3);
    p16(&mut bytes, 18, 62);
    p32(&mut bytes, 20, 1);
    p64(&mut bytes, 24, 0x10100);
    p64(&mut bytes, 32, 64);
    p16(&mut bytes, 52, 64);
    p16(&mut bytes, 54, 56);
    p16(&mut bytes, 56, 4);
    for (index, (kind, flags, offset, size, align)) in [
        (1u32, 5u32, 0u64, 2048u64, 4096u64),
        (3, 4, 384, elf::LOADER.len() as u64 + 1, 1),
        (2, 4, 512, 64, 8),
        (0x6474_e551, 6, 0, 0, 16),
    ]
    .into_iter()
    .enumerate()
    {
        let p = 64 + index * 56;
        p32(&mut bytes, p, kind);
        p32(&mut bytes, p + 4, flags);
        p64(&mut bytes, p + 8, offset);
        p64(
            &mut bytes,
            p + 16,
            if size == 0 { 0 } else { 0x10000 + offset },
        );
        p64(&mut bytes, p + 32, size);
        p64(&mut bytes, p + 40, size);
        p64(&mut bytes, p + 48, align);
    }
    bytes[384..384 + elf::LOADER.len()].copy_from_slice(elf::LOADER.as_bytes());
    for (index, (tag, value)) in [(1u64, 1u64), (5, 0x10300), (10, 11), (0, 0)]
        .into_iter()
        .enumerate()
    {
        p64(&mut bytes, 512 + index * 16, tag);
        p64(&mut bytes, 520 + index * 16, value);
    }
    bytes[768..779].copy_from_slice(b"\0libc.so.6\0");
    bytes
}
fn replace_file(fixture: &Fixture, name: &str, bytes: &[u8]) {
    fixture.writable();
    fs::remove_file(fixture.root.join(name)).unwrap();
    fs::write(fixture.root.join(name), bytes).unwrap();
    fs::set_permissions(fixture.root.join(name), fs::Permissions::from_mode(0o555)).unwrap();
    fixture.freeze();
}

#[test]
fn signer_runtime_synthetic_elf_translates_virtual_string_table() {
    let bytes = sample();
    let info = elf::parse(&bytes, true).unwrap();
    assert_eq!(info.interpreter.as_deref(), Some(elf::LOADER));
    assert_eq!(info.needed, ["libc.so.6"]);
    assert_eq!(info.program_headers, 4);
    assert_eq!(info.kind, 3);
    assert!(info.soname.is_none());
    println!("ELF_string_table=virtual_address_mapped_to_file synthetic_image_not_executed=true");
}

#[test]
fn signer_runtime_elf_header_program_and_range_mutations_refuse() {
    let original = sample();
    let oversized = vec![0; elf::MAX_IMAGE + 1];
    assert!(elf::parse(&oversized, true).is_err());
    for cut in 0..64 {
        assert!(elf::parse(&original[..cut], true).is_err());
    }
    let bad_fields = [
        (16, 4u64, 2usize),
        (18, 3, 2),
        (20, 0, 4),
        (24, 0, 8),
        (32, u64::MAX, 8),
        (48, 1, 4),
        (52, 63, 2),
        (54, 55, 2),
        (56, 0, 2),
        (56, 33, 2),
        (56, 65535, 2),
        (64 + 4, 7, 4),
        (64 + 32, 2049, 8),
        (64 + 40, u64::MAX, 8),
        (64 + 48, 3, 8),
        (120 + 8, u64::MAX, 8),
        (176 + 32, 63, 8),
        (232 + 4, 7, 4),
    ];
    for (offset, value, width) in bad_fields {
        let mut bytes = original.clone();
        match width {
            2 => p16(&mut bytes, offset, value as u16),
            4 => p32(&mut bytes, offset, value as u32),
            _ => p64(&mut bytes, offset, value),
        }
        assert!(elf::parse(&bytes, true).is_err(), "offset={offset}");
    }
    for index in [0, 4, 5, 6, 7, 8, 15] {
        let mut bytes = original.clone();
        bytes[index] = 255;
        assert!(elf::parse(&bytes, true).is_err(), "ident={index}");
    }
    for tag in [5, 0xffff_ffff, 2] {
        let mut bytes = original.clone();
        p32(&mut bytes, 120, tag);
        assert!(elf::parse(&bytes, true).is_err());
    }
    let mut bytes = original.clone();
    p32(&mut bytes, 120, 1);
    assert!(elf::parse(&bytes, true).is_err()); // overlapping LOAD intervals
    let mut bytes = original.clone();
    bytes.resize(16384, 0);
    p64(&mut bytes, 96, 16384);
    p64(&mut bytes, 104, 16384);
    p64(&mut bytes, 208, 513 * 16);
    p64(&mut bytes, 216, 513 * 16);
    assert!(elf::parse(&bytes, true).is_err()); // independently bounded dynamic entries
}

#[test]
fn signer_runtime_dynamic_names_search_flags_and_unknown_tags_refuse() {
    for count in [8, 9] {
        let mut bytes = sample();
        bytes[512..768].fill(0);
        bytes[768..900].fill(0);
        for index in 0..count {
            p64(&mut bytes, 512 + index * 16, 1);
            p64(&mut bytes, 520 + index * 16, (1 + index * 3) as u64);
            bytes[769 + index * 3..771 + index * 3].copy_from_slice(&[b'a', b'0' + index as u8]);
        }
        p64(&mut bytes, 512 + count * 16, 5);
        p64(&mut bytes, 520 + count * 16, 0x10300);
        p64(&mut bytes, 528 + count * 16, 10);
        p64(&mut bytes, 536 + count * 16, (1 + count * 3) as u64);
        let size = ((count + 3) * 16) as u64;
        p64(&mut bytes, 208, size);
        p64(&mut bytes, 216, size);
        assert_eq!(elf::parse(&bytes, true).is_ok(), count == 8);
    }
    for length in [128, 129] {
        let mut bytes = sample();
        bytes[769..769 + length].fill(b'a');
        bytes[769 + length] = 0;
        p64(&mut bytes, 552, (length + 2) as u64);
        assert_eq!(elf::parse(&bytes, true).is_ok(), length == 128);
    }
    for (tag, value) in [
        (15u64, 1u64),
        (29, 1),
        (22, 0),
        (30, 1),
        (0x6fff_fffb, 0x80),
        (0x6fff_fefb, 1),
        (0x6fff_fefc, 1),
        (0x7fff_fffd, 1),
        (0x7fff_ffff, 1),
        (0xffff_ffff, 0),
        (5, 0x10300),
    ] {
        let mut bytes = sample();
        p64(&mut bytes, 512, tag);
        p64(&mut bytes, 520, value);
        assert!(elf::parse(&bytes, true).is_err(), "tag={tag:#x}");
    }
    for (offset, value) in [
        (520, 0u64),
        (520, 11),
        (536, 768),
        (536, u64::MAX),
        (552, 0),
        (552, 65537),
        (560, 1),
        (568, 1),
    ] {
        let mut bytes = sample();
        p64(&mut bytes, offset, value);
        assert!(elf::parse(&bytes, true).is_err(), "dynamic offset={offset}");
    }
    for text in [
        b"../libc".as_slice(),
        b"lib/c",
        b"$ORIGIN",
        b"lib:c",
        b"bad\n",
        b"\xff",
    ] {
        let mut bytes = sample();
        bytes[768..779].fill(0);
        bytes[769..769 + text.len()].copy_from_slice(text);
        assert!(elf::parse(&bytes, true).is_err());
    }
    let mut bytes = sample();
    bytes[384] = b'.';
    assert!(elf::parse(&bytes, true).is_err());
    let mut bytes = sample();
    bytes[778] = b'x';
    assert!(elf::parse(&bytes, true).is_err());
}

#[test]
fn signer_runtime_original_four_file_inventory_is_not_runtime_admission() {
    let fixture = Fixture::new().unwrap();
    let mut retained = Retained::new(&fixture).unwrap();
    retained.check().unwrap();
    assert_eq!(retained.inputs.len(), 4);
    assert!(retained.inputs[0]
        .info
        .needed
        .iter()
        .any(|name| name == "libgcc_s.so.1"));
    for input in &retained.inputs {
        let hash: String = input
            .hash
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        println!(
            "runtime_input={} sha256={hash} needed={:?} original_fixture_only=true",
            input.name, input.info.needed
        );
    }
    println!("original_runtime_files=4 declared_elf_dependencies=checked launch_root_enforced=false custody_or_handoff=false");
}

#[test]
fn signer_runtime_file_modes_types_links_and_root_refuse() {
    for case in 0..8 {
        let fixture = Fixture::new().unwrap();
        let path = fixture.root.join(NAMES[2]);
        fixture.writable();
        match case {
            0 => {}
            1 => fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap(),
            2 => {
                fs::remove_file(&path).unwrap();
                symlink(fixture.root.join(NAMES[0]), &path).unwrap();
            }
            3 => {
                fs::remove_file(&path).unwrap();
                fs::hard_link(fixture.root.join(NAMES[0]), &path).unwrap();
            }
            4 => {
                fs::remove_file(&path).unwrap();
                let name = CString::new(path.as_os_str().as_bytes()).unwrap();
                // SAFETY: create only an owned fixture FIFO; NONBLOCK opener must refuse it.
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o555) }, 0);
            }
            5 => {
                fs::remove_file(&path).unwrap();
                fs::create_dir(&path).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
            }
            _ => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
                let writer = OpenOptions::new().write(true).open(&path).unwrap();
                writer
                    .set_len(if case == 6 {
                        63
                    } else {
                        elf::MAX_IMAGE as u64 + 1
                    })
                    .unwrap();
                assert!(read_bounded(&writer.try_clone().unwrap()).is_err());
                // A readable descriptor tests the stream bound rather than EBADF.
                assert!(read_bounded(&File::open(&path).unwrap()).is_err());
                fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
            }
        }
        if case != 0 {
            fixture.freeze();
        }
        assert!(Retained::new(&fixture).is_err(), "case={case}");
        if case == 5 {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
}

#[test]
fn signer_runtime_equal_byte_named_and_root_replacement_is_sticky() {
    for root in [false, true] {
        let fixture = Fixture::new().unwrap();
        let mut retained = Retained::new(&fixture).unwrap();
        if root {
            fs::rename(&fixture.root, fixture.temp.path().join("root-old")).unwrap();
            fs::create_dir(&fixture.root).unwrap();
            fixture.freeze();
        } else {
            let bytes = fs::read(fixture.root.join(NAMES[2])).unwrap();
            replace_file(&fixture, NAMES[2], &bytes);
            // Test-only bracket reset isolates original-file identity checks.
            // The actual guard exposes no refresh/reset operation.
            retained.stamp = Stamp::of(retained.root.metadata().unwrap());
        }
        assert!(retained.check().is_err());
        assert!(retained.refused);
        assert!(retained.check().is_err());
    }
}

#[test]
fn signer_runtime_content_hash_detects_drift_and_cannot_be_repaired() {
    let fixture = Fixture::new().unwrap();
    let mut retained = Retained::new(&fixture).unwrap();
    let path = fixture.root.join(NAMES[2]);
    let original = fs::read(&path).unwrap();
    let mut changed = original.clone();
    *changed.last_mut().unwrap() ^= 1;
    assert_eq!(
        elf::parse(&changed, false).unwrap(),
        retained.inputs[2].info
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(&path, changed).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
    // Synthetic metadata reset demonstrates the independent retained digest.
    retained.inputs[2].stamp = Stamp::of(retained.inputs[2].file.metadata().unwrap());
    assert!(retained.check().is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(&path, original).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(retained.check().is_err());
}

#[test]
fn signer_runtime_unknown_missing_extra_graph_and_objects_refuse() {
    let fixture = Fixture::new().unwrap();
    let retained = Retained::new(&fixture).unwrap();
    let original: Vec<_> = retained
        .inputs
        .iter()
        .map(|input| input.info.clone())
        .collect();
    for case in 0..5 {
        let mut info = original.clone();
        match case {
            0 => info[0].needed.push("foreign.so".to_owned()),
            1 => info[2].soname = Some("foreign.so".to_owned()),
            2 => info[1].needed.push(NAMES[1].to_owned()),
            3 => info[0].needed.retain(|name| name != NAMES[3]),
            _ => info.pop().map(|_| ()).unwrap(),
        }
        assert!(graph(&info).is_err(), "graph case={case}");
    }
    drop(retained);
    for extra in [false, true] {
        let fixture = Fixture::new().unwrap();
        fixture.writable();
        if extra {
            fs::write(fixture.root.join("ld.so.preload"), b"fixture injection").unwrap();
        } else {
            fs::remove_file(fixture.root.join(NAMES[2])).unwrap();
        }
        fixture.freeze();
        assert!(Retained::new(&fixture).is_err());
    }
}

#[test]
fn signer_runtime_cloexec_positional_reads_and_healthy_drop_preserve_owner() {
    let fixture = Fixture::new().unwrap();
    let mut retained = Retained::new(&fixture).unwrap();
    for file in std::iter::once(&retained.root).chain(retained.inputs.iter().map(|i| &i.file)) {
        // SAFETY: fixed flag query on the retained live descriptor, no transfer.
        assert_ne!(
            unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
            0
        );
    }
    let mut duplicate = retained.inputs[0].file.try_clone().unwrap();
    duplicate.seek(SeekFrom::Start(13)).unwrap();
    retained.check().unwrap();
    assert_eq!(duplicate.stream_position().unwrap(), 13);
    drop(retained);
    assert!(duplicate.metadata().unwrap().is_file());
    let path = fixture.root.join(NAMES[2]);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let writer = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
    // SAFETY: local fixture identity queries only.
    let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
    assert!(readonly(&writer, false, uid, gid).is_err());
    let reader = File::open(&path).unwrap();
    assert!(readonly(&reader, false, uid.wrapping_add(1), gid).is_err());
    assert!(readonly(&reader, false, uid, gid.wrapping_add(1)).is_err());
    let root_link = fixture.temp.path().join("root-link");
    symlink(&fixture.root, &root_link).unwrap();
    assert!(root_file(&root_link).is_err());
}

#[test]
fn signer_runtime_seeded_parser_mutations_and_independent_retention() {
    let original = sample();
    let mut seed = 0x0740_3b28_u64;
    for _ in 0..128 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut bytes = original.clone();
        let index = seed as usize % bytes.len();
        bytes[index] ^= 1;
        assert!(std::panic::catch_unwind(|| elf::parse(&bytes, true)).is_ok());
    }
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let fixture = Fixture::new().unwrap();
                let mut retained = Retained::new(&fixture).unwrap();
                retained.check().unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    println!("seed=0x07403b28 parser_mutations=128 independent_runtime_fixtures=4 runtime_or_launch_authority=false");
}
