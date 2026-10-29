#ifndef HC074_NATIVE_WRAP_H
#define HC074_NATIVE_WRAP_H
#include <stdint.h>
typedef struct {
    uint64_t malloc_calls, calloc_calls, realloc_calls, free_calls;
    uint64_t allocs, requested, failed, external_frees;
    int64_t live, peak;
} hc_stats;
typedef struct { hc_stats c, rust; uint64_t errors; } hc_snapshot;
int hc_begin(void);
int hc_end(hc_snapshot *out);
void hc_rust_enter(void);
void hc_rust_leave(void);
int hc_c_probe(void);
#endif
