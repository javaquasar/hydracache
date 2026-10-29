fn main() -> Result<(), Box<dyn std::error::Error>> {
    xtask::imap_soak_worker_075::run(std::env::args().skip(1).collect())
}
