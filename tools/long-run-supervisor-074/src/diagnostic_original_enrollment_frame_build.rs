//! Original custody recipe with enrollment before its unchanged closing round.
use super::*;
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCollectorSinks<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_original_enrollment_frame(
        self,
    ) -> Result<
        OriginalEnrollmentFrame<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        EnrollmentFrameFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        self.prepare_enrollment_frame_with(
            #[cfg(test)]
            None,
            #[cfg(test)]
            0,
            #[cfg(test)]
            &mut |_, _| {},
        )
    }
    pub(super) fn prepare_enrollment_frame_with(
        self,
        #[cfg(test)] fixture: Option<&std::path::Path>,
        #[cfg(test)] fault: u8,
        #[cfg(test)] checkpoint: &mut dyn FnMut(
            &'static str,
            &mut OriginalEnrollmentFrame<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ),
    ) -> Result<
        OriginalEnrollmentFrame<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        EnrollmentFrameFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        #[cfg(test)]
        let allow_fixture = fixture.is_some();
        #[cfg(not(test))]
        let allow_fixture = false;
        let mut held = OriginalEnrollmentFrame {
            collector: None,
            custodian: None,
            inventory: None,
            ready: false,
            allow_fixture,
            original: OriginalCustodiedPreparation {
                original: OriginalGroupedPreparation {
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
                },
                custody: None,
                ready: false,
                #[cfg(test)]
                final_delay: false,
            },
        };
        let result: Result<(), BarredCustodyError> = (|| {
            let original = &mut held.original;
            original
                .original
                .transport
                .material
                .outputs
                .material_round()
                .map_err(OriginalStartMaterialError::from)
                .map_err(OriginalPipeTransportError::from)
                .map_err(OriginalCustodyError::from)?;
            let material = &mut original.original.transport.material;
            let prepared = &material.outputs.pipes.starting.original.original.prepared;
            let r = &prepared.original.original.reservation;
            let body = prepared.consent_to_sign();
            material.projection = requirements(
                &r.state,
                &body.collector_unit_sha256,
                &body.reserved_state_sha256,
            )
            .map_err(OriginalPipeTransportError::from)
            .map_err(OriginalCustodyError::from)?;
            material.starting = Some(body.starting_state.clone());
            material.bundle = Some(
                load_bundle(
                    &r.original.original.policy,
                    r.original.original.build.identity(),
                    #[cfg(test)]
                    fixture,
                )
                .map_err(OriginalStartMaterialError::from)
                .map_err(OriginalPipeTransportError::from)
                .map_err(OriginalCustodyError::from)?,
            );
            original
                .original
                .transport
                .material
                .bind(allow_fixture)
                .map_err(OriginalPipeTransportError::from)
                .map_err(OriginalCustodyError::from)?;
            let material = &original.original.transport.material;
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
            let spec =
                worker(&r.state, &material.projection).map_err(OriginalCustodyError::from)?;
            let [Some(stdout), Some(stderr)] = &material.outputs.pipes.pipes else {
                return Err(OriginalCustodyError::from(OriginalGroupedPrepError::Binding).into());
            };
            original.original.transport.message = Some(
                message(&spec, &stdout.writer, &stderr.writer)
                    .map_err(OriginalCustodyError::from)?,
            );
            inspect_message(
                original.original.transport.message.as_ref().unwrap(),
                &spec,
                &stdout.writer,
                &stderr.writer,
            )
            .map_err(OriginalCustodyError::from)?;
            let material = &mut original.original.transport.material;
            let prepared = &mut material.outputs.pipes.starting.original.original.prepared;
            let previous = prepared
                .point
                .as_ref()
                .ok_or(OriginalGroupedPrepError::Binding)
                .map_err(OriginalCustodyError::from)?;
            let progress = &mut prepared.original.original;
            let state = material
                .starting
                .as_ref()
                .ok_or(OriginalGroupedPrepError::Binding)
                .map_err(OriginalCustodyError::from)?;
            let now = progress
                .packet
                .original_reservation_time(Some(previous))
                .map_err(OriginalCustodyError::from)?;
            let bounds = boundaries(
                state.reserved_monotonic_ns,
                state
                    .cell_started_monotonic_ns
                    .ok_or(OriginalGroupedPrepError::Binding)
                    .map_err(OriginalCustodyError::from)?,
                progress.packet.received_time().boottime_ns(),
                state.last_observed_monotonic_ns,
                now.boottime_ns(),
            )
            .map_err(OriginalCustodyError::from)?;
            original.original.last = Some(now);
            original.original.bounds = Some(bounds);
            original.original.alarm =
                Some(Alarm::arm(bounds.first()).map_err(OriginalCustodyError::from)?);
            let storage = &mut original
                .original
                .transport
                .material
                .outputs
                .pipes
                .starting
                .original
                .original
                .prepared
                .original
                .original
                .reservation
                .storage;
            original.custody = Some(
                storage
                    .retain_original_collector_custody()
                    .map_err(OriginalCustodyError::from)?,
            );
            original.inspect_custody()?;
            #[cfg(test)]
            checkpoint("retained", &mut held);
            // Only the approved intermediate observation frequency changes.
            // Both children remain barred throughout opening/closing validation.
            held.enroll(
                #[cfg(test)]
                fault,
            )?;
            #[cfg(test)]
            checkpoint("enrolled", &mut held);
            let original = &mut held.original;
            if original
                .original
                .original_round(allow_fixture)
                .map_err(OriginalCustodyError::from)?
                != bounds
            {
                return Err(OriginalCustodyError::from(OriginalGroupedPrepError::Binding).into());
            }
            let alarm = original
                .original
                .alarm
                .as_ref()
                .ok_or(OriginalGroupedPrepError::Binding)
                .map_err(OriginalCustodyError::from)?;
            if alarm.deadline != bounds.first() {
                return Err(OriginalCustodyError::from(OriginalGroupedPrepError::Binding).into());
            }
            original.inspect_custody()?;
            original.inspect_deadline()?;
            held.observe()?;
            // No ordinary callback, send or start follows this closing observation.
            held.original.original.transport.material.ready = true;
            held.original.ready = true;
            held.ready = true;
            #[cfg(test)]
            checkpoint("closed", &mut held);
            Ok(())
        })();
        match result {
            Ok(()) => Ok(held),
            Err(error) => {
                held.original.refuse();
                Err(EnrollmentFrameFailure {
                    error,
                    held: Box::new(held),
                })
            }
        }
    }
}
