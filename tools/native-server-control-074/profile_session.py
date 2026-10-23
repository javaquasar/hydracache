"""Render one attribution-only session from the frozen holder's safety frame.

No process, network, filesystem write or admission entry point. The consumed
holder run is not rerun: only its unchanged image and pre/postflight are reused.
"""
import hashlib

TEMPLATE_SHA256 = 'e6a2bf2b654ab9a25888bb96bcfb94463c1d66b7c1c577c3927097fdba4dccf5'
SELECTOR = ('diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::'
            'tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::'
            'collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::'
            'starting_tests::privileged_root::cgroup::held_live::privileged_consuming_held_live_matrix')


def render(template):
    """Keep every original safety predicate; add observers, never guard clocks."""
    if hashlib.sha256(template).hexdigest() != TEMPLATE_SHA256:
        raise ValueError('Frozen holder safety frame changed')
    text = template.decode('ascii')
    # The checked-out source is CRLF on Windows; preserve all other bytes.
    line_end = '\r\n' if '\r\n' in text else '\n'
    normalized = text.replace('\r\n', '\n')
    marker = 'selector=' + SELECTOR + '\n'
    if normalized.count(marker) != 1:
        raise ValueError('Ambiguous selector boundary')
    prefix = normalized.split(marker)[0]
    for before, after in (
        ('/tmp/hc074-native-held-root-20261023.??????', '/tmp/hc074-native-cost-root-20261024.??????'),
        ('mkdir holder-once', 'mkdir profile-once'),
        ('holder-session.sh', 'profile-session.sh'),
        ('HC074_NATIVE_HOLDER exit=', 'HC074_NATIVE_PROFILE exit='),
    ):
        if prefix.count(before) != 1:
            raise ValueError('Ambiguous safety frame edit: ' + before)
        prefix = prefix.replace(before, after)
    suffix = '''result=0
/usr/bin/env -i PATH=/usr/sbin:/usr/bin:/sbin:/bin \\
 HC074_OWNED_ROOT_FIXTURE=local-only-v1 HC074_START_MATERIAL_BUNDLE="$root/bundle" \\
 HC074_CONSUMING_HELD_LIVE_CASE=healthy HC074_CONSUMING_HELD_COST=1 \\
 /usr/bin/timeout --signal=TERM --kill-after=5s 1200s \\
 nice -n 10 "$root/libtest" "$selector" --ignored --exact --nocapture --test-threads=1 \\
 >matrix.log 2>&1 || result=$?
printf '%s\\n' "$result" >matrix.exit
sync -f "$root/matrix.log"
# A failed profile remains useful attribution and remains a failed test.
# Never continue to another case, dispatch a second attempt or grant admission.
exit "$result"
'''
    return (prefix + marker + suffix).replace('\n', line_end).encode('ascii')
