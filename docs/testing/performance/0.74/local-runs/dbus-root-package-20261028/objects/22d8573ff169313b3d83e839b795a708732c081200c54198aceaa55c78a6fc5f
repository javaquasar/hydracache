//! Test-only grouped schedule. No old validator or production observation changes.
use super::*;

impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCollectorSinks<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_grouped_fixture_for_test(
        self,
        path: &std::path::Path,
        mut checkpoint: impl FnMut(
            &'static str,
            &mut OriginalCollectorSafetyAlarm<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ),
    ) -> Result<
        OriginalCollectorSafetyAlarm<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        OriginalSafetyAlarmFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        assert_ne!(unsafe { libc::geteuid() }, 0);
        assert_ne!(unsafe { libc::getegid() }, 0);
        let mut held = OriginalCollectorSafetyAlarm {
            alarm: None,
            bounds: None,
            last: None,
            transport: OriginalCollectorPipeTransport {
                message: None,
                material: OriginalCollectorStartMaterial {
                    bundle: None,
                    projection: Vec::new(),
                    starting: None,
                    ready: false,
                    outputs: self,
                },
            },
        };
        let result = (|| {
            held.transport
                .material
                .outputs
                .material_round()
                .map_err(OriginalStartMaterialError::from)
                .map_err(OriginalPipeTransportError::from)?;
            checkpoint("opened", &mut held);
            let material = &mut held.transport.material;
            let prepared = &material.outputs.pipes.starting.original.original.prepared;
            let r = &prepared.original.original.reservation;
            let body = prepared.consent_to_sign();
            material.projection = requirements(
                &r.state,
                &body.collector_unit_sha256,
                &body.reserved_state_sha256,
            )
            .map_err(OriginalPipeTransportError::from)?;
            material.starting = Some(body.starting_state.clone());
            // Same signed, nonexecuted fixture inspector and reviewed dummy key
            // as the original material fixture, never fixed-origin admission.
            let policy: crate::diagnostic_builder::BuilderPolicy =
                serde_json::from_slice(include_bytes!(
                    "../../../docs/testing/performance/0.74/diagnostic-builder-public-policy.json"
                ))
                .unwrap();
            let trust = crate::diagnostic_artifacts::BuildTrust {
                key: crate::diagnostic_builder::decode_key(&policy.builder_key_hex).unwrap(),
                repository_id: policy.repository_id,
                builder_id: policy.builder_id,
            };
            material.bundle = Some(
                crate::diagnostic_artifacts::linux::inspect_fixture(
                    path,
                    unsafe { libc::geteuid() },
                    unsafe { libc::getegid() },
                    r.original.original.build.identity(),
                    &trust,
                )
                .map_err(OriginalStartMaterialError::from)
                .map_err(OriginalPipeTransportError::from)?,
            );
            checkpoint("loaded", &mut held);
            held.transport
                .material
                .bind(true)
                .map_err(OriginalPipeTransportError::from)?;
            checkpoint("bound", &mut held);
            let material = &held.transport.material;
            let r = &material
                .outputs
                .pipes
                .starting
                .original
                .original
                .prepared
                .original
                .original
                .reservation;
            let spec = worker(&r.state, &material.projection)?;
            let [Some(stdout), Some(stderr)] = &material.outputs.pipes.pipes else {
                return Err(OriginalSafetyAlarmError::Binding);
            };
            held.transport.message = Some(message(&spec, &stdout.writer, &stderr.writer)?);
            inspect_message(
                held.transport.message.as_ref().unwrap(),
                &spec,
                &stdout.writer,
                &stderr.writer,
            )?;
            checkpoint("message", &mut held);

            // Provisional timer only. The unchanged original_round below must
            // independently confirm every binding and these original bounds.
            let material = &mut held.transport.material;
            let prepared = &mut material.outputs.pipes.starting.original.original.prepared;
            let previous = prepared
                .point
                .as_ref()
                .ok_or(OriginalSafetyAlarmError::Binding)?;
            let progress = &mut prepared.original.original;
            let state = material
                .starting
                .as_ref()
                .ok_or(OriginalSafetyAlarmError::Binding)?;
            let now = progress.packet.original_reservation_time(Some(previous))?;
            let bounds = boundaries(
                state.reserved_monotonic_ns,
                state
                    .cell_started_monotonic_ns
                    .ok_or(OriginalSafetyAlarmError::Binding)?,
                progress.packet.received_time().boottime_ns(),
                state.last_observed_monotonic_ns,
                now.boottime_ns(),
            )?;
            held.last = Some(now);
            held.bounds = Some(bounds);
            held.alarm = Some(Alarm::arm(bounds.first())?);
            checkpoint("armed", &mut held);
            // No caller callback occurs after this complete closing observation.
            if held.original_round(true)? != bounds {
                return Err(OriginalSafetyAlarmError::Binding);
            }
            let alarm = held
                .alarm
                .as_ref()
                .ok_or(OriginalSafetyAlarmError::Binding)?;
            if alarm.deadline != bounds.first() {
                return Err(OriginalSafetyAlarmError::Binding);
            }
            alarm.inspect()?;
            held.transport.material.ready = true;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(held),
            Err(error) => {
                held.transport.material.outputs.refuse();
                Err(OriginalSafetyAlarmFailure {
                    error,
                    held: Box::new(held),
                })
            }
        }
    }
}

impl OriginalCollectorSafetyAlarm<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn tamper_grouped_for_test(&mut self, kind: &str) {
        match kind {
            "message" => self.transport.message = None,
            "projection" => self.transport.material.projection.push(b' '),
            "state" => self.transport.material.starting.as_mut().unwrap().revision += 1,
            "writer" => self
                .transport
                .material
                .outputs
                .corrupt_descriptor_for_test(3),
            "alarm" => super::set(self.alarm.as_ref().unwrap(), 0, 0, libc::TFD_TIMER_ABSTIME),
            _ => panic!("unknown grouped fault"),
        }
    }
}
impl OriginalSafetyAlarmFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn retry_grouped_for_test(&mut self) -> Result<(), OriginalSafetyAlarmError> {
        self.held.revalidate_fixture_for_test()
    }
}
