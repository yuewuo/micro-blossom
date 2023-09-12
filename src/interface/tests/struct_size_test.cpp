#include <cstdio>
#include <cassert>
#include "../interface.h"

int main()
{
    printf("sizeof(BroadcastMessage) = %ld\n", sizeof(BroadcastMessage));
    assert(sizeof(BroadcastMessage) == 6);

    printf("sizeof(GlobalVertex) = %ld\n", sizeof(GlobalVertex));
    assert(sizeof(GlobalVertex) == 8);

    printf("sizeof(LocalVertex) = %ld\n", sizeof(LocalVertex));

    return 0;
}
