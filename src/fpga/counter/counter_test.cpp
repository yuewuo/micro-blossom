#include <memory>
#include <verilated.h>
#include "Vcounter.h"

#define MAX_CLOCK 100


int main(int argc, char** argv) {
    Verilated::mkdir("logs");
    const std::unique_ptr<VerilatedContext> contextp{new VerilatedContext};
    contextp->debug(0);
    contextp->randReset(2);
    contextp->traceEverOn(true);
    contextp->commandArgs(argc, argv);
    const std::unique_ptr<Vcounter> top{new Vcounter{contextp.get(), "TOP"}};

    // set initial input signals
    top->rst_n = !0;
    top->clk = 0;

    for (int clk=0; clk < MAX_CLOCK && !contextp->gotFinish(); ++clk) {
        contextp->timeInc(1);
        top->clk = !top->clk;

        if (!top->clk) {  // change the reset signal at the negative edge
            top->rst_n = !(contextp->time() > 1 && contextp->time() < 10);
        }

        top->eval();

        VL_PRINTF("[%" PRId64 "] clk=%x rstl=%x counter=%" PRIx64 "\n",
                    contextp->time(), top->clk, top->rst_n, top->count);

        // TODO: you can add any test assertions here
    }

    top->final();

    Verilated::mkdir("logs");
    contextp->coveragep()->write("logs/coverage.dat");

    return 0;
}
