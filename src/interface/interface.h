#include "stdint.h"

typedef struct
{
    unsigned char type : 4;
} BroadcastMessage;

typedef struct
{

} ConvergecastMessage;

/*
 * In the algorithm we always use full address to distinguish vertices between fusion partitions,
 * but such a 64-bit full address is too large for distributed dual module logic.
 * The dual driver should convert the full address `VertexFullAddress` to a local address `VertexAddress`
 * and vice versa when it interacts with the distributed dual module.
 * The dual driver needs to maintain an offset value of type `VertexFullAddress` to do this conversion.
 */
typedef struct
{
    uint32_t t;
    uint16_t i;
    uint16_t j;
} VertexFullAddress;

#ifndef VERTEX_T_BITS
#define VERTEX_T_BITS 6 // t <= 64
#endif

#ifndef VERTEX_I_BITS
#define VERTEX_I_BITS 5 // i <= 32
#endif

#ifndef VERTEX_J_BITS
#define VERTEX_J_BITS 5 // j <= 32
#endif

#ifndef VERTEX_BITS_DATA_TYPE
#define VERTEX_BITS_DATA_TYPE uint16_t // 6 + 5 + 5 = 16 bits
#endif

typedef struct
{
    VERTEX_BITS_DATA_TYPE t : VERTEX_T_BITS;
    VERTEX_BITS_DATA_TYPE i : VERTEX_I_BITS;
    VERTEX_BITS_DATA_TYPE j : VERTEX_J_BITS;
} VertexAddress;
