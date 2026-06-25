use std::fs;
use std::path::PathBuf;

use hydracache_client_protocol::java_migration::{
    java_exception_mapping, JavaClientRuntimeConfig, JavaClientTopology, JavaCodecDescriptor,
    JavaCodecKind, JavaCodecRegistryContract, JavaExceptionKind, JavaLockOperation,
    JavaLockProtocolFamily, JavaMapCasExpectation, JavaMapOperation, JavaMapProtocolFamily,
    JavaMigrationContractError, SpringCacheMode, UnsupportedHazelcastApiManifest,
    JAVA_MIGRATION_CONTRACT_VERSION, SUPPORTED_SPRING_BOOT_GENERATIONS,
};
use hydracache_client_protocol::{ClientErrorCode, ClientErrorEnvelope};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

mod java_migration_contract {
    use super::*;

    #[test]
    fn protocol_errors_map_to_documented_java_exception_kinds() {
        let cases = [
            (
                ClientErrorCode::IncompatibleVersion,
                false,
                JavaExceptionKind::Protocol,
                "HydraCacheProtocolException",
            ),
            (
                ClientErrorCode::Unauthenticated,
                false,
                JavaExceptionKind::Authentication,
                "HydraCacheAuthenticationException",
            ),
            (
                ClientErrorCode::Unauthorized,
                false,
                JavaExceptionKind::Authorization,
                "HydraCacheAuthorizationException",
            ),
            (
                ClientErrorCode::TenantQuota,
                true,
                JavaExceptionKind::QuotaExceeded,
                "HydraCacheQuotaExceededException",
            ),
            (
                ClientErrorCode::RateLimited,
                true,
                JavaExceptionKind::RateLimited,
                "HydraCacheRateLimitedException",
            ),
            (
                ClientErrorCode::ResidencyDenied,
                false,
                JavaExceptionKind::ResidencyDenied,
                "HydraCacheResidencyDeniedException",
            ),
            (
                ClientErrorCode::TooLarge,
                false,
                JavaExceptionKind::PayloadTooLarge,
                "HydraCachePayloadTooLargeException",
            ),
            (
                ClientErrorCode::DeadlineExceeded,
                true,
                JavaExceptionKind::Timeout,
                "HydraCacheTimeoutException",
            ),
            (
                ClientErrorCode::Conflict,
                false,
                JavaExceptionKind::Conflict,
                "HydraCacheConflictException",
            ),
            (
                ClientErrorCode::BackendUnavailable,
                true,
                JavaExceptionKind::BackendUnavailable,
                "HydraCacheBackendUnavailableException",
            ),
            (
                ClientErrorCode::MalformedFrame,
                false,
                JavaExceptionKind::MalformedFrame,
                "HydraCacheMalformedFrameException",
            ),
        ];

        for (code, retryable, kind, class_name) in cases {
            let error =
                ClientErrorEnvelope::new(code, retryable, "redacted").with_retry_after_ms(25);
            let mapping = java_exception_mapping(&error);

            assert_eq!(mapping.kind, kind, "{code:?}");
            assert_eq!(mapping.class_name, class_name, "{code:?}");
            assert_eq!(mapping.retryable, retryable, "{code:?}");
            assert!(mapping.preserves_request_id, "{code:?}");
            assert!(mapping.preserves_retry_after, "{code:?}");
        }
    }

    #[test]
    fn codec_registry_contract_rejects_ambiguous_or_reflective_serializer() {
        let user_codec =
            JavaCodecDescriptor::explicit("user-profile-v1", "com.acme.UserProfile", 1).unwrap();
        let mut registry = JavaCodecRegistryContract::new();
        registry.register(user_codec.clone()).unwrap();

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get("user-profile-v1"), Some(&user_codec));

        let duplicate =
            JavaCodecDescriptor::explicit("user-profile-v1", "com.acme.OtherProfile", 1).unwrap();
        let duplicate_error = registry.register(duplicate).unwrap_err();
        assert_eq!(
            duplicate_error,
            JavaMigrationContractError::AmbiguousCodec {
                codec_id: "user-profile-v1".to_owned()
            }
        );

        let reflective = JavaCodecDescriptor::new(
            "profile-reflective",
            "com.acme.UserProfile",
            1,
            JavaCodecKind::ReflectiveFallback,
        )
        .unwrap();
        assert_eq!(
            JavaCodecRegistryContract::new()
                .register(reflective)
                .unwrap_err(),
            JavaMigrationContractError::UnsupportedSerializer("reflective-fallback")
        );

