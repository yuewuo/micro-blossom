#include <memory>
#include <verilated.h>
#include "verilated_vcd_c.h"
#include "Vbroadcast_tree.h"

int main(int argc, char **argv)
{
    Verilated::mkdir("logs");
    const std::unique_ptr<VerilatedContext> contextp{new VerilatedContext};
    contextp->debug(0);
    contextp->randReset(2);
    contextp->traceEverOn(true);
    contextp->commandArgs(argc, argv);
    const std::unique_ptr<VerilatedVcdC> tfp{new VerilatedVcdC};
    const std::unique_ptr<Vbroadcast_tree> top{new Vbroadcast_tree{contextp.get(), "TOP"}};

    // set initial input signals
    top->trace(tfp.get(), 99);
    tfp->open("logs/sim.vcd");

    for (int i = 0; i < 40; ++i)
    {
        top->message = i;
        contextp->timeInc(1);
        top->eval();
        tfp->dump(contextp->time());
    }

    top->final();

    tfp->close();
    Verilated::mkdir("logs");
    contextp->coveragep()->write("logs/coverage.dat");

    return 0;
}
