module counter (
    input clk,       // Clock input
    input rst_n,     // Active low reset
    output reg [31:0] count // 32-bit wide counter output
);

    // Parameter for maximum count value
    parameter MAX_COUNT = 32'd1000; // Default to 1000, adjust as necessary

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) 
            count <= 32'd0;         // If reset is active, initialize counter to zero
        else if (count == MAX_COUNT)
            count <= 32'd0;         // If counter reaches the maximum value, wrap around
        else
            count <= count + 32'd1; // Otherwise, just increment the counter
    end

endmodule
