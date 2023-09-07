`timescale 1ns/1ps

module counter (
    input clk,       // Clock input
    input rst_n,     // Active low reset
    output reg [31:0] count // 32-bit wide counter output
);

    // Parameter for maximum count value
    parameter MAX_COUNT = 32'h0010;

    always_ff @(posedge clk) begin
        if (!rst_n) begin
            $display("reset counter");
            count <= 32'd0;         // If reset is active, initialize counter to zero
        end else if (count == MAX_COUNT) begin
            $display("wrap counter");
            count <= 32'd0;         // If counter reaches the maximum value, wrap around
        end else begin
            $display("add counter");
            count <= count + 32'd1; // Otherwise, just increment the counter
        end
    end

endmodule
