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
const uintptr_t UB_BASE_READOUT = UB_BASE + 4 * 1024 * 1024;
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
    hardware_info.conflict_channels = Xil_In8(UB_BASE + 16);
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
    Xil_Out32(64 * 1024 + 4 * context_id, instruction);
#else
    uint64_t data = ((uint64_t)instruction) | (((uint64_t)context_id) << 32);
    Xil_Out64(UB_BASE + 4096, data);
#endif
}

void get_obstacle(struct ReadoutHead *head,
                  struct ReadoutConflict *conflicts,
                  uint8_t conflict_channels,
                  uint16_t context_id)
{
    uintptr_t base = UB_BASE_READOUT + 1024 * context_id;
    uint64_t raw_head = Xil_In64(base);
    head->growable = raw_head;
    head->accumulated_grown = raw_head >> 16;
    head->maximum_growth = raw_head >> 32;
    for (int i = 0; i < conflict_channels; ++i)
    {
        uintptr_t conflict_base = base + 32 + i * 16;
        uint64_t raw_1 = Xil_In64(conflict_base);
        uint64_t raw_2 = Xil_In64(conflict_base + 8);
        struct ReadoutConflict *conflict = conflicts + i;
        conflict->node_1 = raw_1;
        conflict->node_2 = raw_1 >> 16;
        conflict->touch_1 = raw_1 >> 32;
        conflict->touch_2 = raw_1 >> 48;
        conflict->vertex_1 = raw_2;
        conflict->vertex_2 = raw_2 >> 16;
        conflict->valid = raw_2 >> 32;
    }
}
