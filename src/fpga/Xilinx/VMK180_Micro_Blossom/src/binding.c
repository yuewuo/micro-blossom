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
    uint64_t raw_1 = Xil_In64(UB_BASE + 8);
    uint32_t raw_2 = Xil_In32(UB_BASE + 16);
    hardware_info.version = raw_1;
    hardware_info.context_depth = raw_1 >> 32;
    hardware_info.conflict_channels = raw_2;
    hardware_info.vertex_bits = raw_2 >> 8;
    hardware_info.weight_bits = raw_2 >> 16;
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

const uint32_t INSTRUCTION_FIND_OBSTACLE = 0b0100;

// void get_conflicts(struct ReadoutHead *head,
//                    struct ReadoutConflict *conflicts,
//                    uint8_t conflict_channels,
//                    uint16_t context_id)
// {
//     execute_instruction(INSTRUCTION_FIND_OBSTACLE, context_id);
//     uintptr_t base = UB_BASE_READOUT + 1024 * context_id;
//     uint64_t raw_head = Xil_In64(base);
//     head->maximum_growth = raw_head;
//     head->accumulated_grown = raw_head >> 16;
//     head->growable = raw_head >> 32;
//     if (head->growable == 0)
//     { // read conflicts only when growable is zero
//         for (int i = 0; i < conflict_channels; ++i)
//         {
//             uintptr_t conflict_base = base + 32 + i * 16;
//             uint64_t raw_1 = Xil_In64(conflict_base);
//             uint64_t raw_2 = Xil_In64(conflict_base + 8);
//             struct ReadoutConflict *conflict = conflicts + i;
//             conflict->node_1 = raw_1;
//             conflict->node_2 = raw_1 >> 16;
//             conflict->touch_1 = raw_1 >> 32;
//             conflict->touch_2 = raw_1 >> 48;
//             conflict->vertex_1 = raw_2;
//             conflict->vertex_2 = raw_2 >> 16;
//             conflict->valid = raw_2 >> 32;
//         }
//     }
// }

// [warninng] only conflict channels = 1 is supported; use the above one when this is no longer true
// try to optimize this function as much as possible
void get_conflicts(struct ReadoutHead *head,
                   struct ReadoutConflict *conflicts,
                   uint8_t conflict_channels,
                   uint16_t context_id)
{
    assert(conflict_channels == 1);
    execute_instruction(INSTRUCTION_FIND_OBSTACLE, context_id);
    memcpy(head, (const void *)UB_BASE_READOUT, 8);
    if (head->growable == 0)
    {
        memcpy(&conflicts[0], (const void *)(UB_BASE_READOUT + 32), 8);
    }
}

void set_maximum_growth(uint16_t length, uint16_t context_id)
{
    uintptr_t base = UB_BASE_READOUT + 1024 * context_id;
    Xil_Out16(base, length);
}