        let native = JavaCodecDescriptor::new(
            "profile-native",
            "com.acme.UserProfile",
            1,
            JavaCodecKind::JavaNativeSerialization,
        )
        .unwrap();
        assert_eq!(
            JavaCodecRegistryContract::new()
                .register(native)
                .unwrap_err(),
            JavaMigrationContractError::UnsupportedSerializer("java-native-serialization")
        );

        let bridge = JavaCodecDescriptor::new(
            "legacy-user-profile",
            "com.acme.LegacyUserProfile",
            1,
            JavaCodecKind::LegacySerializerBridge,
        )
        .unwrap();
        assert_eq!(
            JavaCodecRegistryContract::new()
                .register(bridge.clone())
                .unwrap_err(),
            JavaMigrationContractError::LegacySerializerBridgeDisabled(
                "legacy-user-profile".to_owned()
            )
        );

        let mut migration_registry =
            JavaCodecRegistryContract::new().with_legacy_serializer_bridge_enabled();
        migration_registry.register(bridge).unwrap();
        assert_eq!(migration_registry.len(), 1);
    }

    #[test]
    fn unsupported_hazelcast_api_surface_is_a_checked_in_manifest() {
        let manifest = UnsupportedHazelcastApiManifest::checked_in().unwrap();

        assert_eq!(manifest.version, JAVA_MIGRATION_CONTRACT_VERSION);
        assert!(manifest.entries.len() >= 8);
        for api in [
            "HazelcastInstance.getExecutorService",
            "HazelcastInstance.getSql",
            "IMap.executeOnKey",
            "ReplicatedMap",
            "ReliableTopic",
        ] {
            let entry = manifest.find(api).expect("manifest entry");
            assert!(
                !entry.migration_hint.trim().is_empty(),
                "{api} should have a migration hint"
            );
        }
        for api in ["IMap.lock", "IMap.tryLock", "FencedLock"] {
            assert!(manifest.find(api).is_none(), "{api} should not be refused");
            let mapping = manifest.find_supported(api).expect("supported mapping");
            assert!(mapping.migration_hint.contains("HydraFencedLock"));
            assert!(mapping.migration_hint.contains("fence"));
        }
        for api in ["IMap.replace", "IMap.remove(key,value)"] {
            assert!(manifest.find(api).is_none(), "{api} should not be refused");
            let mapping = manifest.find_supported(api).expect("supported mapping");
            assert!(
                mapping.migration_hint.contains("protocol-v2"),
                "{api} should document protocol-v2 CAS mapping"
            );
        }
        assert!(manifest
            .find_supported("HazelcastInstance.getCPSubsystem().getLock")
            .expect("lock-only CP mapping")
            .migration_hint
            .contains("Lock-only"));

        let future_error =
            UnsupportedHazelcastApiManifest::parse("version=3\nunsupported|IMap.lock|hint")
                .unwrap_err();
        assert_eq!(
            future_error,
            JavaMigrationContractError::UnsupportedManifestVersion {
                actual: 3,
                supported: JAVA_MIGRATION_CONTRACT_VERSION
            }
        );

        let root = repo_root();
        let manifest_file = fs::read_to_string(
            root.join("crates/hydracache-client-protocol/manifests/unsupported_hazelcast_apis.txt"),
        )
        .expect("unsupported Hazelcast manifest");
        let docs = fs::read_to_string(root.join("docs/integrations/java-migration.md"))
            .expect("java migration docs");
        let compat = fs::read_to_string(root.join("docs/COMPAT.md")).expect("compat");

        assert!(manifest_file.contains("supported|IMap.lock|"));
        assert!(manifest_file.contains("supported|IMap.replace|"));
        assert!(manifest_file.contains("supported|IMap.remove(key,value)|"));
        assert!(docs.contains("Java/Spring Migration Contract"));
        assert!(docs.contains("HydraCacheMap<String, UserProfile>"));
        assert!(docs.contains("HydraFencedLock"));
        assert!(docs.contains("mode: native"));
        assert!(compat.contains("Java migration toolkit contract"));
        assert!(compat.contains("| Java migration toolkit contract | `2` |"));
        assert!(compat.contains("unsupported_hazelcast_apis.txt"));
    }

    #[test]
    fn java_lock_operation_maps_to_wire_family() {
        let cases = [
            (
                JavaLockOperation::Lock,
                JavaLockProtocolFamily::TryLock {
                    returns_fence: false,
                    blocking_wait: true,
                },
            ),
            (
                JavaLockOperation::LockAndGetFence,
                JavaLockProtocolFamily::TryLock {
                    returns_fence: true,
                    blocking_wait: true,
                },
            ),
            (
                JavaLockOperation::TryLock,
                JavaLockProtocolFamily::TryLock {
                    returns_fence: true,
                    blocking_wait: false,
                },
            ),
            (
                JavaLockOperation::TryLockTimed,
                JavaLockProtocolFamily::TryLock {
                    returns_fence: true,
                    blocking_wait: true,
                },
            ),
            (JavaLockOperation::Unlock, JavaLockProtocolFamily::Unlock),
            (
                JavaLockOperation::GetFence,
                JavaLockProtocolFamily::GetLockOwnership,
            ),
            (
                JavaLockOperation::IsLocked,
                JavaLockProtocolFamily::GetLockOwnership,
            ),
            (
                JavaLockOperation::IsLockedByCurrentThread,
                JavaLockProtocolFamily::GetLockOwnership,
            ),
            (
                JavaLockOperation::ForceUnlock,
                JavaLockProtocolFamily::ForceUnlock { privileged: true },
            ),
        ];

        for (operation, family) in cases {
            assert_eq!(operation.protocol_family(), family, "{operation:?}");
        }
    }

    #[test]
    fn force_unlock_is_marked_privileged() {
        assert!(JavaLockOperation::ForceUnlock.is_privileged());
        assert!(!JavaLockOperation::Unlock.is_privileged());
        assert_eq!(
            JavaLockOperation::ForceUnlock.protocol_family(),
            JavaLockProtocolFamily::ForceUnlock { privileged: true }
        );
    }

    #[test]
    fn java_runtime_config_modes_and_facade_contract_are_reviewable() {
        assert_eq!(SUPPORTED_SPRING_BOOT_GENERATIONS, &[2, 3, 4]);

        let config = JavaClientRuntimeConfig::client_first(
            [
                "https://cache-a.internal:8443",
                "https://cache-b.internal:8443",
            ],
            "core",
            "gameservice",
        );
        config.validate().unwrap();
        assert!(config.smart_routing);
        assert!(config.customizer_hooks_enabled);
        assert_eq!(config.deadline_ms, 5_000);

        let mut member = config.clone();
        member.topology = JavaClientTopology::Member;
        assert_eq!(
            member.validate().unwrap_err(),
            JavaMigrationContractError::UnsupportedClientTopology("member")
        );

        assert!(SpringCacheMode::Native.lazy_dynamic_cache_names());
        assert!(SpringCacheMode::JCache.requires_jcache_provider());
        assert!(!SpringCacheMode::None.lazy_dynamic_cache_names());

        assert_eq!(
            JavaMapOperation::Get.protocol_family(),
            JavaMapProtocolFamily::Get
        );
        assert_eq!(
            JavaMapOperation::ContainsKey.protocol_family(),
            JavaMapProtocolFamily::Get
        );
        assert_eq!(
            JavaMapOperation::PutIfAbsent.protocol_family(),
            JavaMapProtocolFamily::ConditionalPutIfAbsent
        );
        assert_eq!(
            JavaMapOperation::Replace.protocol_family(),
            JavaMapProtocolFamily::ConditionalReplace {
                expectation: JavaMapCasExpectation::ExactValue,
            }
        );
        assert_eq!(
            JavaMapOperation::ReplaceIfPresent.protocol_family(),
            JavaMapProtocolFamily::ConditionalReplace {
                expectation: JavaMapCasExpectation::Present,
            }
        );
        assert_eq!(
            JavaMapOperation::RemoveIfValue.protocol_family(),
            JavaMapProtocolFamily::ConditionalRemove
        );
        assert_eq!(
            JavaMapOperation::Remove.protocol_family(),
            JavaMapProtocolFamily::Invalidate
        );
        assert_eq!(
            JavaMapOperation::GetAll.protocol_family(),
            JavaMapProtocolFamily::BatchGet
        );
        assert_eq!(
            JavaMapOperation::PutAll.protocol_family(),
            JavaMapProtocolFamily::BatchPut
        );
        assert_eq!(
            JavaMapOperation::EvictRegion.protocol_family(),
            JavaMapProtocolFamily::EvictRegion
        );
    }
}
