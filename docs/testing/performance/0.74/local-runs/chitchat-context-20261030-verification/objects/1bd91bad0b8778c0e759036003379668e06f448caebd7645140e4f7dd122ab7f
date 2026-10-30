/* Appended after the byte-identical qualified wrap.c. No observer-state change. */
int hc_peek(hc_snapshot *out) {
    if (!state.active || state.depth || !out) return 0;
    *out = state.snap;
    return 1;
}
