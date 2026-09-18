/* Candidate fixed-block kernels; no engine integration or timing claim.
   All pointers address a complete readable/writable 16-byte block.
   Inputs use the same validated variable roster and support bitplane.
   EventKey complement is valid only with the retained owning space. */
#include <arm_neon.h>
#include <stdint.h>
#define LEAF __attribute__((noinline,used))

LEAF uint64_t event_complement(uint64_t region) { return region ^ UINT64_C(1); }

LEAF void steal_block(uint64_t *out, const uint64_t *b,
                     const uint64_t *c, const uint64_t *a) {
    uint64x2_t r = vbicq_u64(vld1q_u64(b), vorrq_u64(vld1q_u64(c),vld1q_u64(a)));
    vst1q_u64(out,r); /* B is already within support. */
}

LEAF void boolean4_block(uint64_t *out, const uint64_t *a,
                        const uint64_t *b, const uint64_t *support, unsigned op) {
    uint64x2_t av=vld1q_u64(a), bv=vld1q_u64(b);
    uint64x2_t m0=vdupq_n_u64(0-((uint64_t)(op >> 0)&1));
    uint64x2_t m1=vdupq_n_u64(0-((uint64_t)(op >> 1)&1));
    uint64x2_t m2=vdupq_n_u64(0-((uint64_t)(op >> 2)&1));
    uint64x2_t m3=vdupq_n_u64(0-((uint64_t)(op >> 3)&1));
    uint64x2_t r=vbslq_u64(av,vbslq_u64(bv,m3,m2),vbslq_u64(bv,m1,m0));
    vst1q_u64(out,vandq_u64(r,vld1q_u64(support)));
}

LEAF void predicate_lookup16(uint8_t *out,const uint8_t *signatures,const uint8_t *table) {
    vst1q_u8(out,vqtbl1q_u8(vld1q_u8(table),vld1q_u8(signatures)));
}

#ifdef PROBE_MAIN
#include <assert.h>
#include <stdio.h>
static uint64_t state=0x73d18f920ce481abULL;
static uint64_t draw(void) { state^=state<<13;state^=state>>7;state^=state<<17;return state; }
int main(void) {
    unsigned operations=0;
    for(unsigned n=0;n<2000;n++) {
        uint64_t a[2]={draw(),draw()}, b[2]={draw(),draw()}, c[2]={draw(),draw()};
        uint64_t s[2]={draw(),draw()}, out[2];
        assert(event_complement(event_complement(a[0])) == a[0]);
        steal_block(out,b,c,a);
        for(unsigned lane=0;lane<2;lane++) assert(out[lane] == (b[lane]&~(c[lane]|a[lane])));
        for(unsigned op=0;op<16;op++) {
            boolean4_block(out,a,b,s,op);
            for(unsigned lane=0;lane<2;lane++) {
                uint64_t expected=0;
                for(unsigned bit=0;bit<64;bit++) {
                    unsigned cell=(((a[lane]>>bit)&1)<<1)|((b[lane]>>bit)&1);
                    expected |= (uint64_t)(((op>>cell)&1)&((s[lane]>>bit)&1))<<bit;
                }
                assert(out[lane]==expected);
            }
            operations++;
        }
    }
    uint8_t sig[16], table[16], result[16];
    for(unsigned i=0;i<16;i++) { sig[i]=i;table[i]=(i&8)==0; }
    predicate_lookup16(result,sig,table);
    for(unsigned i=0;i<16;i++) assert(result[i] == ((i&8)==0));
    printf("32000 Boolean blocks (4096000 world bits), 2000 steal blocks, complement and TBL pass.\n");
    assert(operations==32000);
}
#endif
