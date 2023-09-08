#include <cstdio>
#include <cassert>
#include "../interface.h"

int main()
{
    if (sizeof(BroadcastMessage) > 4)
    {
        printf("sizeof(BroadcastMessage) = %ld\n", sizeof(BroadcastMessage));
        throw;
    }

    return 0;
}
