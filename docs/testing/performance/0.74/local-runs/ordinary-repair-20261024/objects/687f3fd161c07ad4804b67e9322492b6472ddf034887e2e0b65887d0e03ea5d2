//! Test-only consuming entry. No replacement validation, send or extraction.
use super::*;

impl JoinedHeldPlacement<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn fixture_materialize(self) {
        let origins = self.fixture_origins();
        println!("HC074-HELD-MATERIAL stage=projection-enter");
        let projected = self.project_held_unit().unwrap();
        assert_eq!(projected.owner.fixture_origins(), origins);
        assert!(projected.owner.fixture_timer_live());
        println!("HC074-HELD-MATERIAL stage=projection-return");
        let mut material = projected.materialize_held_unit().unwrap();
        assert_eq!(material.owner.owner.fixture_origins(), origins);
        assert!(material.owner.owner.fixture_timer_live());
        assert_eq!(material.inventory.as_ref().unwrap().descriptors.len(), 20);
        println!("HC074-HELD-MATERIAL stage=material-return descriptors=20 original_bounds=true");
        material.revalidate().unwrap();
        assert_eq!(material.owner.owner.fixture_origins(), origins);
        assert!(material.owner.owner.fixture_timer_live());
        println!("HC074-HELD-MATERIAL stage=revalidate-return original_bounds=true");
        drop(material);
        println!("HC074-HELD-MATERIAL stage=drop-return original_send=false completion=false");
    }
}
