use hydracache_long_run_supervisor_074::diagnostic_signer_isolation::policy_bytes;

const ALLOW: u32 = 0x7fff0000;
const DENY: u32 = 0x00050001;
const KILL: u32 = 0x80000000;
const ARCH: u32 = 0xc000003e;
const BLOCKED: [u32; 23] = [
    59, 322, 57, 58, 56, 435, 101, 317, 272, 308, 161, 80, 81, 165, 166, 155, 428, 429, 430, 431,
    432, 433, 442,
];

// Independent bounded classic-BPF interpreter, never installs a caller filter.
fn evaluate(bytes: &[u8], nr: u32, arch: u32, args: [u64; 6]) -> Option<u32> {
    if bytes.len() != 512 {
        return None;
    }
    let mut data = [0u8; 64];
    data[..4].copy_from_slice(&nr.to_le_bytes());
    data[4..8].copy_from_slice(&arch.to_le_bytes());
    for (i, arg) in args.iter().enumerate() {
        data[16 + i * 8..24 + i * 8].copy_from_slice(&arg.to_le_bytes());
    }
    let mut pc = 0usize;
    let mut a = 0;
    for _ in 0..64 {
        let row = bytes.get(pc * 8..pc * 8 + 8)?;
        let code = u16::from_le_bytes(row[..2].try_into().ok()?);
        let k = u32::from_le_bytes(row[4..].try_into().ok()?);
        match code {
            0x20 => {
                if row[2] != 0 || row[3] != 0 {
                    return None;
                }
                a = u32::from_le_bytes(data.get(k as usize..k as usize + 4)?.try_into().ok()?);
                pc += 1;
            }
            0x15 | 0x45 => {
                let yes = if code == 0x15 { a == k } else { a & k != 0 };
                pc += 1 + if yes { row[2] } else { row[3] } as usize;
            }
            0x06 => {
                return if row[2] == 0 && row[3] == 0 {
                    Some(k)
                } else {
                    None
                }
            }
            _ => return None,
        }
    }
    None
}
fn expected(nr: u32, arch: u32, args: [u64; 6]) -> u32 {
    if arch != ARCH || nr & 0x40000000 != 0 {
        KILL
    } else if BLOCKED.contains(&nr) || (nr == 157 && (args[0] != 4 || args[1] != 0)) {
        DENY
    } else {
        ALLOW
    }
}

#[test]
fn ordinary_policy_has_exact_fixed_shape_and_closed_denial_set() {
    let bytes = policy_bytes();
    assert_eq!(bytes.len(), 512);
    assert_eq!(&bytes[..8], &[0x20, 0, 0, 0, 4, 0, 0, 0]);
    for (i, nr) in BLOCKED.iter().enumerate() {
        assert_eq!(
            &bytes[(6 + i * 2) * 8..(7 + i * 2) * 8],
            &[0x15, 0, 0, 1, *nr as u8, (*nr >> 8) as u8, 0, 0]
        );
        assert_eq!(
            &bytes[(7 + i * 2) * 8..(8 + i * 2) * 8],
            &[6, 0, 0, 0, 1, 0, 5, 0]
        );
    }
}
#[test]
fn ordinary_policy_all_native_numbers_and_argument_classes() {
    for nr in 0..1024 {
        for args in [
            [0; 6],
            [u64::MAX; 6],
            [4, 0, 0, 0, 0, 0],
            [4, 1, 0, 0, 0, 0],
            [1u64 << 32, 0, 0, 0, 0, 0],
            [4, 1u64 << 32, 0, 0, 0, 0],
        ] {
            assert_eq!(
                evaluate(&policy_bytes(), nr, ARCH, args),
                Some(expected(nr, ARCH, args)),
                "nr={nr}"
            );
        }
    }
}
#[test]
fn ordinary_policy_architecture_and_x32_are_terminal() {
    for arch in [0, 0x40000003, 0xc00000b7, u32::MAX, ARCH] {
        for nr in [0, 20, 39, 157, 0x40000000, 0x40000027, u32::MAX] {
            assert_eq!(
                evaluate(&policy_bytes(), nr, arch, [4, 0, 0, 0, 0, 0]),
                Some(expected(nr, arch, [4, 0, 0, 0, 0, 0]))
            );
        }
    }
}
#[test]
fn ordinary_policy_prctl_tail_checks_both_full_width_arguments() {
    for args in [
        [4, 0, 1, 2, 3, 4],
        [4, 0, u64::MAX, u64::MAX, 0, 0],
        [4, 1, 0, 0, 0, 0],
        [4 | (1 << 32), 0, 0, 0, 0, 0],
        [4, 1 << 32, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0],
    ] {
        assert_eq!(
            evaluate(&policy_bytes(), 157, ARCH, args),
            Some(expected(157, ARCH, args))
        );
    }
}
#[test]
fn ordinary_policy_256_seeded_argument_mutations() {
    let mut seed = 0x0740c079u64;
    for i in 0..256 {
        let mut args = [0; 6];
        for arg in &mut args {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            *arg = seed;
        }
        if i % 4 == 0 {
            args[0] = 4;
            args[1] = 0;
        }
        let nr = if i % 2 == 0 { 157 } else { seed as u32 };
        assert_eq!(
            evaluate(&policy_bytes(), nr, ARCH, args),
            Some(expected(nr, ARCH, args))
        );
    }
}
#[test]
fn ordinary_policy_four_independent_readers_and_malformed_model_inputs() {
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                for _ in 0..64 {
                    assert_eq!(evaluate(&policy_bytes(), 39, ARCH, [0; 6]), Some(ALLOW));
                }
                policy_bytes()
            })
        })
        .collect();
    for thread in threads {
        assert_eq!(thread.join().unwrap(), policy_bytes());
    }
    assert!(evaluate(&[0; 511], 0, ARCH, [0; 6]).is_none());
    let mut malformed = policy_bytes();
    malformed[0] = 0xff;
    assert!(evaluate(&malformed, 0, ARCH, [0; 6]).is_none());
    let mut malformed = policy_bytes();
    malformed[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(evaluate(&malformed, 0, ARCH, [0; 6]).is_none());
}
#[test]
fn ordinary_probe_cli_refuses_unknown_command_before_process_mutation() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_diagnostic_signer_isolation_probe"))
        .arg("--exec")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert_eq!(out.stderr, b"Fixed signer isolation probe refused.\n");
}
