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

const float TIMER_FREQUENCY = 200e6; // 200MHz

uint64_t get_native_time()
{
    return Xil_In64(UB_BASE);
}

float diff_native_time(uint64_t start, uint64_t end)
{
    // it's impossible for a 64 bit timer to overflow
    return (float)(end - start) / TIMER_FREQUENCY;
}

MicroBlossomHardwareInfo get_hardware_info()
{
    MicroBlossomHardwareInfo hardware_info;
    memcpy((void *)&hardware_info, (const void *)(UB_BASE + 8), 8);
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
    memcpy((void *)&readout, (const void *)(UB_CONTEXT(context_id) + 32), 8);
    Xil_Out16(UB_CONTEXT(context_id), 0); // clear grown
    return readout;
}
