#include <stdlib.h>
#include <wasi-ce.h>

__attribute__((weak, export_name("canonical_abi_realloc")))
void *canonical_abi_realloc(
void *ptr,
size_t orig_size,
size_t org_align,
size_t new_size
) {
  void *ret = realloc(ptr, new_size);
  if (!ret)
  abort();
  return ret;
}

__attribute__((weak, export_name("canonical_abi_free")))
void canonical_abi_free(
void *ptr,
size_t size,
size_t align
) {
  free(ptr);
}
#include <string.h>

void wasi_ce_string_set(wasi_ce_string_t *ret, const char *s) {
  ret->ptr = (char*) s;
  ret->len = strlen(s);
}

void wasi_ce_string_dup(wasi_ce_string_t *ret, const char *s) {
  ret->len = strlen(s);
  ret->ptr = canonical_abi_realloc(NULL, 0, 1, ret->len);
  memcpy(ret->ptr, s, ret->len);
}

void wasi_ce_string_free(wasi_ce_string_t *ret) {
  canonical_abi_free(ret->ptr, ret->len, 1);
  ret->ptr = NULL;
  ret->len = 0;
}
void wasi_ce_payload_free(wasi_ce_payload_t *ptr) {
  canonical_abi_free(ptr->ptr, ptr->len * 1, 1);
}
typedef struct {
  // 0 if `val` is `ok`, 1 otherwise
  uint8_t tag;
  union {
    wasi_ce_string_t ok;
    wasi_ce_error_t err;
  } val;
} wasi_ce_expected_string_error_t;
static int64_t RET_AREA[3];
__attribute__((export_name("ce-handler")))
int32_t __wasm_export_wasi_ce_ce_handler(int32_t arg, int32_t arg0) {
  wasi_ce_string_t arg1 = (wasi_ce_string_t) { (char*)(arg), (size_t)(arg0) };
  wasi_ce_string_t ok;
  wasi_ce_error_t ret = wasi_ce_ce_handler(&arg1, &ok);
  
  wasi_ce_expected_string_error_t ret2;
  if (ret <= 2) {
    ret2.tag = 1;
    ret2.val.err = ret;
  } else {
    ret2.tag = 0;
    ret2.val.ok = ok;
  }
  int32_t variant6;
  int32_t variant7;
  int32_t variant8;
  switch ((int32_t) (ret2).tag) {
    case 0: {
      const wasi_ce_string_t *payload = &(ret2).val.ok;
      variant6 = 0;
      variant7 = (int32_t) (*payload).ptr;
      variant8 = (int32_t) (*payload).len;
      break;
    }
    case 1: {
      const wasi_ce_error_t *payload3 = &(ret2).val.err;
      int32_t variant;
      switch ((int32_t) *payload3) {
        case 0: {
          variant = 0;
          break;
        }
        case 1: {
          variant = 1;
          break;
        }
      }
      variant6 = 1;
      variant7 = variant;
      variant8 = 0;
      break;
    }
  }
  int32_t ptr = (int32_t) &RET_AREA;
  *((int32_t*)(ptr + 16)) = variant8;
  *((int32_t*)(ptr + 8)) = variant7;
  *((int32_t*)(ptr + 0)) = variant6;
  return ptr;
}
