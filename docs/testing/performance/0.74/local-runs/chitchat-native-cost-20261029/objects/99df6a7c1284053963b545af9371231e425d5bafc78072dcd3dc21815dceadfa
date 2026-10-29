#include "wrap.h"
#include <assert.h>
#include <errno.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static hc_snapshot finish(void) { hc_snapshot s; assert(hc_end(&s)); return s; }
static void golden(hc_stats s) {
    assert(s.allocs == 2 && s.realloc_calls == 1 && s.requested == 512);
    assert(s.live == 0 && s.peak == 256 && s.failed == 0 && s.external_frees == 0);
}
static void *thread(void *out) {
    assert(hc_begin()); assert(hc_c_probe()); *(hc_snapshot *)out = finish(); return NULL;
}
int main(void) {
    void *(*volatile m)(size_t) = malloc;
    void *(*volatile c)(size_t, size_t) = calloc;
    void *(*volatile r)(void *, size_t) = realloc;
    void (*volatile f)(void *) = free;
    assert(hc_begin()); assert(hc_c_probe()); hc_snapshot s = finish();
    golden(s.c); assert(!s.errors && !s.rust.allocs);
    assert(hc_begin()); hc_rust_enter(); hc_rust_enter(); assert(hc_c_probe());
    hc_rust_leave(); hc_rust_leave(); s = finish(); golden(s.rust); assert(!s.errors && !s.c.allocs);

    assert(hc_begin()); unsigned char *p = m(64); assert(p); memset(p, 0x6b, 64);
    errno = 0; void *q = r(p, SIZE_MAX); assert(!q && errno == ENOMEM);
    for (size_t i = 0; i < 64; ++i) assert(p[i] == 0x6b);
    errno = EDOM; f(p); assert(errno == EDOM);
    errno = 0; assert(!c(SIZE_MAX, 2) && errno == ENOMEM);
    s = finish(); assert(!s.errors && s.c.failed == 2 && s.c.live == 0 && s.c.requested == 64);

    assert(hc_begin()); p = m(32); assert(p); q = r(p, 0); assert(!q); /* Observed GNU contract. */
    s = finish(); assert(!s.errors && !s.c.failed && s.c.live == 0 && s.c.realloc_calls == 1);
    assert(hc_begin()); assert(!hc_begin()); assert(hc_c_probe()); s = finish(); golden(s.c);
    assert(!hc_end(&s));

    void *pointers[4097]; assert(hc_begin());
    for (size_t i = 0; i < 4097; ++i) { pointers[i] = m(8); assert(pointers[i]); }
    for (size_t i = 0; i < 4097; ++i) f(pointers[i]);
    s = finish(); assert(s.errors == 1 && s.c.external_frees == 1 && s.c.live == 0);
    pthread_t threads[2]; hc_snapshot results[2];
    for (size_t i = 0; i < 2; ++i) assert(!pthread_create(&threads[i], NULL, thread, &results[i]));
    for (size_t i = 0; i < 2; ++i) {
        assert(!pthread_join(threads[i], NULL)); golden(results[i].c);
        assert(!results[i].errors && !results[i].rust.allocs);
    }
    puts("HC074_NATIVE_C_CONTROLS:{\"complete\":true,\"groups\":7,\"thread_isolation\":true}");
    return 0;
}
