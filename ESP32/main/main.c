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
#include "esp_pm.h"

static const char *TAG = "QMS-LOGS";

// Configure Automatic Light Sleep
void setup_power_management() {
    // 1. Configure Power Management Lock
    // This allows the CPU to lower its frequency or stop when idle.
    esp_pm_config_t pm_config = {
        .max_freq_mhz = 240, // Max CPU speed
        .min_freq_mhz = 80,  // Min CPU speed (lowers when idle)
        .light_sleep_enable = true // Enable automatic light sleep
    };
    ESP_ERROR_CHECK(esp_pm_configure(&pm_config));
}

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

    setup_power_management();

    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());

    wifi_setup();

    esp_wifi_set_ps(WIFI_PS_MIN_MODEM);

    while (true)
    {
        int event = button_pressed(PUSH_BUTTON);

        wifi_mode_t mode;
        esp_wifi_get_mode(&mode);

        if (event == SHORT_PRESS_BIT)
        {
            if (mode == WIFI_MODE_STA)
            {
                http_send_next_command();
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