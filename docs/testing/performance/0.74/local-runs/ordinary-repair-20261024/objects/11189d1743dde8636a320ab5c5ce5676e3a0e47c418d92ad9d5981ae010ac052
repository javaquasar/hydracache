//! Privileged local fixture seam, not installed bundle or production issuer.
use super::*;
impl<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    OriginalCollectorSinks<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn prepare_privileged_enrollment_for_test(
        self,
        path: &std::path::Path,
    ) -> Result<
        OriginalEnrollmentFrame<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        EnrollmentFrameFailure<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        crate::diagnostic_two_phase_launch::root_enrollment::privileged_fixture::require(
            path.parent().unwrap(),
        );
        assert_eq!(path.file_name().unwrap(), "bundle");
        self.prepare_enrollment_frame_with(Some(path), 0, &mut |_, _| {})
    }
}
