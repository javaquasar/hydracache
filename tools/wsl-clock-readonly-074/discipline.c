/* Query only. modes=0 reads timex state; this helper has no setting mode. */
#define _POSIX_C_SOURCE 200809L
#include <stdio.h>
#include <sys/timex.h>
#include <unistd.h>
#include <time.h>

int main(void) {
    struct timespec raw_before, raw_after, boot, real, mono;
    if (clock_gettime(CLOCK_MONOTONIC_RAW, &raw_before) ||
        clock_gettime(CLOCK_BOOTTIME, &boot) ||
        clock_gettime(CLOCK_REALTIME, &real) ||
        clock_gettime(CLOCK_MONOTONIC, &mono)) {
        perror("clock read");
        return 1;
    }
    struct timex state = {0};
    const int result = adjtimex(&state);
    if (result < 0) {
        perror("adjtimex read-only");
        return 1;
    }
    if (clock_gettime(CLOCK_MONOTONIC_RAW, &raw_after)) {
        perror("raw after");
        return 1;
    }
    printf("{\"modes\":%u,\"result\":%d,\"tick_us\":%ld,"
           "\"frequency_scaled_ppm\":%ld,\"offset\":%ld,\"status\":%d,"
           "\"constant\":%ld,\"tolerance_scaled_ppm\":%ld,"
           "\"maxerror_us\":%ld,\"esterror_us\":%ld,\"precision_us\":%ld,"
           "\"tai\":%d,\"time_sec\":%ld,\"time_fraction\":%ld,"
           "\"user_hz\":%ld,\"euid\":%lu,"
           "\"raw_before_ns\":%lld,\"raw_after_ns\":%lld,"
           "\"boottime_ns\":%lld,\"realtime_ns\":%lld,\"monotonic_ns\":%lld}\n",
           state.modes, result, state.tick, state.freq, state.offset, state.status,
           state.constant, state.tolerance, state.maxerror, state.esterror,
           state.precision, state.tai, (long)state.time.tv_sec,
           (long)state.time.tv_usec, sysconf(_SC_CLK_TCK), (unsigned long)geteuid(),
           (long long)raw_before.tv_sec * 1000000000 + raw_before.tv_nsec,
           (long long)raw_after.tv_sec * 1000000000 + raw_after.tv_nsec,
           (long long)boot.tv_sec * 1000000000 + boot.tv_nsec,
           (long long)real.tv_sec * 1000000000 + real.tv_nsec,
           (long long)mono.tv_sec * 1000000000 + mono.tv_nsec);
    return 0;
}
