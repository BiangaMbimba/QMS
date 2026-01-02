#ifndef __MEMORY_HEADER
#define __MEMORY_HEADER

#include "esp_system.h"

esp_err_t nvs_get_info(const char *type, const char *key, char *device_info, size_t device_info_len);
esp_err_t nvs_set_info(const char *type, const char *key, const char *device_info);

#endif