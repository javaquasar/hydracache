# Pre-capture canary authoring refusal

The initial native focused command runs the two old inventory assertions
successfully and one of the two new name-capacity assertions fails. The new
negative control replaces the first `revision: usize` in the fixture, which
belongs to Entry::open, not name(). The exact helper adapter correctly leaves
that unrelated change visible instead of panicking. The test expected a panic.

Target the complete `fn name(revision: usize` signature for this negative
control. Keep the separate outside-helper visibility assertion and all old
assertions unchanged. No product candidate, measurement or threshold has been
changed, and no measured invocation has occurred. This is a new test-authoring
refusal, not evidence of a runtime failure or performance result.
