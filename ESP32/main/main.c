#include <string.h>
#include <driver/gpio.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_system.h"
#include "esp_event.h"
#include "esp_log.h"
#include "nvs_flash.h"
#include "nvs.h"
#include "esp_wifi.h"
#include "wifi_set.h"
#include "pins.h"
#include "websocket.h"

static const char *TAG = "QMS-LOGS";

void app_main(void)
{

    gpio_reset_pin(GOOD_LED);
    gpio_reset_pin(CONF_LED);

    gpio_set_direction(GOOD_LED, GPIO_MODE_OUTPUT);
    gpio_set_direction(CONF_LED, GPIO_MODE_OUTPUT);

    gpio_set_level(GOOD_LED, true);

    esp_err_t nvs = nvs_flash_init();
    if (nvs == ESP_ERR_NVS_NO_FREE_PAGES || nvs == ESP_ERR_NVS_NEW_VERSION_FOUND)
    {
        ESP_ERROR_CHECK(nvs_flash_erase());
        nvs = nvs_flash_init();
    }
    ESP_ERROR_CHECK(nvs);

    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());

    wifi_setup();

    while (true)
    {
        int event = button_pressed(PUSH_BUTTON);

        wifi_mode_t mode;
        esp_wifi_get_mode(&mode);

        if (event == SHORT_PRESS_BIT)
        {
            if (mode == WIFI_MODE_STA)
            {
                websocket_send_message("{\"message\": \"NEXT\"}");
                ESP_LOGI(TAG, "Button pressed -> Send increment ");
            }
        }

        else if (event == LONG_PRESS_BIT)
        {

            if (mode == WIFI_MODE_STA)
            {
                wifi_ap_mode();
            }
            else
            {
                ESP_LOGI(TAG, "Esp restarted ...");
                esp_restart();
            }
        }

        vTaskDelay(pdMS_TO_TICKS(100));
    }
}