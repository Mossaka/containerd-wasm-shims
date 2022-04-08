#ifndef __BINDINGS_WASI_CE_H
#define __BINDINGS_WASI_CE_H
#ifdef __cplusplus
extern "C"
{
  #endif
  
  #include <stdint.h>
  #include <stdbool.h>
  
  typedef struct {
    char *ptr;
    size_t len;
  } wasi_ce_string_t;
  
  void wasi_ce_string_set(wasi_ce_string_t *ret, const char *s);
  void wasi_ce_string_dup(wasi_ce_string_t *ret, const char *s);
  void wasi_ce_string_free(wasi_ce_string_t *ret);
  // General purpose error.
  typedef uint8_t wasi_ce_error_t;
  #define WASI_CE_ERROR_SUCCESS 0
  #define WASI_CE_ERROR_ERROR 1
  typedef struct {
    uint8_t *ptr;
    size_t len;
  } wasi_ce_payload_t;
  void wasi_ce_payload_free(wasi_ce_payload_t *ptr);
  wasi_ce_error_t wasi_ce_ce_handler(wasi_ce_string_t *event, wasi_ce_string_t *ret0);
  #ifdef __cplusplus
}
#endif
#endif
