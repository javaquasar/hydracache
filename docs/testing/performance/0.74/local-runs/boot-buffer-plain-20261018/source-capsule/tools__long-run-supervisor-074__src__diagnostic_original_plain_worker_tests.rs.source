//! Uncaptured retained worker timing only, before reservation.
use super::*;

#[test]
#[ignore = "manual nonroot plain original worker pairs; no reservation or execution"]
fn original_plain_worker_batch() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    with_original_at(false, None, |original, paths, _, _, _| {
        original.revalidate().unwrap();
        let request = original.original.original.request.clone();
        let worker = fs::read(&paths.worker).unwrap();
        let oidc = fs::read(&paths.oidc).unwrap();
        let started = std::time::Instant::now();
        for _ in 0..128 {
            original.original.original.worker.revalidate().unwrap();
        }
        let wall_ns = started.elapsed().as_nanos();
        original.revalidate().unwrap();
        assert_eq!(original.original.original.request, request);
        assert_eq!(fs::read(&paths.worker).unwrap(), worker);
        assert_eq!(fs::read(&paths.oidc).unwrap(), oidc);
        assert!(!original.is_refused());
        assert!(wall_ns > 0);
        println!("HC074-PLAIN-WORKER iterations=128 wall_ns={wall_ns} original=true unchanged=true capture=false reservation=false start=false");
    });
}
