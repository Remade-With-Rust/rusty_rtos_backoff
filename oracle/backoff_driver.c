/* The C arm of K7's first differential: FreeRTOS's own backoffAlgorithm.
 *
 * `backoff_algorithm.c` is compiled VERBATIM from the pinned checkout
 * (`kairos oracle fetch --lib backoffAlgorithm`, v1.4.2 at 14f4c88) -- not
 * copied into this tree, not edited. It needs no shims at all: it includes
 * only <assert.h> and <stddef.h> and its own header, which is part of why it
 * is the right library to start K7 on.
 *
 * The trace it writes is checked in, so the Rust side diffs it in CI with no
 * C toolchain -- the same arrangement the kernel corpus and the three heap
 * differentials use.
 *
 * WHAT THIS WORKLOAD IS FOR
 *
 * A backoff context has exactly four interesting branches, and a run that
 * misses any of them proves correspondingly less:
 *
 *   1. the jitter maximum DOUBLING, while it is below half the ceiling;
 *   2. the jitter maximum CLAMPING to the ceiling, when it is not;
 *   3. attempts EXHAUSTED, which must leave the context untouched;
 *   4. RETRY_FOREVER, which must never exhaust.
 *
 * So the workload is a table of contexts chosen to hit all four -- including
 * an ODD ceiling, because the doubling test is integer division and an odd
 * ceiling clamps one step earlier than "half" suggests. The Rust side asserts
 * that every branch was reached rather than trusting the table.
 */

#include <stdint.h>
#include <stdio.h>

#include "backoff_algorithm.h"

/* The driver's LCG, written out in full so the Rust side reproduces it
 * exactly. `rand()` is implementation-defined and would make the two arms
 * incomparable by construction -- the same reason the heap drivers carry
 * their own. */
static uint32_t ulSeed = 12345u;

static uint32_t prvNext( void )
{
    ulSeed = ( ulSeed * 1664525u ) + 1013904223u;
    return ulSeed;
}

/* base, ceiling, attempts. */
struct Case
{
    uint16_t base;
    uint16_t max;
    uint32_t attempts;
    const char * why;
};

static const struct Case xCases[] =
{
    {  500, 10000,  5, "the coreMQTT defaults: doubles four times, then clamps" },
    {    1,     1,  8, "base == ceiling: clamps on the very first call" },
    {    1, 65535, 20, "the full u16 range, doubling all the way up" },
    {  100,   999, 10, "an ODD ceiling: integer division clamps early" },
    {    0,  1000,  6, "a zero base: the range is a single value, 0" },
    { 1000,   100,  6, "ceiling BELOW base: clamps down immediately" },
    {  250,  4000,  0, "zero attempts: exhausted before the first call" },
    {  250,  4000, BACKOFF_ALGORITHM_RETRY_FOREVER, "retry forever" },
};

#define CASES ( sizeof( xCases ) / sizeof( xCases[ 0 ] ) )

/* How many calls to make per case. Enough that RETRY_FOREVER is seen not to
 * exhaust, and that every bounded case runs PAST its limit so the exhausted
 * branch is exercised more than once. */
#define CALLS 24u

int main( void )
{
    printf( "geometry cases=%u calls=%u forever=%u\n",
            (unsigned) CASES, CALLS, (unsigned) BACKOFF_ALGORITHM_RETRY_FOREVER );

    for( unsigned c = 0; c < CASES; c++ )
    {
        BackoffAlgorithmContext_t xContext;
        BackoffAlgorithm_InitializeParams( &xContext,
                                           xCases[ c ].base,
                                           xCases[ c ].max,
                                           xCases[ c ].attempts );

        printf( "case %u %u %u %u\n",
                c,
                (unsigned) xCases[ c ].base,
                (unsigned) xCases[ c ].max,
                (unsigned) xCases[ c ].attempts );

        for( unsigned n = 0; n < CALLS; n++ )
        {
            uint32_t ulRandom = prvNext();
            uint16_t usBackoff = 0;
            BackoffAlgorithmStatus_t xStatus =
                BackoffAlgorithm_GetNextBackoff( &xContext, ulRandom, &usBackoff );

            /* Every observable the context has, after every call: the status,
             * the delay, and the two fields that carry state forward. A
             * transcription that got the delay right and the jitter climb
             * wrong would pass on the first line and fail on the second. */
            printf( "next %u %lu %s %u %u %lu\n",
                    n,
                    (unsigned long) ulRandom,
                    ( xStatus == BackoffAlgorithmSuccess ) ? "ok" : "exhausted",
                    (unsigned) usBackoff,
                    (unsigned) xContext.nextJitterMax,
                    (unsigned long) xContext.attemptsDone );
        }
    }

    printf( "end\n" );
    return 0;
}
