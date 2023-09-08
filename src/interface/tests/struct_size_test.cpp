#include <cstdio>
#include <cassert>
#include "../interface.h"

int main()
{
    printf("sizeof(BroadcastMessage) = %ld\n", sizeof(BroadcastMessage));
    assert(sizeof(BroadcastMessage) <= 4);

    printf("sizeof(VertexFullAddress) = %ld\n", sizeof(VertexFullAddress));
    assert(sizeof(VertexFullAddress) == 8);

    assert(VERTEX_T_BITS + VERTEX_I_BITS + VERTEX_J_BITS <= 8 * sizeof(VERTEX_BITS_DATA_TYPE));
    printf("sizeof(VertexAddress) = %ld\n", sizeof(VertexAddress));
    assert(sizeof(VertexAddress) == sizeof(VERTEX_BITS_DATA_TYPE));

    return 0;
}
