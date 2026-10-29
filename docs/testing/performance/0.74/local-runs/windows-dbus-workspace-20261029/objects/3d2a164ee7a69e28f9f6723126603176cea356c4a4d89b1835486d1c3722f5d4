"""Compose one existing dispatch, frozen observer and its original held outputs.

No launch API, arbitrary log paths, receipt mutation or execution authority.
"""

from launch_preflight import decode
from owned_process_observer import Refusal
from target_result_join import join_target


class BoundTargetObservation:
    def __init__(self, dispatch, outputs, catalog):
        if dispatch.attempt_id != outputs.attempt_id or dispatch.directory != outputs.reservation:
            raise Refusal("dispatch/output reservation mismatch")
        if dispatch.owner is None or dispatch.owner is not outputs.owner or dispatch.custody is not outputs.custody:
            raise Refusal("dispatch/output original owner mismatch")
        if not outputs.attach_attempted or outputs.first_refusal is not None:
            raise Refusal("original outputs were not bound live")
        self.dispatch, self.outputs, self.catalog = dispatch, outputs, frozenset(catalog)
        self.attempted = False
        self.first_refusal = None
        self.binding = None

    def finish(self):
        if self.attempted:
            raise Refusal("bound result join cannot be repeated")
        self.attempted = True
        try:
            custody = self.dispatch.custody
            if custody.observation is None or custody.exit_proof is None:
                raise Refusal("frozen observation and terminal proof required")
            raw, binding = self.outputs.seal(custody)
            self.binding = binding
            intent, result = decode(raw["intent.json"]), decode(raw["result.json"])
            wrapper_exit = int(result["exit"] != 0 or result["outer_refusal"] is not None
                               or result["source_unchanged"] is not True or result["head_unchanged"] is not True)
            if custody.exit_proof["owner_returncode"] != wrapper_exit:
                raise Refusal("wrapper owner exit contradicts original result")
            target = join_target(intent, result, raw["stdout.log"].decode("utf-8"),
                                 raw["stderr.log"].decode("utf-8"), self.outputs.expected, self.catalog)
            target["live_attempt_binding_required"] = False
            if not custody.release_attempted:
                custody.finalize_observation()
            if not custody.released or custody.custody_refusal is not None:
                raise Refusal("held observation cleanup not proven")
            return dict(binding=binding, target=target, dispatch=self.dispatch.result(), observation=custody.result(),
                        diagnostic_pass=target["selected_target_pass"] and custody.observation["observations_complete"]
                            and self.dispatch.acknowledged and self.dispatch.first_refusal is None,
                        full_package_pass=False, actual_cargo_shape_proven=False,
                        all_writers_proven_stopped=False, execution_completion_authorized=False, admission=False)
        except Exception as error:
            self.first_refusal = str(error)
            raise
