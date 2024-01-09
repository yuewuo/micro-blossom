#include <stdio.h>
#include "xil_printf.h"
#include "xil_cache.h"
#include "binding.h"

int main()
{
    Xil_DCacheEnable();
    Xil_ICacheEnable();

    rust_main();
    return 0;
}
