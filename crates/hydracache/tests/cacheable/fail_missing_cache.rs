use hydracache::cacheable;

fn main() {
    let _future = cacheable!(
        key = "value:1",
        load = || async { Ok::<_, std::io::Error>(1_u64) },
    );
}
