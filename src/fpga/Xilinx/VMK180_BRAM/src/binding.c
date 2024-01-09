#include <stdio.h>
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

uint32_t test_read32()
{
    return Xil_In32(BRAM_BASE);
}

void test_write32(uint32_t value)
{
    Xil_Out32(BRAM_BASE, value);
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
