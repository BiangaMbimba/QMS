#include "memory.h"
#include "nvs.h"
#include "esp_log.h"

static const char *TAG = "MQS-MEMORY";

esp_err_t nvs_get_info(const char *type, const char *key, char *device_info, size_t device_info_len)
{
    nvs_handle_t nvs_handler;

    esp_err_t err = nvs_open(type, NVS_READONLY, &nvs_handler);

    if (err != ESP_OK && err != ESP_ERR_NVS_NOT_FOUND)
    {
        ESP_LOGE(TAG, "Error opening NVS: %s (0x%x)", esp_err_to_name(err), err);
        return ESP_FAIL;
    }

    if (err != ESP_ERR_NVS_NOT_FOUND)
    {
        err = nvs_get_str(nvs_handler, key, device_info, &device_info_len);

        if (err == ESP_OK)
            ESP_LOGI(TAG, "Device info got, %s -> %s", key, device_info);
    }

    nvs_close(nvs_handler);
    return err;
}

esp_err_t nvs_set_info(const char *type, const char *key, const char *device_info) {
    nvs_handle_t nvs_handler;
    esp_err_t error = nvs_open(type, NVS_READWRITE, &nvs_handler);

    if (error != ESP_OK)
    {
        ESP_LOGE(TAG, "Failed to saved %s to NVS ...", key);
        return ESP_FAIL;
    }

    nvs_set_str(nvs_handler, key, device_info);
    esp_err_t err = nvs_commit(nvs_handler);
    nvs_close(nvs_handler);

    if (err != ESP_OK)
    {
        ESP_LOGE(TAG, "Error opening NVS: %s (0x%x)", esp_err_to_name(err), err);
        return ESP_FAIL;
    }

    ESP_LOGI(TAG, "%s -> %s saved", key, device_info);

    return error;
}