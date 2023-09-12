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
    parameter NODES = 10
) (
    input wire [MESSAGE_WIDTH-1:0] message,
 	output wire [MESSAGE_WIDTH*NODES-1:0] outputs
);

genvar i;
generate
    if (NODES < MAX_FANOUT) begin : gen_direct_copy
        for (i=0; i<NODES; i=i+1) begin
          assign outputs[(i+1)*MESSAGE_WIDTH-1:i*MESSAGE_WIDTH] = message;
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
                .outputs(outputs[(i+1)*subtree*MESSAGE_WIDTH-1: i*subtree*MESSAGE_WIDTH])
            );
        end
        if (subtree_num * subtree < NODES) begin : remaining_tree
            broadcast_tree #(
                .MESSAGE_WIDTH(MESSAGE_WIDTH),
                .MAX_FANOUT(MAX_FANOUT),
                .NODES(NODES - subtree_num * subtree)
            ) broadcast_subtree(
                .message(message),
                .outputs(outputs[MESSAGE_WIDTH*NODES-1: subtree_num*subtree*MESSAGE_WIDTH])
            );
        end
    end
endgenerate

endmodule
