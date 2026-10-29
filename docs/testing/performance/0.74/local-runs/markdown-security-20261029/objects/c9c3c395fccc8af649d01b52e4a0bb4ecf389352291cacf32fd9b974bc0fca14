use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn name_parse(c: &mut Criterion) {
    const WELL_KNOWN_NAME: &str = "a.very.loooooooooooooooooo-ooooooo_0000o0ng.Name.\
            That.Is.Valid.For.DBus.and_good.For.benchmarks.I-guess";
    const UNIQUE_NAME: &str = ":a.very.loooooooooooooooooo-ooooooo_0000o0ng.Name.\
            That.Is.Valid.For.DBus.and_good.For.benchmarks.I-guess";
    const INTERFACE_NAME: &str = "a.very.loooooooooooooooooo_ooooooo_0000o0ng.Name.\
            That.Is.Valid.For.DBus.and_good.For.benchmarks.I_guess";
    const MEMBER_NAME: &str = "a_very_loooooooooooooooooo_ooooooo_0000o0ng_Name_\
            That_Is_Valid_For_DBus_and_good_For_benchmarks_I_guess";

    let mut group = c.benchmark_group("parse_name");
    group.bench_function("well_known", |b| {
        b.iter(|| {
            zbus::names::WellKnownName::try_from(black_box(WELL_KNOWN_NAME)).unwrap();
        })
    });

    group.bench_function("unique", |b| {
        b.iter(|| {
            zbus::names::UniqueName::try_from(black_box(UNIQUE_NAME)).unwrap();
        })
    });

    group.bench_function("bus", |b| {
        b.iter(|| {
            // Use a well-known name since the parser first tries unique name.
            zbus::names::BusName::try_from(black_box(WELL_KNOWN_NAME)).unwrap();
        })
    });

    group.bench_function("interface", |b| {
        b.iter(|| {
            zbus::names::InterfaceName::try_from(black_box(INTERFACE_NAME)).unwrap();
        })
    });

    group.bench_function("error", |b| {
        b.iter(|| {
            // Error names follow the same rules are interface names.
            zbus::names::ErrorName::try_from(black_box(INTERFACE_NAME)).unwrap();
        })
    });

    group.bench_function("member", |b| {
        b.iter(|| {
            zbus::names::MemberName::try_from(black_box(MEMBER_NAME)).unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, name_parse);
criterion_main!(benches);
