// max d=31 (31^3 < 32768), for 0.1% physical error rate we have 18 reported obstacles on average
// since there is no need to save memory space, we just allocate whatever convenient; for now we assume 8MB
// 1. 128KB control block at [0, 0x2_0000]
//    0: (RO) 64 bits timer counter
//    8: (RO) 32 bits version register
//    12: (RO) 32 bits context depth
//    16: (RO) 8 bits number of conflict channels (no more than 6 is supported)
//    17: (RO) 8 bits dualConfig.vertexBits
//    18: (RO) 8 bits dualConfig.weightBits
//    24: (RW) 32 bits instruction counter
//    32: (RW) 32 bits readout counter
//  - (64 bits only) the following 4KB section is designed to allow burst writes (e.g. use xsdb "mwr -bin -file" command)
//    0x1000: (WO) (32 bits instruction, 16 bits context id)
//    0x1008: (WO) (32 bits instruction, 16 bits context id)
//    0x1010: ... repeat for 512: in total 4KB space
//  - (32 bits only) the following 4KB section is designed for 32 bit bus where context id is encoded in the address
//    0x2000: 32 bits instruction for context 0
//    0x2004: 32 bits instruction for context 1
//    0x2008: ... repeat for 1024: in total 4KB space
// 2. 128KB context readouts at [0x2_0000, 0x4_0000), each context takes 128 byte space, assuming no more than 1024 contexts
//    [context 0]
//      0: (RW) 16 bits maximum growth (offloaded primal), when 0, disable offloaded primal,
//                  write to this field will automatically clear accumulated grown value
//      2: (RW) 16 bits accumulated grown value (for primal offloading)
//      4: (RO) 16 bits growable value (writing to this position has no effect)
//      8: (RW) 64 bits timestamp of receiving the last ``load obstacles'' instruction
//      16: (RW) 64 bits timestamp of receiving the last ``growable = infinity'' response
//      (at most 6 concurrent conflict report, large enough)
//      32: (RO) 128 bits conflict value [0] (96 bits conflict value, 8 bits is_valid)
//      48: (RO) 128 bits conflict value [1]
//      64: (RO) 128 bits conflict value [2]
//         ...
//    [context 1]
//      128: (RO) 32 bits growable value, when 0, the conflict values are valid
//         ...
//
