#include <stdio.h>
#include <assert.h>
#include "binding.h"
#include "xil_types.h"
#include "xil_printf.h"
#include "xil_io.h"
#include "xparameters_ps.h"
#include "xiltimer.h"

void print_char(char c)
{
    putchar(c);
}

#ifdef ARMR5
const UINTPTR BRAM_BASE = 0x80000000;
#else
const UINTPTR BRAM_BASE = 0xA4000000;
#endif

// // cross-access, not optimal, but could work (A72 access LPD AXI, R5F access FPD AXI...)
// #ifdef ARMR5
// const UINTPTR BRAM_BASE = 0xA4000000;
// #else
// const UINTPTR BRAM_BASE = 0x80000000;
// #endif

uint32_t test_read32(uint32_t bias)
{
    // assert(bias < 8192);
    return Xil_In32(BRAM_BASE + bias);
}

void test_write32(uint32_t bias, uint32_t value)
{
    // assert(bias < 8192);
    Xil_Out32(BRAM_BASE + bias, value);
}

uint64_t test_read64(uint32_t bias)
{
    // assert(bias < 8192);
    return Xil_In64(BRAM_BASE + bias);
}

void test_write64(uint32_t bias, uint64_t value)
{
    // assert(bias < 8192);
    Xil_Out64(BRAM_BASE + bias, value);
}

void test_read128(uint32_t bias, uint64_t (*values)[2])
{
    memcpy(values, (const void *)(BRAM_BASE + bias), 16);
}

void test_read256(uint32_t bias, uint64_t (*values)[4])
{
    test_read128(bias, (uint64_t(*)[2])(values));
    test_read128(bias + 16, (uint64_t(*)[2])(((uint64_t *)values) + 2));
}

void set_leds(uint32_t)
{
}

uint64_t get_native_time()
{
    XTime time_val;
    XTime_GetTime(&time_val);
    return time_val;
}

float diff_native_time(uint64_t start, uint64_t end)
{
    XTime native_duration;
    if (end < start)
    {
        // note that XTime_GetTime actually returns 32 bit clock, even though XTime definition is 64 bits....
        native_duration = (((uint32_t)-1) - start) + end;
    }
    else
    {
        native_duration = end - start;
    }
    return (float)native_duration / (float)(COUNTS_PER_SECOND);
}
