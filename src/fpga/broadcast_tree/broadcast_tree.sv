
function integer max_full_subtree_nodes(integer nodes, max_fanouts);
begin
    max_full_subtree_nodes = 1;
    while (max_full_subtree_nodes * max_fanouts < nodes) begin
        max_full_subtree_nodes *= max_fanouts;
    end
end
endfunction

module broadcast_tree #(
    parameter MESSAGE_WIDTH = 16,
    parameter MAX_FANOUT = 3,
    parameter NODES = 100
) (
    input wire [MESSAGE_WIDTH-1:0] message,
    output wire [MESSAGE_WIDTH-1:0] outputs [0:NODES-1]
);

genvar i;
generate
    if (NODES < MAX_FANOUT) begin : gen_direct_copy
        for (i=0; i<NODES; i=i+1) begin
            assign outputs[i] = message;
        end
    end else begin : gen_subtree
        parameter subtree = max_full_subtree_nodes(NODES, MAX_FANOUT);
        parameter subtree_num = NODES / subtree;
        for (i=0; i<subtree_num; i=i+1) begin
            broadcast_tree #(
                .MESSAGE_WIDTH(MESSAGE_WIDTH),
                .MAX_FANOUT(MAX_FANOUT),
                .NODES(subtree)
            ) broadcast_subtree(
                .message(message),
                .outputs(outputs[i * subtree: (i+1) * subtree-1])
            );
        end
        for (i=subtree_num * subtree; i<NODES; i=i+1) begin
            assign outputs[i] = message;
        end
    end
endgenerate

endmodule
