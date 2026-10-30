#include "wrap.h"
#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
extern int hc_peek(hc_snapshot *out);
int main(void) {
    hc_snapshot a, b;
    assert(!hc_peek(&a));
    assert(hc_begin()); assert(!hc_peek(NULL));
    hc_rust_enter(); assert(!hc_peek(&a)); hc_rust_leave();
    void *(*volatile alloc)(size_t) = malloc;
    void (*volatile dealloc)(void *) = free;
    unsigned char *p = alloc(128); assert(p); memset(p, 0x71, 128);
    assert(hc_peek(&a)); assert(hc_peek(&b));
    assert(!memcmp(&a, &b, sizeof(a)));
    assert(!a.errors && a.c.allocs == 1 && a.c.live == 128 && a.c.peak == 128);
    assert(!a.rust.allocs && !a.c.failed && !a.c.external_frees);
    dealloc(p); assert(hc_end(&b));
    assert(!b.errors && b.c.live == 0 && b.c.free_calls == 1);
    assert(!hc_peek(&a));
    puts("HC074_PEEK_CONTROL:{\"complete\":true}");
    return 0;
}
