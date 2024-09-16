What languages could I use? Hare? As far as I can see, Hare does not support dynamic libraries, and for whatever reason I would like to be able to create a shared object. Zig? I think it is subjectively ugly and I have almost no experience in it. C? Fine but lame. Rust? I am slowly starting to think I might have been over-hyped. Go? Fine but lame.

I have not read a proper book or so on how to implement a proper B-tree, let alone a DB/KV, but I have a idea of how it works. However, what breaks my head is a dynamically sized key or a value that is larger than a page.


Node Layout (in Pseudo-C):
```c
struct node __attribute__((packed, aligned(4096))) {
  uint16_t type: 1, // 1 if inner.
           pageid: 15;
  uint16_t num_keys;
  uint16_t offsets[num_keys + 1]; // offsets within this page to the keys or KV pairs
  union {
    struct inner {
      uint32_t child_page_offsets_in_file[num_keys + 1];
      uint8_t[/*dynamic size can be calculated based on key_offsets*/] keys[num_keys];
    };
    struct leaf {
      struct {
        uint16_t key_len;
        uint16_t val_len;
        uint8_t key[key_len];
        uint8_t val[val_len];
      } keys_and_vals[num_keys];
    };
  };
};
```

Insert-Algo idea: Copy-On-Write:
__TODO: What about the case where the key is already present?__

- Walk to leaf node resp. for the key
  - If it has the space:
    1. Copy the page, insert key/val in the copy
    2. Swap pointer to leaf page in parent (or root pointer)
    3. FIXME: This should happen atomically/unfailable/also using Copy-On-Write!
    4. Add orig. leaf page to free list.
  - If not:
    1. Split node into two, insert key/val in the first (or second? does this matter?) half.
    2. Return to parent node:
      - the orig node which should be added to the free list later.
      - the new first half
      - the new second half
      - the key at which the node was splitted
    3. Replace the orig pointer by the first half and insert the second half.
      - If it has the space for the second half:
        - Either: Do proper COW like for leaf insert, or:
        - Hacky: Because size fixed, just overwrite?
      - Is not:
        - Split! See 1. for how to do that.
 
I should maybe really go check out some literature :/ But fuck it, I think this can work.

