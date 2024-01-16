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

const UINTPTR UB_BASE = 0x400000000;
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
    hardware_info.version = Xil_In32(UB_BASE + 8);
    hardware_info.context_depth = Xil_In32(UB_BASE + 12);
    hardware_info.obstacle_channels = Xil_In8(UB_BASE + 16);
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

void execute_instruction(uint32_t instruction, uint16_t context_id)
{
#ifdef ARMR5
#error "not implemented"
#else
    uint64_t data = ((uint64_t)instruction) | (((uint64_t)context_id) << 32);
    Xil_Out64(UB_BASE + 4096, data);
#endif
}
