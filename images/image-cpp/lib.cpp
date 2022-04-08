#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <iostream>

#include "bindings/wasi-ce.h"


wasi_ce_error_t wasi_ce_ce_handler(wasi_ce_string_t *event, wasi_ce_string_t *ret0) {
    char * str;
    str = (char *)malloc(sizeof(char) * (24));
    std::strncpy(str, event->ptr + 27, 23);
    printf("Received event id: `%s`\n", str);
    *ret0 = *event;
    return WASI_CE_ERROR_SUCCESS;
}
