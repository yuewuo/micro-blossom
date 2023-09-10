#include "stdint.h"

#define WEIGHT_DATA_TYPE uint8_t

/*
 * A local vertex address is the coordinate system used in a single FPGA decoding block.
 * Usually, the size of the decoding block is limited by the resources in the FPGA,
 * which means we could use much fewer bits to identify each vertex in the block.
 * When a vertex is mirrored, e.g. the real vertex exists in another block, the software
 * is responsible for mirroring their behavior when fusing them.
 * FPGA side need this `mirror` bit to determine whether it should notify the CPU side
 * of any state updates of the vertex
 */
#define LOCAL_VERTEX_DATA_TYPE uint16_t
typedef LOCAL_VERTEX_DATA_TYPE LocalVertex;
typedef LOCAL_VERTEX_DATA_TYPE LocalNode;

/*
 * Decoding block requires larger address
 */
#define T_BIAS_DATA_TYPE uint32_t      // 2^32 measurement rounds
#define BLOCK_INDEX_DATA_TYPE uint16_t // 2^16=65536 logical qubits

typedef struct
{
    T_BIAS_DATA_TYPE t_bias;
    BLOCK_INDEX_DATA_TYPE block_idx;
    LocalVertex local;
} GlobalVertex;

typedef struct
{
    T_BIAS_DATA_TYPE t_bias;
    BLOCK_INDEX_DATA_TYPE block_idx;
    LocalNode local;
} GlobalNode;

/*
 * Broadcast Message
 */

enum __attribute__((__packed__)) BroadcastType
{
    Grow = 0b00,
    SetSpeed = 0b01,
    SetParent = 0b10,
};

// use one-hot encoding for simpler hardware
enum __attribute__((__packed__)) Speed
{
    Stop = 0b00,
    Plus = 0b01,
    Minus = 0b10,
};

typedef struct
{
    WEIGHT_DATA_TYPE length;
} BoardcastMessageGrow;

typedef struct
{
    LocalNode node;
    Speed speed;
} BoardcastMessageSetSpeed;

typedef struct
{
    LocalNode node;
    LocalNode parent;
} BoardcastMessageSetParent;

typedef struct
{
    BroadcastType type;
    union
    {
        BoardcastMessageGrow grow;
        BoardcastMessageSetSpeed set_speed;
        BoardcastMessageSetParent set_parent;
    } data;
} BroadcastMessage;

/*
 * Convergecast Message
 */

enum __attribute__((__packed__)) ConvergecastType
{
    NonZeroGrow = 0b100,
    Conflict = 0b000,
    TouchingVirtual = 0b010,
    BlossomNeedExpand = 0b001,
};

typedef struct
{
    WEIGHT_DATA_TYPE length;
} ConvergecastMessageNonZeroGrow;

typedef struct
{
    LocalNode node_1;
    LocalVertex touch_1;
    LocalNode node_2;
    LocalVertex touch_2;
} ConvergecastMessageConflict;

typedef struct
{
    LocalNode node_1;
    LocalVertex touch_1;
    LocalVertex vertex; // it touches a virtual vertex
} ConvergecastMessageTouchingVirtual;

typedef struct
{
    LocalNode blossom;
} ConvergecastMessageBlossomNeedExpand;

typedef struct
{
    ConvergecastType type;
    union
    {
        ConvergecastMessageNonZeroGrow non_zero_grow;
        ConvergecastMessageConflict conflict;
        ConvergecastMessageTouchingVirtual touching_virtual;
        ConvergecastMessageBlossomNeedExpand blossom_need_expand;
    } data;
} ConvergecastMessage;
