//! Compiled outside the workspace by imap-pre074-tools against the development contract.
use hydracache_cluster_testkit::imap_backend_contract_075::{BackendBinding, BackendOperation, DevelopmentBackendPort, DispatchMode, ReferenceBackendPort};
use hydracache_cluster_testkit::provisional_backend_075::{NamespaceIdentity, PartitionRoute};
use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_service_075::{ServiceCommand, UnifiedIMapService};

fn observe() -> String {
    let service=UnifiedIMapService::new(CollectionBounds::default(),3).unwrap();
    let binding=BackendBinding{namespace:NamespaceIdentity{tenant:1,namespace:2,generation:3},route:PartitionRoute{partition:0,epoch:4,owner:5},client:"packaged-consumer".into()};
    let port=ReferenceBackendPort::new(binding.clone(),service.clone(),DispatchMode::LocalReference).unwrap();
    let mut command=ServiceCommand::point("put",&[0,255]); command.value=Some(vec![0,128,255]); command.identity=Some(MutationIdentity::new("packaged-consumer",1));
    let prepared=port.prepare(&binding,command).unwrap();
    assert_eq!(service.statistics().requests,0);
    let result=port.execute(prepared).unwrap();
    let view=service.execute(ServiceCommand::point("get",&[0,255])).unwrap();
    let value=view.value.unwrap().iter().map(|b|format!("{b:02x}")).collect::<String>();
    format!("{{\"schema\":\"imap075-external-rust-consumer-v1\",\"kind\":\"{}\",\"value\":\"{value}\",\"operation_count\":{}}}",result.kind,BackendOperation::ALL.len())
}
fn main() { println!("{}",observe()); }

#[cfg(test)]
mod tests {
    #[test]
    fn external_consumer_preserves_the_owned_binary_request() {
        assert_eq!(super::observe(),"{\"schema\":\"imap075-external-rust-consumer-v1\",\"kind\":\"inserted\",\"value\":\"0080ff\",\"operation_count\":40}");
    }
}
