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

const uintptr_t UB_BASE = 0x400000000;
const uintptr_t UB_BASE_READOUT = UB_BASE + 128 * 1024;
#define UB_CONTEXT(context_id) (UB_BASE_READOUT + 128 * (context_id))
#define RESET_INSTRUCTION (0x00000024)
#define FIND_OBSTACLE_INSTRUCTION (0x00000004)

const float TIMER_FREQUENCY = 200e6; // 200MHz

uint64_t get_native_time()
{
    return Xil_In64(UB_BASE);
}

float get_native_frequency()
{
    return TIMER_FREQUENCY;
}

float diff_native_time(uint64_t start, uint64_t end)
{
    // it's impossible for a 64 bit timer to overflow
    return (float)(end - start) / TIMER_FREQUENCY;
}

MicroBlossomHardwareInfo get_hardware_info()
{
    MicroBlossomHardwareInfo hardware_info;
    memcpy((void *)&hardware_info, (const void *)(UB_BASE + 8), 16);
    return hardware_info;
}

void clear_instruction_counter()
{
    Xil_Out32(UB_BASE + 24, 0);
}

uint32_t get_instruction_counter()
{
    return Xil_In32(UB_BASE + 24);
}

void clear_error_counter()
{
    Xil_Out32(UB_BASE + 48, 0);
}

uint32_t get_error_counter()
{
    return Xil_In32(UB_BASE + 48);
}

void execute_instruction(uint32_t instruction, uint16_t context_id)
{
#ifdef ARMR5
    Xil_Out32(UB_BASE + 8192 + 4 * context_id, instruction);
#else
    uint64_t data = ((uint64_t)instruction) | (((uint64_t)context_id) << 32);
    Xil_Out64(UB_BASE + 4096, data);
#endif
}

void set_maximum_growth(uint16_t length, uint16_t context_id)
{
    Xil_Out16(UB_CONTEXT(context_id) + 16, length);
}

uint16_t get_maximum_growth(uint16_t context_id)
{
    return Xil_In16(UB_CONTEXT(context_id) + 16);
}

SingleReadout get_single_readout(uint16_t context_id)
{
    SingleReadout readout;
    memcpy((void *)&readout, (const void *)(UB_CONTEXT(context_id) + 32), 16);
    Xil_Out16(UB_CONTEXT(context_id), 0); // clear grown
    return readout;
}

void reset_context(uint16_t context_id)
{
    execute_instruction(RESET_INSTRUCTION, context_id);
    // find obstacle to make sure the reset instruction is flushed
    get_single_readout(context_id);
}

void reset_all(uint16_t context_depth)
{
    for (uint16_t context_id = 0; context_id < context_depth; ++context_id)
    {
        execute_instruction(RESET_INSTRUCTION, context_id);
        // prefetching: reduce the time of waiting for responses on individual context
        execute_instruction(FIND_OBSTACLE_INSTRUCTION, context_id);
    }
    for (uint16_t context_id = 0; context_id < context_depth; ++context_id)
    {
        get_single_readout(context_id);
    }
}

uint64_t get_fast_cpu_time()
{
    // XTime time_val;
    // XTime_GetTime(&time_val);
    // return time_val;
    // 2024.6.7: the above is not fast at all: about 258ns per function call
    // use the CNTPCT_EL0 register instead (only applicable to aarch64)
    uint64_t cntpct;
    // isb();  // no need in our purpose (just get a rough time estimation but need to be extremely fast)
    asm volatile("mrs %0, cntpct_el0" : "=r" (cntpct));
    return cntpct;
    // benchmark:
    //    with isb: 42ns
    //    without isb: 6.4ns
}

uint64_t get_fast_cpu_duration_ns(uint64_t start)
{
    uint32_t now = get_fast_cpu_time();
    uint64_t cntfrq;
    asm volatile("mrs %0, cntfrq_el0" : "=r" (cntfrq));
    return (float)(now - start) / (float)cntfrq * 1e9;
}

 void setup_load_stall_emulator(uint64_t start_time, uint32_t interval, uint16_t context_id) {
    Xil_Out64(UB_CONTEXT(context_id) + 112, start_time);
    Xil_Out32(UB_CONTEXT(context_id) + 120, interval);
}

uint64_t get_last_load_time(uint16_t context_id) {
    return Xil_In64(UB_CONTEXT(context_id));
}

uint64_t get_last_finish_time(uint16_t context_id) {
    return Xil_In64(UB_CONTEXT(context_id) + 8);
}