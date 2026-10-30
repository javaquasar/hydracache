pub fn owned_config(value: owned::FailureDetectorConfig) -> owned::FailureDetectorConfig {
    value
}

pub fn owned_id(value: &owned::ChitchatId) -> &owned::ChitchatId {
    value
}

pub fn owned_transport(
    value: &owned::transport::ChannelTransport,
) -> &dyn owned::transport::Transport {
    value
}

#[cfg(test)]
mod tests {
    use std::any::{TypeId, type_name};

    #[test]
    fn identical_display_names_do_not_establish_nominal_identity() {
        assert_eq!(
            type_name::<legacy::FailureDetectorConfig>(),
            type_name::<owned::FailureDetectorConfig>()
        );
        assert_eq!(
            type_name::<legacy::ChitchatId>(),
            type_name::<owned::ChitchatId>()
        );
        assert_ne!(
            TypeId::of::<legacy::FailureDetectorConfig>(),
            TypeId::of::<owned::FailureDetectorConfig>()
        );
        assert_ne!(
            TypeId::of::<legacy::ChitchatId>(),
            TypeId::of::<owned::ChitchatId>()
        );
        assert_ne!(
            TypeId::of::<dyn legacy::transport::Transport>(),
            TypeId::of::<dyn owned::transport::Transport>()
        );
    }

    #[test]
    fn small_wire_surfaces_agree_across_the_two_real_packages() {
        let addr = ([127, 0, 0, 1], 18401).into();
        let old_id = legacy::ChitchatId::new("identity-ключ", 7, addr);
        let new_id = owned::ChitchatId::new("identity-ключ", 7, addr);
        let old_wire = legacy::Serializable::serialize_to_vec(&old_id);
        let new_wire = owned::Serializable::serialize_to_vec(&new_id);
        assert_eq!(old_wire, new_wire);
        let mut cursor = old_wire.as_slice();
        let decoded =
            <owned::ChitchatId as owned::Deserializable>::deserialize(&mut cursor).unwrap();
        assert_eq!(decoded, new_id);
        assert!(cursor.is_empty());
        assert_eq!(
            legacy::Serializable::serialize_to_vec(&legacy::ChitchatMessage::BadCluster),
            owned::Serializable::serialize_to_vec(&owned::ChitchatMessage::BadCluster)
        );
    }

    #[test]
    fn owned_nominal_surface_remains_usable() {
        let _ = super::owned_config(owned::FailureDetectorConfig::default());
        let value = owned::ChitchatId::new("owned", 1, ([127, 0, 0, 1], 18402).into());
        assert!(std::ptr::eq(super::owned_id(&value), &value));
        let transport = owned::transport::ChannelTransport::with_mtu(65507);
        let _: &dyn owned::transport::Transport = super::owned_transport(&transport);
    }
}
