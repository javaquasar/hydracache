use hydracache::cacheable_loader;

fn main() {
    let _future = cacheable_loader!(
        key = "value:1",
        load = || async { Ok::<_, std::io::Error>(1_u64) },
    );
}
