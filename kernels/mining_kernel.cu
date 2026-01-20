#include <cstdint>

#define ROTRIGHT(a,b) (((a) >> (b)) | ((a) << (32-(b))))
#define CH(x,y,z) (((x) & (y)) ^ (~(x) & (z)))
#define MAJ(x,y,z) (((x) & (y)) ^ ((x) & (z)) ^ ((y) & (z)))
#define EP0(x) (ROTRIGHT(x,2) ^ ROTRIGHT(x,13) ^ ROTRIGHT(x,22))
#define EP1(x) (ROTRIGHT(x,6) ^ ROTRIGHT(x,11) ^ ROTRIGHT(x,25))
#define SIG0(x) (ROTRIGHT(x,7) ^ ROTRIGHT(x,18) ^ ((x) >> 3))
#define SIG1(x) (ROTRIGHT(x,17) ^ ROTRIGHT(x,19) ^ ((x) >> 10))

typedef unsigned char BYTE;
typedef unsigned int WORD;

// SHA-256 constants
__constant__ WORD K_const[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
    0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
    0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
    0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

__constant__ WORD H_const[8] = {
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
};

__device__ void sha256_transform(WORD state[8], const BYTE data[64], WORD* K_shared) {
    WORD a, b, c, d, e, f, g, h, i, j, t1, t2, m[64];

    for (i = 0, j = 0; i < 16; ++i, j += 4)
        m[i] = (data[j] << 24) | (data[j+1] << 16) | (data[j+2] << 8) | (data[j+3]);
    for (; i < 64; ++i)
        m[i] = SIG1(m[i-2]) + m[i-7] + SIG0(m[i-15]) + m[i-16];

    a = state[0]; b = state[1]; c = state[2]; d = state[3];
    e = state[4]; f = state[5]; g = state[6]; h = state[7];

    for (i = 0; i < 64; ++i) {
        t1 = h + EP1(e) + CH(e,f,g) + K_shared[i] + m[i];
        t2 = EP0(a) + MAJ(a,b,c);
        h = g; g = f; f = e; e = d + t1;
        d = c; c = b; b = a; a = t1 + t2;
    }

    state[0] += a; state[1] += b; state[2] += c; state[3] += d;
    state[4] += e; state[5] += f; state[6] += g; state[7] += h;
}

__device__ void sha256_hash(const BYTE* data, int len, BYTE hash[32], 
                             WORD* K_shared, WORD* H_shared) {
    WORD state[8];
    BYTE buffer[64];
    int i;
    
    // Copy initial hash values from shared memory
    for (i = 0; i < 8; i++) state[i] = H_shared[i];
    
    // Process 64-byte blocks
    int full_blocks = len / 64;
    for (int block = 0; block < full_blocks; block++) {
        sha256_transform(state, data + block * 64, K_shared);
    }
    
    // Handle remaining bytes + padding
    int remaining = len % 64;
    for (i = 0; i < remaining; i++) buffer[i] = data[full_blocks * 64 + i];
    buffer[remaining] = 0x80;
    
    if (remaining < 56) {
        for (i = remaining + 1; i < 56; i++) buffer[i] = 0;
    } else {
        for (i = remaining + 1; i < 64; i++) buffer[i] = 0;
        sha256_transform(state, buffer, K_shared);
        for (i = 0; i < 56; i++) buffer[i] = 0;
    }
    
    // Append length in bits
    unsigned long long bitlen = len * 8ULL;
    buffer[63] = bitlen; buffer[62] = bitlen >> 8;
    buffer[61] = bitlen >> 16; buffer[60] = bitlen >> 24;
    buffer[59] = bitlen >> 32; buffer[58] = bitlen >> 40;
    buffer[57] = bitlen >> 48; buffer[56] = bitlen >> 56;
    sha256_transform(state, buffer, K_shared);
    
    // Produce final hash (big-endian format)
    for (i = 0; i < 4; i++) {
        hash[i]    = (state[0] >> (24-i*8)) & 0xFF;
        hash[i+4]  = (state[1] >> (24-i*8)) & 0xFF;
        hash[i+8]  = (state[2] >> (24-i*8)) & 0xFF;
        hash[i+12] = (state[3] >> (24-i*8)) & 0xFF;
        hash[i+16] = (state[4] >> (24-i*8)) & 0xFF;
        hash[i+20] = (state[5] >> (24-i*8)) & 0xFF;
        hash[i+24] = (state[6] >> (24-i*8)) & 0xFF;
        hash[i+28] = (state[7] >> (24-i*8)) & 0xFF;
    }
}

__device__ int compare_hash_to_target(const BYTE* hash, const WORD* target_parts) {
    // Compare 256-bit hash to 256-bit target
    // hash: 32 bytes from SHA-256 in BIG-endian format (hash[0] is MSB)
    // target_parts: 8 x u32 words from little-endian byte array
    
    // Iterate from most significant word to least significant
    for (int i = 7; i >= 0; i--) {
        // Extract word from hash
        // Since hash is big-endian and word i corresponds to bytes at offset i*4,
        // we need: hash[i*4] is MSB of this word, hash[i*4+3] is LSB
        WORD hash_word = (hash[4*i+3] << 24) | (hash[4*i+2] << 16) | 
                         (hash[4*i+1] << 8)  | hash[4*i];
        
        WORD target_word = target_parts[i];
        
        if (hash_word < target_word) return 1;  // hash < target (valid)
        if (hash_word > target_word) return 0;  // hash > target (invalid)
        // If equal, continue to next less significant word
    }
    return 0;  // hash == target, treat as invalid (matching Python behavior)
}

extern "C" __global__
void mine_kernel(const BYTE* header_prefix, const WORD* target_parts,
                 unsigned int start_nonce, int batch_size,
                 unsigned int* result_nonce, BYTE* result_hash) {
    // Shared memory for SHA-256 constants
    __shared__ WORD K_shared[64];
    __shared__ WORD H_shared[8];
    
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= batch_size) return;

    // Load constants into shared memory (per block)
    if (threadIdx.x < 64) {
        K_shared[threadIdx.x] = K_const[threadIdx.x];
    }
    if (threadIdx.x < 8) {
        H_shared[threadIdx.x] = H_const[threadIdx.x];
    }
    __syncthreads();

    unsigned int nonce = start_nonce + idx;
    
    // Build header in local memory
    BYTE header[80];
    for (int i = 0; i < 76; i++) header[i] = header_prefix[i];
    
    // Add nonce (little-endian)
    header[76] = nonce & 0xFF;
    header[77] = (nonce >> 8) & 0xFF;
    header[78] = (nonce >> 16) & 0xFF;
    header[79] = (nonce >> 24) & 0xFF;
    
    BYTE hash1[32];
    BYTE hash2[32];
    
    // First SHA-256
    sha256_hash(header, 80, hash1, K_shared, H_shared);
    
    // Second SHA-256
    sha256_hash(hash1, 32, hash2, K_shared, H_shared);
    
    // Check if hash meets target
    if (compare_hash_to_target(hash2, target_parts)) {
        // Use atomic compare-and-swap to store the result
        unsigned int old = atomicCAS(result_nonce, 0xFFFFFFFF, nonce);
        if (old == 0xFFFFFFFF) {
            // This thread won the race, also store the hash
            for (int i = 0; i < 32; i++) {
                result_hash[i] = hash2[i];
            }
        }
    }
}