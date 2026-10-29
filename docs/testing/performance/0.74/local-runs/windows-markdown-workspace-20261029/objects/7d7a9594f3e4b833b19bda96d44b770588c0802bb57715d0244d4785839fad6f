//! Native argument echo only; never invokes Cargo or a model image.
fn main() {
    for argument in std::env::args().skip(1) {
        println!("{argument:?}");
    }
}
